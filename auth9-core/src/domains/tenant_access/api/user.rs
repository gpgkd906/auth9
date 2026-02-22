//! User API handlers

use crate::api::{
    write_audit_log_generic, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::config::Config;
use crate::domain::{AddUserToTenantInput, CreateUserInput, StringUuid, UpdateUserInput, User};
use crate::error::{AppError, Result};
use crate::keycloak::{CreateKeycloakUserInput, KeycloakCredential, KeycloakUserUpdate};
use crate::middleware::auth::AuthUser;
use crate::policy::{
    enforce, enforce_with_state, is_platform_admin_with_db, PolicyAction, PolicyInput,
    ResourceScope,
};
use crate::state::{HasBranding, HasServices};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// User list query parameters (extends PaginationQuery with search)
#[derive(Debug, Clone, Deserialize)]
pub struct UserListQuery {
    #[serde(
        default = "default_page",
        deserialize_with = "crate::api::deserialize_page"
    )]
    pub page: i64,
    #[serde(
        default = "default_per_page",
        deserialize_with = "crate::api::deserialize_per_page",
        alias = "limit"
    )]
    pub per_page: i64,
    pub search: Option<String>,
    /// Optional tenant_id filter (platform admin only) to scope user list to a specific tenant
    pub tenant_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
struct UserListItem {
    #[serde(flatten)]
    user: User,
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<Uuid>,
}

fn with_tenant_context(users: Vec<User>, tenant_id: Option<Uuid>) -> Vec<UserListItem> {
    users
        .into_iter()
        .map(|user| UserListItem { user, tenant_id })
        .collect()
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

/// Check if user can manage the target tenant
/// Requires the user to be an owner of the target tenant or a platform admin
async fn require_tenant_owner<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    target_tenant_id: Uuid,
) -> Result<()> {
    enforce_with_state(
        state,
        auth,
        &PolicyInput {
            action: PolicyAction::TenantOwner,
            scope: ResourceScope::Tenant(StringUuid::from(target_tenant_id)),
        },
    )
    .await
}

/// Check if user is actually an owner of the target tenant (no platform admin bypass).
/// Used for ownership transfer operations where only the current owner should be allowed.
async fn require_actual_tenant_owner<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    target_tenant_id: Uuid,
) -> Result<()> {
    enforce_with_state(
        state,
        auth,
        &PolicyInput {
            action: PolicyAction::TenantActualOwner,
            scope: ResourceScope::Tenant(StringUuid::from(target_tenant_id)),
        },
    )
    .await
}

/// Check if user can manage users within a tenant
/// Platform admin can always manage, tenant admin with appropriate role can manage their tenant
fn require_user_management_permission(config: &Config, auth: &AuthUser) -> Result<()> {
    enforce(
        config,
        auth,
        &PolicyInput {
            action: PolicyAction::UserManage,
            scope: ResourceScope::Global,
        },
    )
}

/// List users
/// - Platform admin (any token type with platform admin email): can list all users
/// - Tenant user (TenantAccess token): can only list users in their tenant
/// - Supports optional `search` query parameter to filter by email or display_name
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(query): Query<UserListQuery>,
) -> Result<impl IntoResponse> {
    // Platform admin check applies to both Identity and TenantAccess tokens
    if is_platform_admin_with_db(&state, &auth).await {
        // Platform admin: if tenant_id is specified, scope to that tenant
        if let Some(tenant_id) = query.tenant_id {
            let users = state
                .user_service()
                .list_tenant_users(StringUuid::from(tenant_id), query.page, query.per_page)
                .await?;
            let total = users.len() as i64;
            return Ok(Json(PaginatedResponse::new(
                with_tenant_context(users, Some(tenant_id)),
                query.page,
                query.per_page,
                total,
            )));
        }
        // Platform admin: can list all users (with optional search)
        let (users, total) = if let Some(ref search) = query.search {
            if !search.is_empty() {
                state
                    .user_service()
                    .search(search, query.page, query.per_page)
                    .await?
            } else {
                state
                    .user_service()
                    .list(query.page, query.per_page)
                    .await?
            }
        } else {
            state
                .user_service()
                .list(query.page, query.per_page)
                .await?
        };

        return Ok(Json(PaginatedResponse::new(
            with_tenant_context(users, None),
            query.page,
            query.per_page,
            total,
        )));
    }

    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
    enforce_with_state(
        &state,
        &auth,
        &PolicyInput {
            action: PolicyAction::UserTenantRead,
            scope: ResourceScope::Tenant(StringUuid::from(tenant_id)),
        },
    )
    .await?;
    let users = state
        .user_service()
        .list_tenant_users(StringUuid::from(tenant_id), query.page, query.per_page)
        .await?;
    let total = users.len() as i64;
    Ok(Json(PaginatedResponse::new(
        with_tenant_context(users, Some(tenant_id)),
        query.page,
        query.per_page,
        total,
    )))
}

