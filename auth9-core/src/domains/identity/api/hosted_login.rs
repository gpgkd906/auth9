//! Hosted Login API handlers
//!
//! Unified authentication endpoints under `/api/v1/hosted-login/` that route
//! through the `IdentityEngine` trait for backend switching (`keycloak` / `auth9_oidc`).
//! These endpoints are designed for direct form submission from Auth9-hosted pages,
//! returning JSON responses instead of OIDC redirects.

use crate::cache::CacheOperations;
use crate::error::{AppError, Result};
use crate::http_support::{write_audit_log_generic, MessageResponse};
use crate::models::password::{ForgotPasswordInput, ResetPasswordInput};
use crate::state::{HasCache, HasPasswordManagement, HasServices, HasSessionManagement};
use axum::{extract::State, http::HeaderMap, Json};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ==================== Types ====================

#[derive(Debug, Deserialize, ToSchema)]
pub struct HostedLoginPasswordRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct HostedLoginTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct HostedLoginLogoutRequest {
    pub post_logout_redirect_uri: Option<String>,
    pub client_id: Option<String>,
}

// ==================== Helpers ====================

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        })
}

// ==================== Handlers ====================

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/password",
    tag = "Identity",
    request_body = HostedLoginPasswordRequest,
    responses(
        (status = 200, description = "Authentication token", body = HostedLoginTokenResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Invalid credentials"),
    )
)]
/// Authenticate with email and password, returning an identity token directly.
///
/// POST /api/v1/hosted-login/password
pub async fn password_login<S: HasServices + HasSessionManagement + HasCache>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<HostedLoginPasswordRequest>,
) -> Result<Json<HostedLoginTokenResponse>> {
    let start = std::time::Instant::now();
    let email = input.email.trim().to_lowercase();
    let password = input.password.clone();

    if email.is_empty() || !email.contains('@') {
        metrics::counter!("auth9_auth_login_total", "result" => "failure", "backend" => "hosted").increment(1);
        return Err(AppError::BadRequest("Invalid email address.".to_string()));
    }
    if password.is_empty() {
        metrics::counter!("auth9_auth_login_total", "result" => "failure", "backend" => "hosted").increment(1);
        return Err(AppError::BadRequest("Password is required.".to_string()));
    }

    // Look up user — return generic error to prevent email enumeration
    let user = match state.user_service().get_by_email(&email).await {
        Ok(user) => user,
        Err(_) => {
            metrics::counter!("auth9_auth_login_total", "result" => "failure", "backend" => "hosted").increment(1);
            return Err(AppError::Unauthorized(
                "Invalid email or password.".to_string(),
            ));
        }
    };

    // Validate password through IdentityEngine (backend-agnostic)
    let valid = state
        .identity_engine()
        .user_store()
        .validate_user_password(&user.identity_subject, &password)
        .await
        .map_err(|_| {
            metrics::counter!("auth9_auth_login_total", "result" => "failure", "backend" => "hosted").increment(1);
            AppError::Unauthorized("Invalid email or password.".to_string())
        })?;

    if !valid {
        metrics::counter!("auth9_auth_login_total", "result" => "failure", "backend" => "hosted").increment(1);
        return Err(AppError::Unauthorized(
            "Invalid email or password.".to_string(),
        ));
    }

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

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "hosted_login.password",
        "user",
        Some(*user.id),
        None,
        None,
    )
    .await;

    metrics::counter!("auth9_auth_login_total", "result" => "success", "backend" => "hosted").increment(1);
    metrics::histogram!("auth9_hosted_login_duration_seconds", "method" => "password").record(start.elapsed().as_secs_f64());

    Ok(Json(HostedLoginTokenResponse {
        access_token: identity_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/logout",
    tag = "Identity",
    responses(
        (status = 200, description = "Logged out successfully", body = MessageResponse),
    )
)]
/// Revoke session and log out, returning JSON (no redirect).
///
/// POST /api/v1/hosted-login/logout
pub async fn hosted_logout<S: HasServices + HasSessionManagement + HasCache>(
    State(state): State<S>,
    auth: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<Json<MessageResponse>> {
    if let Some(TypedHeader(Authorization(bearer))) = auth {
        match HasServices::jwt_manager(&state).verify_identity_token(bearer.token()) {
            Ok(claims) => {
                if let Some(ref sid) = claims.sid {
                    // Revoke session in database
                    if let Ok(session_id) = uuid::Uuid::parse_str(sid) {
                        if let Ok(user_id) = uuid::Uuid::parse_str(&claims.sub) {
                            let _ = state
                                .session_service()
                                .revoke_session(session_id.into(), user_id.into())
                                .await;
                        }
                    }

                    // Blacklist token for immediate revocation
                    let now = Utc::now().timestamp();
                    let remaining_ttl = if claims.exp > now {
                        (claims.exp - now) as u64
                    } else {
                        0
                    };

                    if remaining_ttl > 0 {
                        let _ = state
                            .cache()
                            .add_to_token_blacklist(sid, remaining_ttl)
                            .await;
                    }

                    // Clean up refresh token sessions
                    let _ = state
                        .cache()
                        .remove_all_refresh_sessions_for_session(sid)
                        .await;

                    // Revoke in identity engine (backend-agnostic)
                    let _ = state
                        .identity_engine()
                        .session_store()
                        .delete_user_session(sid)
                        .await;

                    tracing::info!(
                        user_id = %claims.sub,
                        session_id = %sid,
                        "Hosted login: session revoked on logout"
                    );
                }

                let _ = write_audit_log_generic(
                    &state,
                    &headers,
                    "hosted_login.logout",
                    "user",
                    uuid::Uuid::parse_str(&claims.sub).ok(),
                    None,
                    None,
                )
                .await;
            }
            Err(e) => {
                tracing::debug!(error = %e, "Hosted login logout with invalid/expired token");
            }
        }
    }

    Ok(Json(MessageResponse::new("Logged out successfully.")))
}

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/start-password-reset",
    tag = "Identity",
    request_body = ForgotPasswordInput,
    responses(
        (status = 200, description = "Password reset email sent", body = MessageResponse),
    )
)]
/// Request a password reset email via hosted login flow.
///
/// POST /api/v1/hosted-login/start-password-reset
pub async fn start_password_reset<S: HasPasswordManagement>(
    State(state): State<S>,
    Json(input): Json<ForgotPasswordInput>,
) -> Result<Json<MessageResponse>> {
    state.password_service().request_reset(input).await?;

    Ok(Json(MessageResponse::new(
        "If an account exists with this email, a password reset link has been sent.",
    )))
}

