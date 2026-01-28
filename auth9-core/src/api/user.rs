//! User API handlers

use crate::api::{PaginatedResponse, PaginationQuery, SuccessResponse, MessageResponse};
use crate::domain::{CreateUserInput, UpdateUserInput, AddUserToTenantInput};
use crate::error::Result;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

/// List users
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let (users, total) = state
        .user_service
        .list(pagination.page, pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        users,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// Get user by ID
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = state.user_service.get(id).await?;
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
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateUserRequest>,
) -> Result<impl IntoResponse> {
    // TODO: Create user in Keycloak first, then in local DB
    // For now, generate a placeholder keycloak_id
    let keycloak_id = format!("kc-{}", uuid::Uuid::new_v4());
    let user = state.user_service.create(&keycloak_id, input.user).await?;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(user))))
}

/// Update user
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> Result<impl IntoResponse> {
    let user = state.user_service.update(id, input).await?;
    Ok(Json(SuccessResponse::new(user)))
}

/// Delete user
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.user_service.delete(id).await?;
    Ok(Json(MessageResponse::new("User deleted successfully")))
}

/// Add user to tenant
#[derive(Debug, Deserialize)]
pub struct AddToTenantRequest {
    pub tenant_id: Uuid,
    pub role_in_tenant: String,
}

pub async fn add_to_tenant(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(input): Json<AddToTenantRequest>,
) -> Result<impl IntoResponse> {
    let tenant_user = state
        .user_service
        .add_to_tenant(AddUserToTenantInput {
            user_id,
            tenant_id: input.tenant_id,
            role_in_tenant: input.role_in_tenant,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant_user))))
}

/// Remove user from tenant
pub async fn remove_from_tenant(
    State(state): State<AppState>,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    state
        .user_service
        .remove_from_tenant(user_id, tenant_id)
        .await?;
    Ok(Json(MessageResponse::new("User removed from tenant")))
}

/// Get user's tenants
pub async fn get_tenants(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let tenants = state.user_service.get_user_tenants(user_id).await?;
    Ok(Json(SuccessResponse::new(tenants)))
}

/// List users in a tenant
pub async fn list_by_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let users = state
        .user_service
        .list_tenant_users(tenant_id, pagination.page, pagination.per_page)
        .await?;
    Ok(Json(SuccessResponse::new(users)))
}