/// Get user by ID
/// Users can only read their own profile, or admins with user:read permission can read any user
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Authorization check: users can only read their own profile
    // unless they have admin permissions
    if auth.user_id != id {
        enforce_with_state(
            &state,
            &auth,
            &PolicyInput {
                action: PolicyAction::UserReadOther,
                scope: ResourceScope::User(StringUuid::from(id)),
            },
        )
        .await?;
    }

    let id = StringUuid::from(id);
    let user = state.user_service().get(id).await?;
    Ok(Json(SuccessResponse::new(user)))
}

/// Create user input (includes optional password for Keycloak)
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    #[serde(flatten)]
    pub user: CreateUserInput,
    pub password: Option<String>,
    pub tenant_id: Option<Uuid>,
}

/// Create user
///
/// This endpoint supports two modes:
/// 1. Authenticated (with valid JWT): Admin can always create users
/// 2. Unauthenticated (public registration): Only allowed if branding.allow_registration is true
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "Tenant Access",
    responses(
        (status = 201, description = "Created")
    )
)]
pub async fn create<S: HasServices + HasBranding>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<CreateUserRequest>,
) -> Result<impl IntoResponse> {
    // If Authorization is present, enforce permissions (tenant admin or platform admin).
    // Otherwise this is public registration (if enabled).
    let auth_user: Option<AuthUser> = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|auth_str| auth_str.strip_prefix("Bearer "))
        .and_then(|token| {
            let jwt = state.jwt_manager();

            // Reject service client tokens on this endpoint.
            if jwt.verify_service_client_token(token).is_ok() {
                return None;
            }

            if let Ok(claims) = jwt.verify_identity_token(token) {
                return AuthUser::from_identity_claims(claims).ok();
            }
            let allowed = &state.config().jwt_tenant_access_allowed_audiences;
            if !allowed.is_empty() {
                if let Ok(claims) = jwt.verify_tenant_access_token_strict(token, allowed) {
                    return AuthUser::from_tenant_access_claims(claims).ok();
                }
            } else if !state.config().is_production() {
                #[allow(deprecated)]
                if let Ok(claims) = jwt.verify_tenant_access_token(token, None) {
                    return AuthUser::from_tenant_access_claims(claims).ok();
                }
            }
            None
        });

    if let Some(ref auth) = auth_user {
        require_user_management_permission(state.config(), auth)?;

        if let Some(requested) = input.tenant_id {
            enforce_with_state(
                &state,
                auth,
                &PolicyInput {
                    action: PolicyAction::UserTenantRead,
                    scope: ResourceScope::Tenant(StringUuid::from(requested)),
                },
            )
            .await?;
        }
    } else {
        // If an Authorization header existed but we couldn't parse a supported token,
        // treat it as an authentication error (don't fall back to public registration).
        if headers.get(axum::http::header::AUTHORIZATION).is_some() {
            return Err(AppError::Unauthorized(
                "Invalid authorization token".to_string(),
            ));
        }

        let branding = state.branding_service().get_branding().await?;
        if !branding.allow_registration {
            return Err(AppError::Forbidden(
                "Public registration is disabled".to_string(),
            ));
        }
    }

    // Validate input before calling Keycloak (catches invalid emails early)
    input.user.validate()?;

    // Validate password against tenant password policy if provided
    if let Some(ref password) = input.password {
        // Determine tenant for policy: explicit tenant_id > caller's token tenant > default
        let effective_tenant_id = input
            .tenant_id
            .or_else(|| auth_user.as_ref().and_then(|a| a.tenant_id));
        let policy = if let Some(tenant_id) = effective_tenant_id {
            let tenant = state
                .tenant_service()
                .get(StringUuid::from(tenant_id))
                .await?;
            tenant.password_policy.unwrap_or_default()
        } else {
            crate::domain::PasswordPolicy::default()
        };
        if let Err(errors) = policy.validate_password(password) {
            return Err(AppError::Validation(errors.join("; ")));
        }
    }

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

/// Get current user's own profile
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_me<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(auth.user_id);
    let user = state.user_service().get(id).await?;
    Ok(Json(SuccessResponse::new(user)))
}

/// Update current user's own profile (display_name, avatar_url)
#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_me<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<UpdateUserInput>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(auth.user_id);
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
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

/// Update user
/// Self-update: users can update their own profile without admin permissions
/// Admin update: requires platform admin or tenant admin to update other users
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> Result<impl IntoResponse> {
    // Self-update: allow users to update their own profile
    // Admin update: require platform admin or tenant admin
    if auth.user_id != id {
        require_user_management_permission(state.config(), &auth)?;
    }

    let id = StringUuid::from(id);
    // Validate input before sending to Keycloak (prevent invalid data reaching external service)
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
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
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Deleted")
    )
)]
pub async fn delete<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(state.config(), &auth)?;

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

/// Valid values for role_in_tenant
const VALID_TENANT_ROLES: &[&str] = &["owner", "admin", "member"];

/// Validate that role_in_tenant is one of the allowed values
fn validate_role_in_tenant(role: &str) -> Result<()> {
    if VALID_TENANT_ROLES.contains(&role) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Invalid role_in_tenant '{}'. Must be one of: {}",
            role,
            VALID_TENANT_ROLES.join(", ")
        )))
    }
}

