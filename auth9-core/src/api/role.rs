//! Role and permission API handlers

use crate::api::{
    extract_actor_id_generic, require_platform_admin_with_db, write_audit_log_generic,
    MessageResponse, SuccessResponse,
};
use crate::config::Config;
use crate::domain::{
    AssignRolesInput, CreatePermissionInput, CreateRoleInput, StringUuid, UpdateRoleInput,
};
use crate::error::{AppError, Result};
use crate::middleware::auth::{AuthUser, TokenType};
use crate::state::HasServices;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

/// Check if user can manage RBAC within a tenant
/// Platform admin can always manage, tenant owner can manage their tenant
fn require_rbac_management_permission(config: &Config, auth: &AuthUser) -> Result<()> {
    match auth.token_type {
        TokenType::Identity => {
            if config.is_platform_admin_email(&auth.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Tenant-scoped token required for RBAC management".to_string(),
                ))
            }
        }
        TokenType::TenantAccess => {
            // Only platform admin or tenant owner/admin with role management permissions
            let has_admin_role = auth.roles.iter().any(|r| r == "admin" || r == "owner");
            let has_rbac_permission = auth
                .permissions
                .iter()
                .any(|p| p == "rbac:write" || p == "rbac:*" || p == "role:write");

            if has_admin_role || has_rbac_permission {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Admin access required: you need admin privileges to manage roles".to_string(),
                ))
            }
        }
        TokenType::ServiceClient => Err(AppError::Forbidden(
            "Service client tokens cannot manage roles".to_string(),
        )),
    }
}

// ==================== Permissions ====================

/// List permissions for a service
pub async fn list_permissions<S: HasServices>(
    State(state): State<S>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service_id = StringUuid::from(service_id);
    let permissions = state.rbac_service().list_permissions(service_id).await?;
    Ok(Json(SuccessResponse::new(permissions)))
}

/// Create permission
/// Requires platform admin to create permissions
pub async fn create_permission<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<CreatePermissionInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    // Validate that the service_id references an existing service
    state
        .client_service()
        .get(input.service_id)
        .await
        .map_err(|_| {
            AppError::BadRequest(format!("Service '{}' does not exist", input.service_id))
        })?;

    let permission = state.rbac_service().create_permission(input).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "permission.create",
        "permission",
        Some(*permission.id),
        None,
        serde_json::to_value(&permission).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(permission))))
}

/// Delete permission
/// Requires platform admin to delete permissions
pub async fn delete_permission<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    let id = StringUuid::from(id);
    let before = state.rbac_service().get_permission(id).await?;
    state.rbac_service().delete_permission(id).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "permission.delete",
        "permission",
        Some(*id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;
    Ok(Json(MessageResponse::new(
        "Permission deleted successfully",
    )))
}

// ==================== Roles ====================

/// List roles for a service
pub async fn list_roles<S: HasServices>(
    State(state): State<S>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service_id = StringUuid::from(service_id);
    let roles = state.rbac_service().list_roles(service_id).await?;
    Ok(Json(SuccessResponse::new(roles)))
}

/// Get role by ID
pub async fn get_role<S: HasServices>(
    State(state): State<S>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let id = StringUuid::from(id);
    let role = state.rbac_service().get_role_with_permissions(id).await?;
    Ok(Json(SuccessResponse::new(role)))
}

/// Create role
/// Requires platform admin to create roles
pub async fn create_role<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<CreateRoleInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    // Validate that the service_id references an existing service
    state
        .client_service()
        .get(input.service_id)
        .await
        .map_err(|_| {
            AppError::BadRequest(format!("Service '{}' does not exist", input.service_id))
        })?;

    let role = state.rbac_service().create_role(input).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "role.create",
        "role",
        Some(*role.id),
        None,
        serde_json::to_value(&role).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(role))))
}

/// Update role
/// Requires platform admin to update roles
pub async fn update_role<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateRoleInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    let id = StringUuid::from(id);
    let before = state.rbac_service().get_role(id).await?;
    let role = state.rbac_service().update_role(id, input).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "role.update",
        "role",
        Some(*role.id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&role).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(role)))
}

/// Delete role
/// Requires platform admin to delete roles
pub async fn delete_role<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    let id = StringUuid::from(id);
    let before = state.rbac_service().get_role(id).await?;
    state.rbac_service().delete_role(id).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "role.delete",
        "role",
        Some(*id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Role deleted successfully")))
}

// ==================== Role-Permission Assignment ====================

#[derive(Debug, Deserialize)]
pub struct AssignPermissionInput {
    pub permission_id: Uuid,
}

/// Assign permission to role
/// Requires platform admin to assign permissions to roles
pub async fn assign_permission<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(input): Json<AssignPermissionInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    let role_id = StringUuid::from(role_id);
    let permission_id = StringUuid::from(input.permission_id);
    state
        .rbac_service()
        .assign_permission_to_role(role_id, permission_id)
        .await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "role.assign_permission",
        "role_permission",
        Some(*role_id),
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Permission assigned to role")))
}

