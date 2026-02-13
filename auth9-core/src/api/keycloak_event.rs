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
///
/// Handles both user events (have `type` field) and admin events
/// (have `operationType`/`resourceType`, no `type`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakEvent {
    /// Event type for user events (e.g., "LOGIN", "LOGIN_ERROR", "LOGOUT")
    /// Optional because admin events use operationType/resourceType instead.
    #[serde(default, rename = "type", alias = "eventType")]
    pub event_type: Option<String>,
    /// Operation type for admin events (e.g., "CREATE", "UPDATE", "DELETE")
    pub operation_type: Option<String>,
    /// Resource type for admin events (e.g., "USER", "GROUP", "CLIENT")
    pub resource_type: Option<String>,
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
    /// Credential type used in authentication (e.g., "password", "otp", "totp")
    /// Keycloak 21+ sends this for TOTP failures instead of auth_method
    pub credential_type: Option<String>,
    /// Identity provider alias for social logins
    pub identity_provider: Option<String>,
    /// Redirect URI
    pub redirect_uri: Option<String>,
    /// Code ID for auth codes
    pub code_id: Option<String>,
}

/// Map Keycloak event type to Auth9 LoginEventType
fn map_event_type_with_details(
    kc_type: &str,
    error: Option<&str>,
    details: &KeycloakEventDetails,
) -> Option<LoginEventType> {
    match kc_type {
        // Successful logins
        "LOGIN" => Some(LoginEventType::Success),
        "CODE_TO_TOKEN" => None, // OAuth code exchange, not a distinct login

        // Failed logins
        "LOGIN_ERROR" => {
            match error {
                Some("invalid_user_credentials") => {
                    // Check auth_method or credential_type to distinguish TOTP failures
                    // Keycloak may send "otp" via authMethod or credentialType depending on version
                    if details.auth_method.as_deref() == Some("otp")
                        || matches!(
                            details.credential_type.as_deref(),
                            Some("otp") | Some("totp")
                        )
                    {
                        Some(LoginEventType::FailedMfa)
                    } else {
                        Some(LoginEventType::FailedPassword)
                    }
                }
                Some("user_not_found") => Some(LoginEventType::FailedPassword),
                Some("invalid_totp") | Some("invalid_otp") | Some("invalid_authenticator") => {
                    Some(LoginEventType::FailedMfa)
                }
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

/// Map Keycloak event type to Auth9 LoginEventType (backward-compatible wrapper for tests)
#[allow(dead_code)]
fn map_event_type(kc_type: &str, error: Option<&str>) -> Option<LoginEventType> {
    map_event_type_with_details(kc_type, error, &KeycloakEventDetails::default())
}

/// Derive failure reason from Keycloak error
fn derive_failure_reason_with_details(
    error: Option<&str>,
    details: &KeycloakEventDetails,
) -> Option<String> {
    error.map(|e| {
        match e {
            "invalid_user_credentials" => {
                // Check auth_method or credential_type to distinguish TOTP failures
                if details.auth_method.as_deref() == Some("otp")
                    || matches!(
                        details.credential_type.as_deref(),
                        Some("otp") | Some("totp")
                    )
                {
                    "Invalid MFA code"
                } else {
                    "Invalid password"
                }
            }
            "user_not_found" => "User not found",
            "invalid_totp" | "invalid_otp" | "invalid_authenticator" => "Invalid MFA code",
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

#[allow(dead_code)]
fn derive_failure_reason(error: Option<&str>) -> Option<String> {
    derive_failure_reason_with_details(error, &KeycloakEventDetails::default())
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
            // Log truncated raw body for debugging (max 500 chars, avoid sensitive data)
            let body_preview = String::from_utf8_lossy(&body[..body.len().min(500)]);
            error!(
                "Failed to parse Keycloak event: {}. Body preview: {}",
                err, body_preview
            );
            return Err(AppError::BadRequest(format!(
                "Invalid event payload: {}",
                err
            )));
        }
    };

    // 3. Skip admin events (operationType/resourceType) - we only track user login events
    if event.event_type.is_none() {
        debug!(
            "Skipping Keycloak admin event: operation={:?}, resource={:?}",
            event.operation_type, event.resource_type
        );
        return Ok(StatusCode::NO_CONTENT);
    }

    let event_type_str = event.event_type.as_deref().unwrap_or("");

    debug!(
        "Received Keycloak event: type={}, user_id={:?}, error={:?}, details={:?}",
        event_type_str, event.user_id, event.error, event.details
    );

    // 4. Map to our login event type (skip non-login events)
    let login_event_type =
        match map_event_type_with_details(event_type_str, event.error.as_deref(), &event.details) {
            Some(t) => t,
            None => {
                // Not a login event we track, acknowledge receipt
                return Ok(StatusCode::NO_CONTENT);
            }
        };

    // 5. Resolve Keycloak user ID to auth9 user ID
    // Keycloak sends its own internal user UUID, which differs from auth9's users.id.
    // We look up the auth9 user by keycloak_id so that login events and security alerts
    // reference the correct user.
    let user_id = if let Some(ref kc_user_id) = event.user_id {
        match state.user_service().get_by_keycloak_id(kc_user_id).await {
            Ok(user) => Some(user.id),
            Err(_) => {
                debug!(
                    "Could not resolve Keycloak user_id {} to auth9 user; storing as-is",
                    kc_user_id
                );
                // Fall back to using the Keycloak UUID directly (for users not yet synced)
                uuid::Uuid::parse_str(kc_user_id).ok().map(StringUuid::from)
            }
        }
    } else {
        None
    };

    // 6. Get email from event details
    let email = event
        .details
        .email
        .clone()
        .or_else(|| event.details.username.clone());

    // 7. Create login event input
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
        failure_reason: derive_failure_reason_with_details(event.error.as_deref(), &event.details),
    };

    // 8. Record the login event
    let event_id = state.analytics_service().record_login_event(input).await?;

    info!(
        "Recorded login event: id={}, type={}, user_id={:?}",
        event_id, event_type_str, event.user_id
    );

    // 9. Trigger security detection analysis
    // Fetch the event we just created by its ID to pass to security detection
    if let Ok(Some(login_event)) = state.analytics_service().get_event(event_id).await {
        if let Err(err) = state
            .security_detection_service()
            .analyze_login_event(&login_event)
            .await
        {
            error!("Security analysis failed for event {}: {}", event_id, err);
            // Don't fail the webhook for security analysis errors
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
        // CODE_TO_TOKEN is OAuth code exchange, not a distinct login event
        assert_eq!(map_event_type("CODE_TO_TOKEN", None), None);
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
    fn test_map_event_type_mfa_via_credential_type() {
        // Keycloak 21+ sends credential_type instead of auth_method for TOTP failures
        let details_otp = KeycloakEventDetails {
            credential_type: Some("otp".to_string()),
            ..Default::default()
        };
        assert_eq!(
            map_event_type_with_details(
                "LOGIN_ERROR",
                Some("invalid_user_credentials"),
                &details_otp
            ),
            Some(LoginEventType::FailedMfa)
        );

        let details_totp = KeycloakEventDetails {
            credential_type: Some("totp".to_string()),
            ..Default::default()
        };
        assert_eq!(
            map_event_type_with_details(
                "LOGIN_ERROR",
                Some("invalid_user_credentials"),
                &details_totp
            ),
            Some(LoginEventType::FailedMfa)
        );

        // auth_method=otp still works
        let details_auth_method = KeycloakEventDetails {
            auth_method: Some("otp".to_string()),
            ..Default::default()
        };
        assert_eq!(
            map_event_type_with_details(
                "LOGIN_ERROR",
                Some("invalid_user_credentials"),
                &details_auth_method
            ),
            Some(LoginEventType::FailedMfa)
        );
    }

    #[test]
    fn test_derive_failure_reason_mfa_via_credential_type() {
        let details_otp = KeycloakEventDetails {
            credential_type: Some("otp".to_string()),
            ..Default::default()
        };
        assert_eq!(
            derive_failure_reason_with_details(Some("invalid_user_credentials"), &details_otp),
            Some("Invalid MFA code".to_string())
        );

        // Without credential_type, defaults to password
        let details_none = KeycloakEventDetails::default();
        assert_eq!(
            derive_failure_reason_with_details(Some("invalid_user_credentials"), &details_none),
            Some("Invalid password".to_string())
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
        let wrong_signature =
            "sha256=0000000000000000000000000000000000000000000000000000000000000000";

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

        assert_eq!(event.event_type, Some("LOGIN_ERROR".to_string()));
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

        assert_eq!(event.event_type, Some("LOGIN".to_string()));
        assert_eq!(event.realm_id, None);
        assert_eq!(event.user_id, None);
        assert_eq!(event.error, None);
    }

    #[test]
    fn test_keycloak_admin_event_deserialization() {
        let json = r#"{
            "operationType": "CREATE",
            "resourceType": "USER",
            "realmId": "auth9",
            "time": 1704067200000
        }"#;

        let event: KeycloakEvent = serde_json::from_str(json).unwrap();

        assert_eq!(event.event_type, None);
        assert_eq!(event.operation_type, Some("CREATE".to_string()));
        assert_eq!(event.resource_type, Some("USER".to_string()));
    }

    // ========================================================================
    // Additional map_event_type edge cases
    // ========================================================================

    #[test]
    fn test_map_event_type_login_error_invalid_otp() {
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("invalid_otp")),
            Some(LoginEventType::FailedMfa)
        );
    }

    #[test]
    fn test_map_event_type_login_error_user_temporarily_disabled() {
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("user_temporarily_disabled")),
            Some(LoginEventType::Locked)
        );
    }

    #[test]
    fn test_map_event_type_login_error_default() {
        // Unknown error code defaults to FailedPassword
        assert_eq!(
            map_event_type("LOGIN_ERROR", Some("some_unknown_error")),
            Some(LoginEventType::FailedPassword)
        );
    }

    #[test]
    fn test_map_event_type_login_error_no_error() {
        // LOGIN_ERROR with None error defaults to FailedPassword
        assert_eq!(
            map_event_type("LOGIN_ERROR", None),
            Some(LoginEventType::FailedPassword)
        );
    }

    #[test]
    fn test_map_event_type_update_totp() {
        assert_eq!(map_event_type("UPDATE_TOTP", None), None);
    }

    #[test]
    fn test_map_event_type_remove_totp() {
        assert_eq!(map_event_type("REMOVE_TOTP", None), None);
    }

    #[test]
    fn test_map_event_type_non_login_events() {
        assert_eq!(map_event_type("LOGOUT_ERROR", None), None);
        assert_eq!(map_event_type("REGISTER_ERROR", None), None);
        assert_eq!(map_event_type("UPDATE_PASSWORD", None), None);
        assert_eq!(map_event_type("RESET_PASSWORD", None), None);
        assert_eq!(map_event_type("SEND_RESET_PASSWORD", None), None);
        assert_eq!(map_event_type("SEND_VERIFY_EMAIL", None), None);
        assert_eq!(map_event_type("VERIFY_EMAIL", None), None);
        assert_eq!(map_event_type("VERIFY_EMAIL_ERROR", None), None);
        assert_eq!(map_event_type("TOKEN_EXCHANGE", None), None);
        assert_eq!(map_event_type("REFRESH_TOKEN_ERROR", None), None);
        assert_eq!(map_event_type("CLIENT_LOGIN", None), None);
        assert_eq!(map_event_type("CLIENT_LOGIN_ERROR", None), None);
    }

    #[test]
    fn test_map_event_type_unknown() {
        assert_eq!(map_event_type("COMPLETELY_UNKNOWN_EVENT", None), None);
    }

    // ========================================================================
    // Additional derive_failure_reason edge cases
    // ========================================================================

    #[test]
    fn test_derive_failure_reason_all_known_errors() {
        assert_eq!(
            derive_failure_reason(Some("user_disabled")),
            Some("Account disabled".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("user_temporarily_disabled")),
            Some("Account temporarily locked".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("expired_code")),
            Some("Authentication code expired".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("invalid_code")),
            Some("Invalid authentication code".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("session_expired")),
            Some("Session expired".to_string())
        );
        assert_eq!(
            derive_failure_reason(Some("invalid_otp")),
            Some("Invalid MFA code".to_string())
        );
    }

    // ========================================================================
    // Additional deserialization edge cases
    // ========================================================================

    #[test]
    fn test_keycloak_event_with_event_type_alias() {
        // Test eventType alias
        let json = r#"{
            "eventType": "LOGIN",
            "time": 1704067200000
        }"#;

        let event: KeycloakEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, Some("LOGIN".to_string()));
    }

    #[test]
    fn test_keycloak_event_details_all_fields() {
        let json = r#"{
            "type": "LOGIN",
            "time": 0,
            "details": {
                "username": "user1",
                "email": "user1@test.com",
                "authMethod": "password",
                "credentialType": "password",
                "identityProvider": "google",
                "redirectUri": "https://app.com/cb",
                "codeId": "code-123"
            }
        }"#;

        let event: KeycloakEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.details.username, Some("user1".to_string()));
        assert_eq!(event.details.email, Some("user1@test.com".to_string()));
        assert_eq!(event.details.auth_method, Some("password".to_string()));
        assert_eq!(
            event.details.credential_type,
            Some("password".to_string())
        );
        assert_eq!(event.details.identity_provider, Some("google".to_string()));
        assert_eq!(
            event.details.redirect_uri,
            Some("https://app.com/cb".to_string())
        );
        assert_eq!(event.details.code_id, Some("code-123".to_string()));
    }

    #[test]
    fn test_keycloak_event_empty_details() {
        let json = r#"{"type": "LOGIN", "time": 0, "details": {}}"#;
        let event: KeycloakEvent = serde_json::from_str(json).unwrap();
        assert!(event.details.username.is_none());
        assert!(event.details.email.is_none());
    }
}