/// Add user to tenant
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddToTenantRequest {
    pub tenant_id: Uuid,
    pub role_in_tenant: String,
}

/// Add user to tenant
/// Requires the caller to be an owner of the target tenant
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/tenants",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn add_to_tenant<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(input): Json<AddToTenantRequest>,
) -> Result<impl IntoResponse> {
    // Validate role_in_tenant enum
    validate_role_in_tenant(&input.role_in_tenant)?;

    // Check authorization: require owner of the target tenant
    require_tenant_owner(&state, &auth, input.tenant_id).await?;

    // Block write operations on non-active tenants
    state
        .tenant_service()
        .require_active(StringUuid::from(input.tenant_id))
        .await?;

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
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleInTenantRequest {
    pub role_in_tenant: String,
}

/// Update user's role in a tenant
/// Requires the caller to be an owner of the target tenant.
/// Setting role to "owner" (ownership transfer) always requires the caller
/// to be the current tenant owner — platform admin bypass is not sufficient.
#[utoipa::path(
    put,
    path = "/api/v1/users/{user_id}/tenants/{tenant_id}",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_role_in_tenant<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateRoleInTenantRequest>,
) -> Result<impl IntoResponse> {
    // Validate role_in_tenant enum
    validate_role_in_tenant(&input.role_in_tenant)?;

    // Ownership transfer requires the caller to actually be the tenant owner
    // (platform admin bypass is NOT allowed for ownership changes)
    if input.role_in_tenant == "owner" {
        require_actual_tenant_owner(&state, &auth, tenant_id).await?;
    } else {
        // Check authorization: require owner of the target tenant
        require_tenant_owner(&state, &auth, tenant_id).await?;
    };

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
#[utoipa::path(
    delete,
    path = "/api/v1/users/{user_id}/tenants/{tenant_id}",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}/tenants",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/mfa",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn enable_mfa<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<EnableMfaInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(state.config(), &auth)?;

    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify admin's password as secondary confirmation
    let admin_user = state
        .user_service()
        .get(StringUuid::from(auth.user_id))
        .await?;
    let password_valid = state
        .keycloak_client()
        .validate_user_password(&admin_user.keycloak_id, &input.confirm_password)
        .await?;
    if !password_valid {
        return Err(AppError::Forbidden(
            "Password confirmation failed. Please provide your correct password to enable MFA."
                .to_string(),
        ));
    }

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

/// Input for enabling MFA - requires admin password confirmation
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct EnableMfaInput {
    /// Admin's own password for secondary verification
    #[validate(length(min = 1, max = 128))]
    pub confirm_password: String,
}

/// Input for disabling MFA - requires admin password confirmation
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct DisableMfaInput {
    /// Admin's own password for secondary verification
    #[validate(length(min = 1, max = 128))]
    pub confirm_password: String,
}

/// Disable MFA for a user
/// Requires platform admin or tenant admin + password confirmation
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/mfa",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn disable_mfa<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<DisableMfaInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant admin
    require_user_management_permission(state.config(), &auth)?;

    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify admin's password as secondary confirmation
    let admin_user = state
        .user_service()
        .get(StringUuid::from(auth.user_id))
        .await?;
    let password_valid = state
        .keycloak_client()
        .validate_user_password(&admin_user.keycloak_id, &input.confirm_password)
        .await?;
    if !password_valid {
        return Err(AppError::Forbidden(
            "Password confirmation failed. Please provide your correct password to disable MFA."
                .to_string(),
        ));
    }

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
/// Requires tenant access: platform admin, matching TenantAccess token, or matching ServiceClient token
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/users",
    tag = "Tenant Access",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list_by_tenant<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    enforce_with_state(
        &state,
        &auth,
        &PolicyInput {
            action: PolicyAction::TenantRead,
            scope: ResourceScope::Tenant(StringUuid::from(tenant_id)),
        },
    )
    .await?;

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

    #[test]
    fn test_validate_role_in_tenant_valid() {
        assert!(validate_role_in_tenant("owner").is_ok());
        assert!(validate_role_in_tenant("admin").is_ok());
        assert!(validate_role_in_tenant("member").is_ok());
    }

    #[test]
    fn test_validate_role_in_tenant_invalid() {
        assert!(validate_role_in_tenant("superadmin").is_err());
        assert!(validate_role_in_tenant("viewer").is_err());
        assert!(validate_role_in_tenant("platform_admin").is_err());
        assert!(validate_role_in_tenant("").is_err());
    }

    #[test]
    fn test_user_list_item_serializes_tenant_id_when_present() {
        let item = UserListItem {
            user: User::default(),
            tenant_id: Some(Uuid::new_v4()),
        };
        let value = serde_json::to_value(item).unwrap();
        assert!(value.get("tenant_id").is_some());
    }

    #[test]
    fn test_user_list_item_omits_tenant_id_when_absent() {
        let item = UserListItem {
            user: User::default(),
            tenant_id: None,
        };
        let value = serde_json::to_value(item).unwrap();
        assert!(value.get("tenant_id").is_none());
    }
}