/// Remove permission from role
/// Requires platform admin to remove permissions from roles
pub async fn remove_permission<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin
    require_platform_admin_with_db(&state, &auth).await?;

    let role_id = StringUuid::from(role_id);
    let permission_id = StringUuid::from(permission_id);
    state
        .rbac_service()
        .remove_permission_from_role(role_id, permission_id)
        .await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "role.remove_permission",
        "role_permission",
        Some(*role_id),
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Permission removed from role")))
}

// ==================== User-Role Assignment ====================

/// Assign roles to user in tenant
/// Requires platform admin or tenant owner to assign roles
pub async fn assign_roles<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<AssignRolesInput>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant owner
    require_rbac_management_permission(state.config(), &auth)?;

    // Additional check: for tenant access tokens, ensure user can only assign within their tenant
    if let TokenType::TenantAccess = auth.token_type {
        if auth.tenant_id != Some(input.tenant_id) {
            return Err(AppError::Forbidden(
                "Cannot assign roles in a different tenant".to_string(),
            ));
        }
    }

    // Validate that all role_ids belong to services within the target tenant
    let target_tenant = StringUuid::from(input.tenant_id);
    for role_id in &input.role_ids {
        let role = state
            .rbac_service()
            .get_role(StringUuid::from(*role_id))
            .await?;
        let service = state.client_service().get(*role.service_id).await?;
        if let Some(ref svc_tenant_id) = service.tenant_id {
            if *svc_tenant_id != target_tenant {
                return Err(AppError::BadRequest(format!(
                    "Role '{}' belongs to a service in a different tenant",
                    role_id
                )));
            }
        }
    }

    let granted_by = extract_actor_id_generic(&state, &headers).map(StringUuid::from);
    state.rbac_service().assign_roles(input, granted_by).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "rbac.assign_roles",
        "user_roles",
        None,
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Roles assigned successfully")))
}

/// Get user roles in tenant
pub async fn get_user_roles<S: HasServices>(
    State(state): State<S>,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    let user_id = StringUuid::from(user_id);
    let tenant_id = StringUuid::from(tenant_id);
    let roles = state
        .rbac_service()
        .get_user_roles(user_id, tenant_id)
        .await?;
    Ok(Json(SuccessResponse::new(roles)))
}

/// Get user assigned roles (raw records with IDs)
pub async fn get_user_assigned_roles<S: HasServices>(
    State(state): State<S>,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    let user_id = StringUuid::from(user_id);
    let tenant_id = StringUuid::from(tenant_id);
    let roles = state
        .rbac_service()
        .get_user_role_records(user_id, tenant_id)
        .await?;
    Ok(Json(SuccessResponse::new(roles)))
}

