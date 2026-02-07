//! User API handlers

use crate::api::{
    write_audit_log_generic, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::domain::{AddUserToTenantInput, CreateUserInput, StringUuid, UpdateUserInput};
use crate::error::{AppError, Result};
use crate::keycloak::{CreateKeycloakUserInput, KeycloakCredential, KeycloakUserUpdate};
use crate::middleware::auth::{AuthUser, TokenType};
use crate::state::{HasBranding, HasServices};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

/// Check if user can manage the target tenant
/// Requires the user to be an owner of the target tenant
async fn require_tenant_owner<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    target_tenant_id: Uuid,
) -> Result<()> {
    // For TenantAccess tokens, check if the token is for this tenant with owner role
    if auth.token_type == TokenType::TenantAccess {
        if auth.tenant_id == Some(target_tenant_id) {
            if auth.roles.iter().any(|r| r == "owner") {
                return Ok(());
            }
        }
        return Err(AppError::Forbidden(
            "Owner access required: you must be an owner of this tenant".to_string(),
        ));
    }

    // For Identity tokens, check the database if user is owner of target tenant
    let user_id = StringUuid::from(auth.user_id);
    let tenant_id = StringUuid::from(target_tenant_id);
    let tenant_users = state.user_service().get_user_tenants(user_id).await?;

    for tu in tenant_users {
        if tu.tenant_id == tenant_id && tu.role_in_tenant == "owner" {
            return Ok(());
        }
    }

    Err(AppError::Forbidden(
        "Owner access required: you must be an owner of this tenant to perform this action"
            .to_string(),
    ))
}

/// Check if user can manage users within a tenant
/// Platform admin can always manage, tenant admin with appropriate role can manage their tenant
fn require_user_management_permission(auth: &AuthUser) -> Result<()> {
    match auth.token_type {
        TokenType::Identity => Ok(()),
        TokenType::TenantAccess => {
            // Check if user has admin/owner role or user:write/user:delete permissions
            let has_admin_role = auth.roles.iter().any(|r| r == "admin" || r == "owner");
            let has_user_write_permission = auth
                .permissions
                .iter()
                .any(|p| p == "user:write" || p == "user:delete" || p == "user:*");

            if has_admin_role || has_user_write_permission {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Admin access required: you need admin privileges to manage users".to_string(),
                ))
            }
        }
    }
}

/// List users
/// - Platform admin (Identity token): can list all users
/// - Tenant user (TenantAccess token): can only list users in their tenant
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    match auth.token_type {
        TokenType::Identity => {
            // Platform admin: can list all users
            let (users, total) = state
                .user_service()
                .list(pagination.page, pagination.per_page)
                .await?;

            Ok(Json(PaginatedResponse::new(
                users,
                pagination.page,
                pagination.per_page,
                total,
            )))
        }
        TokenType::TenantAccess => {
            // Tenant user: can only list users in their tenant
            let tenant_id = auth
                .tenant_id
                .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
            let users = state
                .user_service()
                .list_tenant_users(
                    StringUuid::from(tenant_id),
                    pagination.page,
                    pagination.per_page,
                )
                .await?;
            // list_tenant_users returns Vec, wrap in PaginatedResponse
            let total = users.len() as i64;
            Ok(Json(PaginatedResponse::new(
                users,
                pagination.page,
                pagination.per_page,
                total,
            )))
        }
    }
}

