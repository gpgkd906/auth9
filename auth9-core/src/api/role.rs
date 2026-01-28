//! Role and permission API handlers

use crate::api::{SuccessResponse, MessageResponse};
use crate::domain::{CreatePermissionInput, CreateRoleInput, UpdateRoleInput, AssignRolesInput};
use crate::error::Result;
use crate::server::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

// ==================== Permissions ====================

/// List permissions for a service
pub async fn list_permissions(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let permissions = state.rbac_service.list_permissions(service_id).await?;
    Ok(Json(SuccessResponse::new(permissions)))
}

/// Create permission
pub async fn create_permission(
    State(state): State<AppState>,
    Json(input): Json<CreatePermissionInput>,
) -> Result<impl IntoResponse> {
    let permission = state.rbac_service.create_permission(input).await?;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(permission))))
}

/// Delete permission
pub async fn delete_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.rbac_service.delete_permission(id).await?;
    Ok(Json(MessageResponse::new("Permission deleted successfully")))
}

// ==================== Roles ====================

/// List roles for a service
pub async fn list_roles(
    State(state): State<AppState>,
    Path(service_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let roles = state.rbac_service.list_roles(service_id).await?;
    Ok(Json(SuccessResponse::new(roles)))
}

/// Get role by ID
pub async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let role = state.rbac_service.get_role_with_permissions(id).await?;
    Ok(Json(SuccessResponse::new(role)))
}

/// Create role
pub async fn create_role(
    State(state): State<AppState>,
    Json(input): Json<CreateRoleInput>,
) -> Result<impl IntoResponse> {
    let role = state.rbac_service.create_role(input).await?;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(role))))
}

/// Update role
pub async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateRoleInput>,
) -> Result<impl IntoResponse> {
    let role = state.rbac_service.update_role(id, input).await?;
    Ok(Json(SuccessResponse::new(role)))
}

/// Delete role
pub async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.rbac_service.delete_role(id).await?;
    Ok(Json(MessageResponse::new("Role deleted successfully")))
}

// ==================== Role-Permission Assignment ====================

#[derive(Debug, Deserialize)]
pub struct AssignPermissionInput {
    pub permission_id: Uuid,
}

/// Assign permission to role
pub async fn assign_permission(
    State(state): State<AppState>,
    Path(role_id): Path<Uuid>,
    Json(input): Json<AssignPermissionInput>,
) -> Result<impl IntoResponse> {
    state
        .rbac_service
        .assign_permission_to_role(role_id, input.permission_id)
        .await?;
    Ok(Json(MessageResponse::new("Permission assigned to role")))
}

/// Remove permission from role
pub async fn remove_permission(
    State(state): State<AppState>,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    state
        .rbac_service
        .remove_permission_from_role(role_id, permission_id)
        .await?;
    Ok(Json(MessageResponse::new("Permission removed from role")))
}

// ==================== User-Role Assignment ====================

/// Assign roles to user in tenant
pub async fn assign_roles(
    State(state): State<AppState>,
    Json(input): Json<AssignRolesInput>,
) -> Result<impl IntoResponse> {
    // TODO: Get current user ID from auth context
    let granted_by = None;
    state.rbac_service.assign_roles(input, granted_by).await?;
    Ok(Json(MessageResponse::new("Roles assigned successfully")))
}

/// Get user roles in tenant
pub async fn get_user_roles(
    State(state): State<AppState>,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    let roles = state.rbac_service.get_user_roles(user_id, tenant_id).await?;
    Ok(Json(SuccessResponse::new(roles)))
}
