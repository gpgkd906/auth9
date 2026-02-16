//! Password management API handlers

use crate::api::{write_audit_log_generic, MessageResponse, SuccessResponse};
use crate::domain::{
    AdminSetPasswordInput, ChangePasswordInput, ForgotPasswordInput, ResetPasswordInput, StringUuid,
};
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::state::{HasPasswordManagement, HasServices};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use validator::Validate;

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
pub async fn reset_password<S: HasPasswordManagement + HasServices>(
    State(state): State<S>,
    Json(input): Json<ResetPasswordInput>,
) -> Result<Json<MessageResponse>, AppError> {
    state.password_service().reset_password(input).await?;

    let _ = write_audit_log_generic(
        &state,
        &HeaderMap::new(),
        "password.reset",
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

/// Change password for authenticated user
pub async fn change_password<S: HasPasswordManagement + HasServices>(
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

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "password.changed",
        "user",
        Some(*user_id),
        None,
        None,
    )
    .await;

    Ok(Json(MessageResponse::new(
        "Password has been changed successfully.",
    )))
}

/// Get password policy for a tenant
pub async fn get_password_policy<S: HasPasswordManagement + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<crate::domain::PasswordPolicy>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SystemConfigRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    let policy = state.password_service().get_policy(tenant_id).await?;
    Ok(Json(SuccessResponse::new(policy)))
}

/// Update password policy for a tenant
pub async fn update_password_policy<S: HasPasswordManagement + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(tenant_id): Path<StringUuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse<crate::domain::PasswordPolicy>>, AppError> {
    // Authorization check MUST run before input validation
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;

    let input: crate::domain::UpdatePasswordPolicyInput =
        serde_json::from_value(body).map_err(|e| AppError::Validation(e.to_string()))?;
    let policy = state
        .password_service()
        .update_policy(tenant_id, input)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "tenant.password_policy.update",
        "tenant",
        Some(*tenant_id),
        None,
        serde_json::to_value(&policy).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(policy)))
}

/// Admin set password for a user (supports temporary passwords)
pub async fn admin_set_password<S: HasPasswordManagement + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(user_id): Path<StringUuid>,
    Json(input): Json<AdminSetPasswordInput>,
) -> Result<Json<MessageResponse>, AppError> {
    // Require UserWrite permission
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::UserWrite,
            scope: ResourceScope::Global,
        },
    )?;

    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    state
        .password_service()
        .admin_set_password(user_id, &input.password, input.temporary)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.password.admin_set",
        "user",
        Some(*user_id),
        None,
        Some(serde_json::json!({ "temporary": input.temporary })),
    )
    .await;

    Ok(Json(MessageResponse::new(
        "Password has been set successfully.",
    )))
}

/// Extract user ID from JWT token in Authorization header
fn extract_user_id<S: HasPasswordManagement + HasServices>(
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
    let jwt = HasServices::jwt_manager(state);

    if let Ok(claims) = jwt.verify_identity_token(token) {
        return StringUuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
    }

    let allowed = &state.config().jwt_tenant_access_allowed_audiences;
    if !allowed.is_empty() {
        if let Ok(claims) = jwt.verify_tenant_access_token_strict(token, allowed) {
            return StringUuid::parse_str(&claims.sub)
                .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
        }
    } else if !state.config().is_production() {
        #[allow(deprecated)]
        if let Ok(claims) = jwt.verify_tenant_access_token(token, None) {
            return StringUuid::parse_str(&claims.sub)
                .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
        }
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

    #[test]
    fn test_forgot_password_input_deserialization() {
        let json = r#"{"email": "test@example.com"}"#;
        let input: ForgotPasswordInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.email, "test@example.com");
    }

    #[test]
    fn test_reset_password_input_deserialization() {
        let json = r#"{"token": "abc123", "new_password": "NewPass123!"}"#;
        let input: ResetPasswordInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.token, "abc123");
        assert_eq!(input.new_password, "NewPass123!");
    }

    #[test]
    fn test_change_password_input_deserialization() {
        let json = r#"{"current_password": "OldPass", "new_password": "NewPass123!"}"#;
        let input: ChangePasswordInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.current_password, "OldPass");
        assert_eq!(input.new_password, "NewPass123!");
    }

    #[test]
    fn test_admin_set_password_input_deserialization() {
        let json = r#"{"password": "TempPass123!", "temporary": true}"#;
        let input: AdminSetPasswordInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.password, "TempPass123!");
        assert!(input.temporary);
    }

    #[test]
    fn test_admin_set_password_input_temporary_default() {
        let json = r#"{"password": "TempPass123!"}"#;
        let input: AdminSetPasswordInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.password, "TempPass123!");
        assert!(!input.temporary);
    }
}