/// Get user by ID
/// Users can only read their own profile, or admins with user:read permission can read any user
pub async fn get<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Authorization check: users can only read their own profile
    // unless they have admin permissions
    if auth.user_id != id {
        // Check if user has admin permissions via TenantAccess token
        if auth.token_type == TokenType::TenantAccess {
            let has_admin_permission = auth.roles.iter().any(|r| r == "admin" || r == "owner")
                || auth
                    .permissions
                    .iter()
                    .any(|p| p == "user:read" || p == "user:*");
            if !has_admin_permission {
                return Err(AppError::Forbidden(
                    "Access denied: you can only view your own profile".to_string(),
                ));
            }
        } else {
            // Identity tokens: check if user is owner/admin of any tenant the target user belongs to
            let target_user_id = StringUuid::from(id);
            let auth_user_id = StringUuid::from(auth.user_id);

            // Get tenants where auth user is owner/admin
            let auth_user_tenants = state.user_service().get_user_tenants(auth_user_id).await?;
            let target_user_tenants = state
                .user_service()
                .get_user_tenants(target_user_id)
                .await?;

            let auth_user_admin_tenant_ids: std::collections::HashSet<_> = auth_user_tenants
                .iter()
                .filter(|tu| tu.role_in_tenant == "owner" || tu.role_in_tenant == "admin")
                .map(|tu| tu.tenant_id)
                .collect();

            let target_user_tenant_ids: std::collections::HashSet<_> =
                target_user_tenants.iter().map(|tu| tu.tenant_id).collect();

            // Auth user must be admin of at least one tenant the target user belongs to
            let has_shared_admin_tenant = auth_user_admin_tenant_ids
                .intersection(&target_user_tenant_ids)
                .next()
                .is_some();

            if !has_shared_admin_tenant {
                return Err(AppError::Forbidden(
                    "Access denied: you can only view your own profile".to_string(),
                ));
            }
        }
    }

    let id = StringUuid::from(id);
    let user = state.user_service().get(id).await?;
    Ok(Json(SuccessResponse::new(user)))
}

/// Create user input (includes optional password for Keycloak)
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    #[serde(flatten)]
    pub user: CreateUserInput,
    pub password: Option<String>,
}

/// Create user
///
/// This endpoint supports two modes:
/// 1. Authenticated (with valid JWT): Admin can always create users
/// 2. Unauthenticated (public registration): Only allowed if branding.allow_registration is true
pub async fn create<S: HasServices + HasBranding>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<CreateUserRequest>,
) -> Result<impl IntoResponse> {
    // Check if this is an authenticated request (admin creating user)
    let is_authenticated = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "))
        .map(|token| {
            state.jwt_manager().verify_identity_token(token).is_ok()
                || state
                    .jwt_manager()
                    .verify_tenant_access_token(token, None)
                    .is_ok()
        })
        .unwrap_or(false);

    // If not authenticated, check if public registration is allowed
    if !is_authenticated {
        let branding = state.branding_service().get_branding().await?;
        if !branding.allow_registration {
            return Err(AppError::Forbidden(
                "Public registration is disabled".to_string(),
            ));
        }
    }

    // Validate input before calling Keycloak (catches invalid emails early)
    input.user.validate()?;

    let credentials = input.password.map(|password| {
        vec![KeycloakCredential {
            credential_type: "password".to_string(),
            value: password,
            temporary: false,
        }]
    });

    let keycloak_id = state
        .keycloak_client()
        .create_user(&CreateKeycloakUserInput {
            username: input.user.email.clone(),
            email: input.user.email.clone(),
            first_name: input.user.display_name.clone(),
            last_name: None,
            enabled: true,
            email_verified: false,
            credentials,
        })
        .await?;

    let user = state
        .user_service()
        .create(&keycloak_id, input.user)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.create",
        "user",
        Some(*user.id),
        None,
        serde_json::to_value(&user).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(user))))
}

/// Update user
/// Requires platform admin or tenant admin to update users
pub async fn update<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(&auth)?;

    let id = StringUuid::from(id);
    let before = state.user_service().get(id).await?;
    if input.display_name.is_some() {
        let update = KeycloakUserUpdate {
            username: None,
            email: None,
            first_name: input.display_name.clone(),
            last_name: None,
            enabled: None,
            email_verified: None,
            required_actions: None,
        };
        state
            .keycloak_client()
            .update_user(&before.keycloak_id, &update)
            .await?;
    }
    let user = state.user_service().update(id, input).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.update",
        "user",
        Some(*user.id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&user).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(user)))
}

/// Delete user
/// Requires platform admin or tenant admin to delete users
pub async fn delete<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(&auth)?;

    let id = StringUuid::from(id);
    let before = state.user_service().get(id).await?;
    if let Err(err) = state
        .keycloak_client()
        .delete_user(&before.keycloak_id)
        .await
    {
        if !matches!(err, crate::error::AppError::NotFound(_)) {
            return Err(err);
        }
    }
    state.user_service().delete(id).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.delete",
        "user",
        Some(*id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("User deleted successfully")))
}