#[utoipa::path(
    post,
    path = "/api/v1/hosted-login/complete-password-reset",
    tag = "Identity",
    request_body = ResetPasswordInput,
    responses(
        (status = 200, description = "Password reset successfully", body = MessageResponse),
    )
)]
/// Complete password reset using a token via hosted login flow.
///
/// POST /api/v1/hosted-login/complete-password-reset
pub async fn complete_password_reset<S: HasPasswordManagement + HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<ResetPasswordInput>,
) -> Result<Json<MessageResponse>> {
    state.password_service().reset_password(input).await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "hosted_login.password_reset",
        "user",
        None,
        None,
        None,
    )
    .await;

    Ok(Json(MessageResponse::new(
        "Password has been reset successfully.",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hosted_login_password_request_deserialization() {
        let json = r#"{"email": "test@example.com", "password": "MySecret123!"}"#; // pragma: allowlist secret
        let input: HostedLoginPasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(input.email, "test@example.com");
        assert_eq!(input.password, "MySecret123!");
    }

    #[test]
    fn test_hosted_login_token_response_serialization() {
        let response = HostedLoginTokenResponse {
            access_token: "tok".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["token_type"], "Bearer");
    }

    #[test]
    fn test_hosted_login_logout_request_deserialization() {
        let json = r#"{"post_logout_redirect_uri": "https://app.example.com", "client_id": "my-app"}"#;
        let input: HostedLoginLogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            input.post_logout_redirect_uri,
            Some("https://app.example.com".to_string())
        );
    }

    #[test]
    fn test_hosted_login_logout_request_empty() {
        let json = r#"{}"#;
        let input: HostedLoginLogoutRequest = serde_json::from_str(json).unwrap();
        assert!(input.post_logout_redirect_uri.is_none());
        assert!(input.client_id.is_none());
    }
}
