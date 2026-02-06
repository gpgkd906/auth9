//! Keycloak Event Webhook Handler
//!
//! Receives login events from Keycloak via the p2-inc/keycloak-events SPI plugin.
//! This enables real-time security monitoring and analytics for authentication events.

use crate::domain::{CreateLoginEventInput, LoginEventType, StringUuid};
use crate::error::AppError;
use crate::state::{HasAnalytics, HasSecurityAlerts, HasServices};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use tracing::{debug, error, info, warn};

type HmacSha256 = Hmac<Sha256>;

/// Keycloak event payload from p2-inc/keycloak-events plugin
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakEvent {
    /// Event type (e.g., "LOGIN", "LOGIN_ERROR", "LOGOUT")
    #[serde(rename = "type")]
    pub event_type: String,
    /// Realm ID
    pub realm_id: Option<String>,
    /// Client ID (application that initiated the login)
    pub client_id: Option<String>,
    /// User ID in Keycloak
    pub user_id: Option<String>,
    /// Session ID
    pub session_id: Option<String>,
    /// IP address of the client
    pub ip_address: Option<String>,
    /// Error code if this is an error event
    pub error: Option<String>,
    /// Event timestamp (epoch millis)
    #[serde(default)]
    pub time: i64,
    /// Additional event details
    #[serde(default)]
    pub details: KeycloakEventDetails,
}

/// Additional details from Keycloak events
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakEventDetails {
    /// Username attempting to login
    pub username: Option<String>,
    /// Email of the user
    pub email: Option<String>,
    /// Authentication method (e.g., "password", "otp")
    pub auth_method: Option<String>,
    /// Identity provider alias for social logins
    pub identity_provider: Option<String>,
    /// Redirect URI
    pub redirect_uri: Option<String>,
    /// Code ID for auth codes
    pub code_id: Option<String>,
}

/// Map Keycloak event type to Auth9 LoginEventType
fn map_event_type(kc_type: &str, error: Option<&str>) -> Option<LoginEventType> {
    match kc_type {
        // Successful logins
        "LOGIN" => Some(LoginEventType::Success),
        "CODE_TO_TOKEN" => Some(LoginEventType::Success),

        // Failed logins
        "LOGIN_ERROR" => {
            match error {
                Some("invalid_user_credentials") | Some("user_not_found") => {
                    Some(LoginEventType::FailedPassword)
                }
                Some("invalid_totp") | Some("invalid_otp") => Some(LoginEventType::FailedMfa),
                Some("user_disabled") | Some("user_temporarily_disabled") => {
                    Some(LoginEventType::Locked)
                }
                _ => Some(LoginEventType::FailedPassword), // Default to password failure
            }
        }

        // MFA events
        "LOGIN_WITH_OTP" => Some(LoginEventType::Success),
        "LOGIN_WITH_OTP_ERROR" => Some(LoginEventType::FailedMfa),
        "UPDATE_TOTP" => None, // Configuration event, not a login
        "REMOVE_TOTP" => None,

        // Social login events
        "IDENTITY_PROVIDER_LOGIN" => Some(LoginEventType::Social),
        "IDENTITY_PROVIDER_LOGIN_ERROR" => Some(LoginEventType::FailedPassword),

        // Account lockout
        "USER_DISABLED_BY_PERMANENT_LOCKOUT" | "USER_DISABLED_BY_TEMPORARY_LOCKOUT" => {
            Some(LoginEventType::Locked)
        }

        // Events we don't track as login events
        "LOGOUT" | "LOGOUT_ERROR" => None,
        "REGISTER" | "REGISTER_ERROR" => None,
        "UPDATE_PASSWORD" | "RESET_PASSWORD" => None,
        "SEND_RESET_PASSWORD" | "SEND_VERIFY_EMAIL" => None,
        "VERIFY_EMAIL" | "VERIFY_EMAIL_ERROR" => None,
        "TOKEN_EXCHANGE" => None,
        "REFRESH_TOKEN" | "REFRESH_TOKEN_ERROR" => None,
        "CLIENT_LOGIN" | "CLIENT_LOGIN_ERROR" => None, // Service account logins

        _ => {
            debug!("Ignoring unknown Keycloak event type: {}", kc_type);
            None
        }
    }
}