/// Add user to tenant
#[derive(Debug, Deserialize)]
pub struct AddToTenantRequest {
    pub tenant_id: Uuid,
    pub role_in_tenant: String,
}

/// Add user to tenant
/// Requires the caller to be an owner of the target tenant
pub async fn add_to_tenant<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(input): Json<AddToTenantRequest>,
) -> Result<impl IntoResponse> {
    // Check authorization: require owner of the target tenant
    require_tenant_owner(&state, &auth, input.tenant_id).await?;

    let tenant_user = state
        .user_service()
        .add_to_tenant(AddUserToTenantInput {
            user_id,
            tenant_id: input.tenant_id,
            role_in_tenant: input.role_in_tenant,
        })
        .await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.add_to_tenant",
        "tenant_user",
        Some(*tenant_user.id),
        None,
        serde_json::to_value(&tenant_user).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant_user))))
}

/// Update user's role in a tenant
#[derive(Debug, Deserialize)]
pub struct UpdateRoleInTenantRequest {
    pub role_in_tenant: String,
}

/// Update user's role in a tenant
/// Requires the caller to be an owner of the target tenant
pub async fn update_role_in_tenant<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateRoleInTenantRequest>,
) -> Result<impl IntoResponse> {
    // Check authorization: require owner of the target tenant
    require_tenant_owner(&state, &auth, tenant_id).await?;

    let user_id = StringUuid::from(user_id);
    let tenant_id = StringUuid::from(tenant_id);
    let tenant_user = state
        .user_service()
        .update_role_in_tenant(user_id, tenant_id, input.role_in_tenant)
        .await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.update_role_in_tenant",
        "tenant_user",
        Some(*tenant_user.id),
        None,
        serde_json::to_value(&tenant_user).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(tenant_user)))
}

/// Remove user from tenant
/// Requires the caller to be an owner of the target tenant
pub async fn remove_from_tenant<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    // Check authorization: require owner of the target tenant
    require_tenant_owner(&state, &auth, tenant_id).await?;

    let user_id = StringUuid::from(user_id);
    let tenant_id = StringUuid::from(tenant_id);
    state
        .user_service()
        .remove_from_tenant(user_id, tenant_id)
        .await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.remove_from_tenant",
        "tenant_user",
        None,
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("User removed from tenant")))
}

/// Get user's tenants (with tenant data for display)
pub async fn get_tenants<S: HasServices>(
    State(state): State<S>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user_id = StringUuid::from(user_id);
    // Use get_user_tenants_with_tenant to include tenant name, slug, logo_url for UI display
    let tenants = state
        .user_service()
        .get_user_tenants_with_tenant(user_id)
        .await?;
    Ok(Json(SuccessResponse::new(tenants)))
}

/// Enable MFA for a user
/// Requires platform admin or tenant admin
pub async fn enable_mfa<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(&auth)?;

    let id = StringUuid::from(id);
    let user = state.user_service().get(id).await?;
    let update = KeycloakUserUpdate {
        username: None,
        email: None,
        first_name: None,
        last_name: None,
        enabled: None,
        email_verified: None,
        required_actions: Some(vec!["CONFIGURE_TOTP".to_string()]),
    };
    state
        .keycloak_client()
        .update_user(&user.keycloak_id, &update)
        .await?;
    let updated = state.user_service().set_mfa_enabled(id, true).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.mfa.enable",
        "user",
        Some(*updated.id),
        None,
        serde_json::to_value(&updated).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(updated)))
}

/// Disable MFA for a user
/// Requires platform admin or tenant admin
pub async fn disable_mfa<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(&auth)?;

    let id = StringUuid::from(id);
    let user = state.user_service().get(id).await?;
    state
        .keycloak_client()
        .remove_totp_credentials(&user.keycloak_id)
        .await?;
    let update = KeycloakUserUpdate {
        username: None,
        email: None,
        first_name: None,
        last_name: None,
        enabled: None,
        email_verified: None,
        required_actions: Some(vec![]),
    };
    state
        .keycloak_client()
        .update_user(&user.keycloak_id, &update)
        .await?;
    let updated = state.user_service().set_mfa_enabled(id, false).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "user.mfa.disable",
        "user",
        Some(*updated.id),
        None,
        serde_json::to_value(&updated).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(updated)))
}