/// Unassign role from user in tenant
/// Requires platform admin or tenant owner to unassign roles
pub async fn unassign_role<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((user_id, tenant_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    // Check authorization: require platform admin or tenant owner
    require_rbac_management_permission(state.config(), &auth)?;

    // Additional check: for tenant access tokens, ensure user can only unassign within their tenant
    if let TokenType::TenantAccess = auth.token_type {
        if auth.tenant_id != Some(tenant_id) {
            return Err(AppError::Forbidden(
                "Cannot unassign roles in a different tenant".to_string(),
            ));
        }
    }

    let user_id = StringUuid::from(user_id);
    let tenant_id = StringUuid::from(tenant_id);
    let role_id = StringUuid::from(role_id);
    state
        .rbac_service()
        .unassign_role(user_id, tenant_id, role_id)
        .await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "rbac.unassign_role",
        "user_roles",
        Some(*user_id),
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Role unassigned successfully")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{MessageResponse, SuccessResponse};
    use crate::domain::{
        AssignRolesInput, CreatePermissionInput, CreateRoleInput, Permission, Role,
        RoleWithPermissions, UpdateRoleInput,
    };

    #[test]
    fn test_assign_permission_input_deserialization() {
        let json = r#"{"permission_id": "550e8400-e29b-41d4-a716-446655440000"}"#;
        let input: AssignPermissionInput = serde_json::from_str(json).unwrap();
        assert_eq!(
            input.permission_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_assign_permission_input_invalid_uuid() {
        let json = r#"{"permission_id": "not-a-uuid"}"#;
        let result: serde_json::Result<AssignPermissionInput> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_assign_permission_input_missing_field() {
        let json = r#"{}"#;
        let result: serde_json::Result<AssignPermissionInput> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_permission_input_deserialization() {
        let json = r#"{
            "service_id": "550e8400-e29b-41d4-a716-446655440000",
            "code": "users:read",
            "name": "Read Users",
            "description": "Read access to users"
        }"#;
        let input: CreatePermissionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.code, "users:read");
        assert_eq!(input.name, "Read Users");
        assert_eq!(input.description, Some("Read access to users".to_string()));
    }

    #[test]
    fn test_create_permission_input_minimal() {
        let json = r#"{
            "service_id": "550e8400-e29b-41d4-a716-446655440000",
            "code": "admin:all",
            "name": "Full Admin"
        }"#;
        let input: CreatePermissionInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.code, "admin:all");
        assert_eq!(input.name, "Full Admin");
        assert!(input.description.is_none());
    }

    #[test]
    fn test_create_permission_input_missing_required_field() {
        let json = r#"{
            "service_id": "550e8400-e29b-41d4-a716-446655440000",
            "code": "users:read"
        }"#;
        let result: serde_json::Result<CreatePermissionInput> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_role_input_deserialization() {
        let json = r#"{
            "service_id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "admin",
            "description": "Administrator role"
        }"#;
        let input: CreateRoleInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "admin");
        assert_eq!(input.description, Some("Administrator role".to_string()));
    }

    #[test]
    fn test_create_role_input_with_parent_and_permissions() {
        let json = r#"{
            "service_id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "editor",
            "description": "Content editor",
            "parent_role_id": "550e8400-e29b-41d4-a716-446655440001",
            "permission_ids": [
                "550e8400-e29b-41d4-a716-446655440002",
                "550e8400-e29b-41d4-a716-446655440003"
            ]
        }"#;
        let input: CreateRoleInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "editor");
        assert!(input.parent_role_id.is_some());
        assert_eq!(input.permission_ids.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_create_role_input_minimal() {
        let json = r#"{
            "service_id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "viewer"
        }"#;
        let input: CreateRoleInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "viewer");
        assert!(input.description.is_none());
        assert!(input.parent_role_id.is_none());
        assert!(input.permission_ids.is_none());
    }

    #[test]
    fn test_update_role_input_partial() {
        let json = r#"{"name": "super-admin"}"#;
        let input: UpdateRoleInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("super-admin".to_string()));
        assert!(input.description.is_none());
    }

    #[test]
    fn test_update_role_input_full() {
        let json = r#"{
            "name": "manager",
            "description": "Manager role with limited access"
        }"#;
        let input: UpdateRoleInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("manager".to_string()));
        assert_eq!(
            input.description,
            Some("Manager role with limited access".to_string())
        );
    }

    #[test]
    fn test_update_role_input_empty() {
        let json = r#"{}"#;
        let input: UpdateRoleInput = serde_json::from_str(json).unwrap();
        assert!(input.name.is_none());
        assert!(input.description.is_none());
        assert!(input.parent_role_id.is_none());
    }

    #[test]
    fn test_update_role_input_with_parent_role() {
        let json = r#"{
            "parent_role_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let input: UpdateRoleInput = serde_json::from_str(json).unwrap();
        assert!(input.parent_role_id.is_some());
        assert_eq!(
            input.parent_role_id.unwrap().unwrap().to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_assign_roles_input_single_role() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "role_ids": ["550e8400-e29b-41d4-a716-446655440002"]
        }"#;
        let input: AssignRolesInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.role_ids.len(), 1);
    }

    #[test]
    fn test_assign_roles_input_multiple_roles() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "role_ids": [
                "550e8400-e29b-41d4-a716-446655440002",
                "550e8400-e29b-41d4-a716-446655440003",
                "550e8400-e29b-41d4-a716-446655440004"
            ]
        }"#;
        let input: AssignRolesInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.role_ids.len(), 3);
    }

    #[test]
    fn test_assign_roles_input_empty_roles() {
        let json = r#"{
            "user_id": "550e8400-e29b-41d4-a716-446655440000",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "role_ids": []
        }"#;
        let input: AssignRolesInput = serde_json::from_str(json).unwrap();
        assert!(input.role_ids.is_empty());
    }

    #[test]
    fn test_assign_roles_input_missing_user_id() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440001",
            "role_ids": []
        }"#;
        let result: serde_json::Result<AssignRolesInput> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_success_response_with_permission() {
        let permission = Permission::default();
        let response = SuccessResponse::new(permission);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
    }

    #[test]
    fn test_success_response_with_role() {
        let role = Role::default();
        let response = SuccessResponse::new(role);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
    }

    #[test]
    fn test_success_response_with_role_with_permissions() {
        let role = Role::default();
        let permissions = vec![Permission::default()];
        let rwp = RoleWithPermissions { role, permissions };
        let response = SuccessResponse::new(rwp);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
        assert!(json.contains("permissions"));
    }

    #[test]
    fn test_message_response_permission_assigned() {
        let response = MessageResponse::new("Permission assigned to role");
        assert_eq!(response.message, "Permission assigned to role");
    }

    #[test]
    fn test_message_response_role_deleted() {
        let response = MessageResponse::new("Role deleted successfully");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Role deleted successfully"));
    }

    #[test]
    fn test_message_response_roles_assigned() {
        let response = MessageResponse::new("Roles assigned successfully");
        assert_eq!(response.message, "Roles assigned successfully");
    }

    #[test]
    fn test_success_response_with_vec_roles() {
        let roles = vec![Role::default(), Role::default()];
        let response = SuccessResponse::new(roles);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
    }

    #[test]
    fn test_success_response_with_vec_permissions() {
        let permissions = vec![Permission::default(), Permission::default()];
        let response = SuccessResponse::new(permissions);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("data"));
    }
}
