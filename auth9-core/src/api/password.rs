//! Password management API handlers

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::{ChangePasswordInput, ForgotPasswordInput, ResetPasswordInput, StringUuid};
use crate::error::AppError;
use crate::state::HasPasswordManagement;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

/// Request password reset email
pub async fn forgot_password<S: HasPasswordManagement>(
    State(state): State<S>,
    Json(input): Json<ForgotPasswordInput>,
) -> Result<Json<MessageResponse>, AppError> {
    state.password_service().request_reset(input).await?;

    // Always return success to prevent email enumeration
    Ok(Json(MessageResponse::new(
        "If an account exists with this email, a password reset link has been sent.",
    )))
}

/// Reset password using token
pub async fn reset_password<S: HasPasswordManagement>(
    State(state): State<S>,
    Json(input): Json<ResetPasswordInput>,
) -> Result<Json<MessageResponse>, AppError> {
    state.password_service().reset_password(input).await?;

    Ok(Json(MessageResponse::new(
        "Password has been reset successfully.",
    )))
}

/// Change password for authenticated user
pub async fn change_password<S: HasPasswordManagement>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<ChangePasswordInput>,
) -> Result<Json<MessageResponse>, AppError> {
    // Extract user ID from JWT token
    let user_id = extract_user_id(&state, &headers)?;

    state
        .password_service()
        .change_password(user_id, input)
        .await?;

    Ok(Json(MessageResponse::new(
        "Password has been changed successfully.",
    )))
}

/// Get password policy for a tenant
pub async fn get_password_policy<S: HasPasswordManagement>(
    State(state): State<S>,
    Path(tenant_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<crate::domain::PasswordPolicy>>, AppError> {
    let policy = state.password_service().get_policy(tenant_id).await?;
    Ok(Json(SuccessResponse::new(policy)))
}

/// Update password policy for a tenant
pub async fn update_password_policy<S: HasPasswordManagement>(
    State(state): State<S>,
    Path(tenant_id): Path<StringUuid>,
    Json(input): Json<crate::domain::UpdatePasswordPolicyInput>,
) -> Result<Json<SuccessResponse<crate::domain::PasswordPolicy>>, AppError> {
    let policy = state
        .password_service()
        .update_policy(tenant_id, input)
        .await?;
    Ok(Json(SuccessResponse::new(policy)))
}

/// Extract user ID from JWT token in Authorization header
fn extract_user_id<S: HasPasswordManagement>(
    state: &S,
    headers: &HeaderMap,
) -> Result<StringUuid, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".to_string()))?;

    // Try identity token first, then tenant access token
    if let Ok(claims) = state.jwt_manager().verify_identity_token(token) {
        return StringUuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
    }

    if let Ok(claims) = state.jwt_manager().verify_tenant_access_token(token, None) {
        return StringUuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
    }

    Err(AppError::Unauthorized(
        "Invalid or expired token".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_response() {
        let response = MessageResponse::new("Test message");
        assert_eq!(response.message, "Test message");
    }
}
