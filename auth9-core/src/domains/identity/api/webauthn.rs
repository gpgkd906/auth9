//! WebAuthn/Passkey API handlers

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::WebAuthnCredential;
use crate::error::AppError;
use crate::state::{HasServices, HasSessionManagement, HasWebAuthn};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

// ==================== Registration (requires auth) ====================

/// Start passkey registration
///
/// POST /api/v1/users/me/passkeys/register/start
pub async fn start_registration<S: HasWebAuthn + HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, AppError> {
    let claims = extract_identity_claims(&state, &headers)?;

    let user_id = crate::domain::StringUuid::parse_str(&claims.sub)
        .map_err(|_| AppError::BadRequest("Invalid user_id in token".to_string()))?;

    let user = state.user_service().get(user_id).await?;

    let ccr = state
        .webauthn_service()
        .start_registration(&claims.sub, &user.email, user.display_name.as_deref())
        .await?;

    // Return as raw JSON (webauthn-rs types serialize to the correct WebAuthn format)
    let mut json = serde_json::to_value(&ccr)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize challenge: {}", e)))?;

    // Override residentKey to "required" for discoverable credential support.
    // webauthn-rs 0.5 start_passkey_registration sets residentKey: "discouraged",
    // but Passkey login (discoverable authentication) requires resident keys.
    // finish_passkey_registration ignores this field during verification, so this is safe.
    if let Some(auth_sel) = json.pointer_mut("/publicKey/authenticatorSelection") {
        auth_sel["residentKey"] = serde_json::json!("required");
        auth_sel["requireResidentKey"] = serde_json::json!(true);
    }

    Ok(Json(json))
}

/// Complete passkey registration
///
/// POST /api/v1/users/me/passkeys/register/complete
pub async fn complete_registration<S: HasWebAuthn>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(body): Json<CompleteRegistrationRequest>,
) -> Result<Json<SuccessResponse<WebAuthnCredential>>, AppError> {
    let claims = extract_identity_claims(&state, &headers)?;

    let credential = state
        .webauthn_service()
        .complete_registration(&claims.sub, &body.credential, body.label)
        .await?;

    Ok(Json(SuccessResponse::new(credential)))
}

/// Request body for completing registration
#[derive(Debug, Deserialize)]
pub struct CompleteRegistrationRequest {
    pub credential: webauthn_rs_proto::RegisterPublicKeyCredential,
    pub label: Option<String>,
}

// ==================== Authentication (public, no auth) ====================

/// Start passkey authentication
///
/// POST /api/v1/auth/webauthn/authenticate/start
pub async fn start_authentication<S: HasWebAuthn>(
    State(state): State<S>,
) -> Result<Json<AuthenticationStartResponse>, AppError> {
    let result = state.webauthn_service().start_authentication().await?;

    let public_key = serde_json::to_value(&result.options)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize challenge: {}", e)))?;

    Ok(Json(AuthenticationStartResponse {
        challenge_id: result.challenge_id,
        public_key,
    }))
}

/// Response for authentication start
#[derive(Debug, Serialize)]
pub struct AuthenticationStartResponse {
    pub challenge_id: String,
    pub public_key: serde_json::Value,
}

/// Complete passkey authentication
///
/// POST /api/v1/auth/webauthn/authenticate/complete
pub async fn complete_authentication<S: HasWebAuthn + HasServices + HasSessionManagement>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(body): Json<CompleteAuthenticationRequest>,
) -> Result<Json<AuthenticationTokenResponse>, AppError> {
    let auth_result = state
        .webauthn_service()
        .complete_authentication(&body.challenge_id, &body.credential)
        .await?;

    // Look up the user
    let user_id = crate::domain::StringUuid::parse_str(&auth_result.user_id)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid user_id in stored credential")))?;

    let user = state.user_service().get(user_id).await?;

    // Create session
    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let session = state
        .session_service()
        .create_session(user.id, None, ip_address, user_agent)
        .await?;

    // Issue identity token
    let jwt_manager = HasServices::jwt_manager(&state);
    let identity_token = jwt_manager.create_identity_token_with_session(
        *user.id,
        &user.email,
        user.display_name.as_deref(),
        Some(*session.id),
    )?;

    Ok(Json(AuthenticationTokenResponse {
        access_token: identity_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
    }))
}

