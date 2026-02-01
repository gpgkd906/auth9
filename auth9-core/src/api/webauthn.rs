//! WebAuthn/Passkey API handlers

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::WebAuthnCredential;
use crate::error::AppError;
use crate::state::HasWebAuthn;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

/// List user's WebAuthn credentials (passkeys)
pub async fn list_passkeys<S: HasWebAuthn>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Result<Json<SuccessResponse<Vec<WebAuthnCredential>>>, AppError> {
    let keycloak_user_id = extract_keycloak_user_id(&state, &headers)?;

    let credentials = state
        .webauthn_service()
        .list_credentials(&keycloak_user_id)
        .await?;

    Ok(Json(SuccessResponse::new(credentials)))
}

/// Delete a WebAuthn credential
pub async fn delete_passkey<S: HasWebAuthn>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(credential_id): Path<String>,
) -> Result<Json<MessageResponse>, AppError> {
    let keycloak_user_id = extract_keycloak_user_id(&state, &headers)?;

    state
        .webauthn_service()
        .delete_credential(&keycloak_user_id, &credential_id)
        .await?;

    Ok(Json(MessageResponse::new("Passkey deleted successfully.")))
}

/// Get the URL to register a new passkey
///
/// Returns a redirect URL to Keycloak's WebAuthn registration flow
pub async fn get_register_url<S: HasWebAuthn>(
    State(state): State<S>,
    axum::extract::Query(params): axum::extract::Query<RegisterUrlParams>,
) -> Result<Json<SuccessResponse<RegisterUrlResponse>>, AppError> {
    let redirect_uri = params
        .redirect_uri
        .unwrap_or_else(|| "/dashboard/settings/passkeys".to_string());

    let url = state.webauthn_service().build_register_url(&redirect_uri);

    Ok(Json(SuccessResponse::new(RegisterUrlResponse { url })))
}

/// Query parameters for register URL
#[derive(Debug, Deserialize)]
pub struct RegisterUrlParams {
    pub redirect_uri: Option<String>,
}

/// Response containing the registration URL
#[derive(Debug, Serialize)]
pub struct RegisterUrlResponse {
    pub url: String,
}

/// Extract Keycloak user ID from JWT token
fn extract_keycloak_user_id<S: HasWebAuthn>(
    state: &S,
    headers: &HeaderMap,
) -> Result<String, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".to_string()))?;

    // The identity token contains the Keycloak user ID in the subject
    if let Ok(claims) = state.jwt_manager().verify_identity_token(token) {
        // The keycloak_id should be stored in our user record
        // For now, we use the sub claim which is the Auth9 user ID
        // In production, you'd look up the user to get their keycloak_id
        return Ok(claims.sub);
    }

    Err(AppError::Unauthorized(
        "Invalid or expired token".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_url_response_serialization() {
        let response = RegisterUrlResponse {
            url: "https://keycloak.example.com/auth".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("keycloak.example.com"));
    }
}