/// Derive failure reason from Keycloak error
fn derive_failure_reason(error: Option<&str>) -> Option<String> {
    error.map(|e| {
        match e {
            "invalid_user_credentials" => "Invalid password",
            "user_not_found" => "User not found",
            "invalid_totp" | "invalid_otp" => "Invalid MFA code",
            "user_disabled" => "Account disabled",
            "user_temporarily_disabled" => "Account temporarily locked",
            "expired_code" => "Authentication code expired",
            "invalid_code" => "Invalid authentication code",
            "session_expired" => "Session expired",
            _ => e,
        }
        .to_string()
    })
}

/// Verify HMAC-SHA256 signature from Keycloak webhook
fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    // Signature format: "sha256=<hex>"
    let expected_hex = signature.strip_prefix("sha256=").unwrap_or(signature);

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };

    mac.update(body);
    let result = mac.finalize();
    let computed_hex = hex::encode(result.into_bytes());

    // Use constant-time comparison to prevent timing attacks
    constant_time_eq(computed_hex.as_bytes(), expected_hex.as_bytes())
}

/// Constant-time byte comparison
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Receive Keycloak events webhook
///
/// POST /api/v1/keycloak/events
///
/// This endpoint receives events from the Keycloak p2-inc/keycloak-events SPI plugin.
/// It validates the HMAC signature (if configured), maps the event to our domain model,
/// records it in the analytics system, and triggers security detection analysis.
pub async fn receive<S: HasServices + HasAnalytics + HasSecurityAlerts>(
    State(state): State<S>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    // 1. Verify webhook signature if secret is configured
    let config = state.config();
    if let Some(ref secret) = config.keycloak.webhook_secret {
        // p2-inc/keycloak-events ext-event-http uses X-Keycloak-Signature header
        let signature = headers
            .get("x-keycloak-signature")
            .or_else(|| headers.get("x-webhook-signature"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if signature.is_empty() {
            warn!("Keycloak webhook received without signature header");
            return Err(AppError::Unauthorized(
                "Missing webhook signature".to_string(),
            ));
        }

        if !verify_signature(secret, &body, signature) {
            warn!("Keycloak webhook signature verification failed");
            return Err(AppError::Unauthorized(
                "Invalid webhook signature".to_string(),
            ));
        }
    } else {
        debug!("Keycloak webhook signature verification skipped (no secret configured)");
    }

    // 2. Parse the event payload
    let event: KeycloakEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(err) => {
            error!("Failed to parse Keycloak event: {}", err);
            return Err(AppError::BadRequest(format!(
                "Invalid event payload: {}",
                err
            )));
        }
    };

    debug!(
        "Received Keycloak event: type={}, user_id={:?}, error={:?}",
        event.event_type, event.user_id, event.error
    );

    // 3. Map to our login event type (skip non-login events)
    let login_event_type = match map_event_type(&event.event_type, event.error.as_deref()) {
        Some(t) => t,
        None => {
            // Not a login event we track, acknowledge receipt
            return Ok(StatusCode::NO_CONTENT);
        }
    };

    // 4. Parse user ID if present (Keycloak uses UUID format)
    let user_id = event
        .user_id
        .as_ref()
        .and_then(|id| uuid::Uuid::parse_str(id).ok())
        .map(StringUuid::from);

    // 5. Get email from event details
    let email = event
        .details
        .email
        .clone()
        .or_else(|| event.details.username.clone());

    // 6. Create login event input
    let input = CreateLoginEventInput {
        user_id,
        email,
        tenant_id: None, // Keycloak events are realm-level, not tenant-specific
        event_type: login_event_type,
        ip_address: event.ip_address.clone(),
        user_agent: headers
            .get("x-forwarded-user-agent")
            .or_else(|| headers.get("user-agent"))
            .and_then(|v| v.to_str().ok())
            .map(String::from),
        device_type: None, // Could be derived from user-agent
        location: None,    // Could be derived from IP via geoip
        session_id: event
            .session_id
            .as_ref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .map(StringUuid::from),
        failure_reason: derive_failure_reason(event.error.as_deref()),
    };

    // 7. Record the login event
    let event_id = state.analytics_service().record_login_event(input).await?;

    info!(
        "Recorded login event: id={}, type={}, user_id={:?}",
        event_id,
        event.event_type,
        event.user_id
    );

    // 8. Trigger security detection analysis
    // Fetch the event we just created to pass to security detection
    if let Ok((events, _)) = state.analytics_service().list_events(1, 1).await {
        if let Some(login_event) = events.into_iter().next() {
            if let Err(err) = state
                .security_detection_service()
                .analyze_login_event(&login_event)
                .await
            {
                error!("Security analysis failed for event {}: {}", event_id, err);
                // Don't fail the webhook for security analysis errors
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_event_type_login_success() {
        assert_eq!(map_event_type("LOGIN", None), Some(LoginEventType::Success));
        assert_eq!(
            map_event_type("CODE_TO_TOKEN", None),
            Some(LoginEventType::Success)
        );
    }

    #[test]
    fn test_map_event_type_login_error() {
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("invalid_user_credentials")),
            Some(LoginEventType::FailedPassword)
        );
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("user_not_found")),
            Some(LoginEventType::FailedPassword)
        );
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("invalid_totp")),
            Some(LoginEventType::FailedMfa)
        );
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("user_disabled")),
            Some(LoginEventType::Locked)
        );
    }

    #[test]
    fn test_map_event_type_mfa() {
        assert_eq!(
            map_event_type("LOGIN_WITH_OTP", None),
            Some(LoginEventType::Success)
        );
        assert_eq!(
            map_event_type("LOGIN_WITH_OTP_ERROR", None),
            Some(LoginEventType::FailedMfa)
        );
    }

    #[test]
    fn test_map_event_type_social() {
        assert_eq!(
            map_event_type("IDENTITY_PROVIDER_LOGIN", None),
            Some(LoginEventType::Social)
        );
        assert_eq!(
            map_event_type("IDENTITY_PROVIDER_LOGIN_ERROR", None),
            Some(LoginEventType::FailedPassword)
        );
    }

    #[test]
    fn test_map_event_type_lockout() {
        assert_eq!(
            map_event_type("USER_DISABLED_BY_PERMANENT_LOCKOUT", None),
            Some(LoginEventType::Locked)
        );
        assert_eq!(
            map_event_type("USER_DISABLED_BY_TEMPORARY_LOCKOUT", None),
            Some(LoginEventType::Locked)
        );
    }

    #[test]
    fn test_map_event_type_non_login() {
        assert_eq!(map_event_type("LOGOUT", None), None);
        assert_eq!(map_event_type("REGISTER", None), None);
        assert_eq!(map_event_type("REFRESH_TOKEN", None), None);
    }

    #[test]
    fn test_derive_failure_reason() {
        assert_eq!(
            derive_failure_reason(Some("invalid_user_credentials")),
            Some("Invalid password".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("user_not_found")),
            Some("User not found".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("invalid_totp")),
            Some("Invalid MFA code".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("unknown_error")),
            Some("unknown_error".to_string())
        );
        assert_eq!(derive_failure_reason(None), None);
    }

    #[test]
    fn test_verify_signature_valid() {
        let secret = "test-secret";
        let body = b"test body";

        // Compute expected signature
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        let signature = format!("sha256={}", expected);

        assert!(verify_signature(secret, body, &signature));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let secret = "test-secret";
        let body = b"test body";
        let wrong_signature = "sha256=0000000000000000000000000000000000000000000000000000000000000000";

        assert!(!verify_signature(secret, body, wrong_signature));
    }

    #[test]
    fn test_verify_signature_without_prefix() {
        let secret = "test-secret";
        let body = b"test body";

        // Compute expected signature without prefix
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(secret, body, &signature));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"", b"a"));
    }

    #[test]
    fn test_keycloak_event_deserialization() {
        let json = r#"{
            "type": "LOGIN_ERROR",
            "realmId": "auth9",
            "clientId": "auth9-portal",
            "userId": "550e8400-e29b-41d4-a716-446655440000",
            "sessionId": "660e8400-e29b-41d4-a716-446655440001",
            "ipAddress": "192.168.1.100",
            "error": "invalid_user_credentials",
            "time": 1704067200000,
            "details": {
                "username": "john.doe",
                "email": "john@example.com",
                "authMethod": "password"
            }
        }"#;

        let event: KeycloakEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.event_type, "LOGIN_ERROR");
        assert_eq!(event.realm_id, Some("auth9".to_string()));
        assert_eq!(event.client_id, Some("auth9-portal".to_string()));
        assert_eq!(
            event.user_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
        assert_eq!(event.ip_address, Some("192.168.1.100".to_string()));
        assert_eq!(event.error, Some("invalid_user_credentials".to_string()));
        assert_eq!(event.details.username, Some("john.doe".to_string()));
        assert_eq!(event.details.email, Some("john@example.com".to_string()));
    }

    #[test]
    fn test_keycloak_event_minimal_deserialization() {
        let json = r#"{
            "type": "LOGIN",
            "time": 1704067200000
        }"#;

        let event: KeycloakEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.event_type, "LOGIN");
        assert_eq!(event.realm_id, None);
        assert_eq!(event.user_id, None);
        assert_eq!(event.error, None);
    }
}