/// Request body for completing authentication
#[derive(Debug, Deserialize)]
pub struct CompleteAuthenticationRequest {
    pub challenge_id: String,
    pub credential: webauthn_rs_proto::PublicKeyCredential,
}

/// Token response for passkey authentication
#[derive(Debug, Serialize)]
pub struct AuthenticationTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// ==================== Management (requires auth) ====================

/// List user's WebAuthn credentials (passkeys)
pub async fn list_passkeys<S: HasWebAuthn + HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Result<Json<SuccessResponse<Vec<WebAuthnCredential>>>, AppError> {
    let claims = extract_identity_claims(&state, &headers)?;

    // Get keycloak_user_id for migration period
    let keycloak_user_id = get_keycloak_user_id(&state, &claims.sub).await.ok();

    let credentials = state
        .webauthn_service()
        .list_credentials(&claims.sub, keycloak_user_id.as_deref())
        .await?;

    Ok(Json(SuccessResponse::new(credentials)))
}

/// Delete a WebAuthn credential
pub async fn delete_passkey<S: HasWebAuthn + HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(credential_id): Path<String>,
) -> Result<Json<MessageResponse>, AppError> {
    let claims = extract_identity_claims(&state, &headers)?;

    let keycloak_user_id = get_keycloak_user_id(&state, &claims.sub).await.ok();

    state
        .webauthn_service()
        .delete_credential(&claims.sub, &credential_id, keycloak_user_id.as_deref())
        .await?;

    Ok(Json(MessageResponse::new("Passkey deleted successfully.")))
}

// ==================== Helpers ====================

/// Extract identity claims from JWT token
fn extract_identity_claims<S: HasWebAuthn>(
    state: &S,
    headers: &HeaderMap,
) -> Result<crate::jwt::IdentityClaims, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".to_string()))?;

    state
        .jwt_manager()
        .verify_identity_token(token)
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))
}

/// Get keycloak_user_id from the user record (for migration period)
async fn get_keycloak_user_id<S: HasServices>(
    state: &S,
    user_id: &str,
) -> Result<String, AppError> {
    let uuid = crate::domain::StringUuid::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid user_id".to_string()))?;
    let user = state.user_service().get(uuid).await?;
    Ok(user.keycloak_id)
}

/// Extract client IP from request headers
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(xff) = headers.get("x-forwarded-for") {
        if let Ok(xff_str) = xff.to_str() {
            if let Some(ip) = xff_str.split(',').next() {
                return Some(ip.trim().to_string());
            }
        }
    }
    if let Some(xri) = headers.get("x-real-ip") {
        if let Ok(ip) = xri.to_str() {
            return Some(ip.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_start_response_serialization() {
        let response = AuthenticationStartResponse {
            challenge_id: "test-challenge-id".to_string(),
            public_key: serde_json::json!({"challenge": "base64data"}),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-challenge-id"));
        assert!(json.contains("challenge"));
    }

    #[test]
    fn test_authentication_token_response_serialization() {
        let response = AuthenticationTokenResponse {
            access_token: "jwt-token-here".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("jwt-token-here"));
        assert!(json.contains("Bearer"));
        assert!(json.contains("3600"));
    }

    #[test]
    fn test_extract_client_ip_xff() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.1, 10.0.0.1".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "10.0.0.5".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("10.0.0.5".to_string()));
    }

    #[test]
    fn test_extract_client_ip_none() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip(&headers);
        assert!(ip.is_none());
    }

    #[test]
    fn test_extract_client_ip_xff_with_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "  192.168.1.1 , 10.0.0.1 ".parse().unwrap(),
        );
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_xff_single() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_extract_client_ip_prefers_xff_over_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.1.1.1".parse().unwrap());
        headers.insert("x-real-ip", "2.2.2.2".parse().unwrap());
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("1.1.1.1".to_string()));
    }

    #[test]
    fn test_authentication_start_response_fields() {
        let response = AuthenticationStartResponse {
            challenge_id: "ch-123".to_string(),
            public_key: serde_json::json!({"rpId": "localhost"}),
        };
        assert_eq!(response.challenge_id, "ch-123");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("rpId"));
    }

    #[test]
    fn test_authentication_token_response_fields() {
        let response = AuthenticationTokenResponse {
            access_token: "tok-123".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 7200,
        };
        assert_eq!(response.access_token, "tok-123");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 7200);
    }
}