/// List users in a tenant
pub async fn list_by_tenant<S: HasServices>(
    State(state): State<S>,
    Path(tenant_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let tenant_id = StringUuid::from(tenant_id);
    let users = state
        .user_service()
        .list_tenant_users(tenant_id, pagination.page, pagination.per_page)
        .await?;
    Ok(Json(SuccessResponse::new(users)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{MessageResponse, PaginatedResponse, SuccessResponse};
    use crate::domain::{CreateUserInput, TenantUser, UpdateUserInput, User};

    #[test]
    fn test_create_user_request_deserialization() {
        let json = r#"{
            "email": "user@example.com",
            "display_name": "John Doe",
            "avatar_url": "https://example.com/avatar.png",
            "password": "secret123"
        }"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user.email, "user@example.com");
        assert_eq!(request.user.display_name, Some("John Doe".to_string()));
        assert_eq!(
            request.user.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
        assert_eq!(request.password, Some("secret123".to_string()));
    }

    #[test]
    fn test_create_user_request_minimal() {
        let json = r#"{
            "email": "minimal@example.com"
        }"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user.email, "minimal@example.com");
        assert!(request.user.display_name.is_none());
        assert!(request.user.avatar_url.is_none());
        assert!(request.password.is_none());
    }

    #[test]
    fn test_create_user_request_missing_email() {
        let json = r#"{
            "display_name": "No Email User"
        }"#;
        let result: serde_json::Result<CreateUserRequest> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_user_input_deserialization() {
        let json = r#"{
            "email": "test@example.com",
            "display_name": "Test User"
        }"#;
        let input: CreateUserInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.email, "test@example.com");
        assert_eq!(input.display_name, Some("Test User".to_string()));
    }

    #[test]
    fn test_update_user_input_partial() {
        let json = r#"{"display_name": "Updated Name"}"#;
        let input: UpdateUserInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.display_name, Some("Updated Name".to_string()));
        assert!(input.avatar_url.is_none());
    }

    #[test]
    fn test_update_user_input_avatar_update() {
        let json = r#"{"avatar_url": "https://new-avatar.example.com/img.jpg"}"#;
        let input: UpdateUserInput = serde_json::from_str(json).unwrap();
        assert!(input.display_name.is_none());
        assert_eq!(
            input.avatar_url,
            Some("https://new-avatar.example.com/img.jpg".to_string())
        );
    }

    #[test]
    fn test_add_to_tenant_request_deserialization() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "role_in_tenant": "admin"
        }"#;
        let request: AddToTenantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.tenant_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(request.role_in_tenant, "admin");
    }

    #[test]
    fn test_add_to_tenant_request_member_role() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "role_in_tenant": "member"
        }"#;
        let request: AddToTenantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.role_in_tenant, "member");
    }

    #[test]
    fn test_add_to_tenant_request_missing_tenant_id() {
        let json = r#"{
            "role_in_tenant": "admin"
        }"#;
        let result: serde_json::Result<AddToTenantRequest> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_to_tenant_request_missing_role() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let result: serde_json::Result<AddToTenantRequest> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_to_tenant_request_invalid_uuid() {
        let json = r#"{
            "tenant_id": "not-a-valid-uuid",
            "role_in_tenant": "admin"
        }"#;
        let result: serde_json::Result<AddToTenantRequest> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_user_to_tenant_input_creation() {
        let input = AddUserToTenantInput {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role_in_tenant: "owner".to_string(),
        };
        assert_eq!(input.role_in_tenant, "owner");
    }

    #[test]
    fn test_create_user_request_with_all_fields() {
        let json = r#"{
            "email": "full@example.com",
            "display_name": "Full Name",
            "avatar_url": "https://cdn.example.com/avatars/full.png",
            "password": "SecureP@ssw0rd!"
        }"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.user.email, "full@example.com");
        assert_eq!(request.user.display_name, Some("Full Name".to_string()));
        assert!(request.user.avatar_url.is_some());
        assert!(request.password.is_some());
    }

    #[test]
    fn test_update_user_input_empty() {
        let json = r#"{}"#;
        let input: UpdateUserInput = serde_json::from_str(json).unwrap();

        assert!(input.display_name.is_none());
        assert!(input.avatar_url.is_none());
    }

    #[test]
    fn test_update_user_input_both_fields() {
        let json = r#"{
            "display_name": "New Name",
            "avatar_url": "https://example.com/new-avatar.png"
        }"#;
        let input: UpdateUserInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.display_name, Some("New Name".to_string()));
        assert_eq!(
            input.avatar_url,
            Some("https://example.com/new-avatar.png".to_string())
        );
    }

    #[test]
    fn test_add_to_tenant_request_roundtrip() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "role_in_tenant": "viewer"
        }"#;

        let request: AddToTenantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.tenant_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(request.role_in_tenant, "viewer");
    }

    #[test]
    fn test_create_user_input_email_only() {
        let json = r#"{"email": "simple@example.com"}"#;
        let input: CreateUserInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.email, "simple@example.com");
        assert!(input.display_name.is_none());
        assert!(input.avatar_url.is_none());
    }

    #[test]
    fn test_add_to_tenant_request_various_roles() {
        let roles = vec!["admin", "member", "viewer", "guest", "superuser"];

        for role in roles {
            let json = format!(
                r#"{{"tenant_id": "550e8400-e29b-41d4-a716-446655440000", "role_in_tenant": "{}"}}"#,
                role
            );
            let request: AddToTenantRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(request.role_in_tenant, role);
        }
    }

    #[test]
    fn test_success_response_with_user() {
        let user = User::default();
        let response = SuccessResponse::new(user);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
        assert!(json.contains("email"));
    }

    #[test]
    fn test_success_response_with_tenant_user() {
        let tenant_user = TenantUser {
            id: crate::domain::StringUuid::new_v4(),
            tenant_id: crate::domain::StringUuid::new_v4(),
            user_id: crate::domain::StringUuid::new_v4(),
            role_in_tenant: "member".to_string(),
            joined_at: chrono::Utc::now(),
        };
        let response = SuccessResponse::new(tenant_user);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
    }

    #[test]
    fn test_paginated_response_with_users() {
        let users = vec![User::default(), User::default()];
        let response = PaginatedResponse::new(users, 1, 10, 2);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
        assert!(json.contains("pagination"));
        assert!(json.contains("\"total\":2"));
    }

    #[test]
    fn test_message_response_user_deleted() {
        let response = MessageResponse::new("User deleted successfully");
        assert_eq!(response.message, "User deleted successfully");
    }

    #[test]
    fn test_message_response_user_removed_from_tenant() {
        let response = MessageResponse::new("User removed from tenant");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("User removed from tenant"));
    }

    #[test]
    fn test_success_response_with_vec_tenant_users() {
        let tenant_user = TenantUser {
            id: crate::domain::StringUuid::new_v4(),
            tenant_id: crate::domain::StringUuid::new_v4(),
            user_id: crate::domain::StringUuid::new_v4(),
            role_in_tenant: "admin".to_string(),
            joined_at: chrono::Utc::now(),
        };
        let tenant_users = vec![tenant_user];
        let response = SuccessResponse::new(tenant_users);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
    }

    #[test]
    fn test_create_user_request_password_only() {
        let json = r#"{
            "email": "pwd@example.com",
            "password": "OnlyPassword123"
        }"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user.email, "pwd@example.com");
        assert!(request.user.display_name.is_none());
        assert_eq!(request.password, Some("OnlyPassword123".to_string()));
    }

    #[test]
    fn test_create_user_request_special_characters_in_email() {
        let json = r#"{
            "email": "user+tag@example.com"
        }"#;
        let request: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.user.email, "user+tag@example.com");
    }

    #[test]
    fn test_update_user_input_null_avatar() {
        let json = r#"{"avatar_url": null}"#;
        let input: UpdateUserInput = serde_json::from_str(json).unwrap();
        assert!(input.avatar_url.is_none());
    }
}
