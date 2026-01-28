//! Tenant API handlers

use crate::api::{PaginatedResponse, PaginationQuery, SuccessResponse, MessageResponse};
use crate::domain::{CreateTenantInput, Tenant, UpdateTenantInput};
use crate::error::Result;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

/// List tenants
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let (tenants, total) = state
        .tenant_service
        .list(pagination.page, pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        tenants,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// Get tenant by ID
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.get(id).await?;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Create tenant
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateTenantInput>,
) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.create(input).await?;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant))))
}

/// Update tenant
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTenantInput>,
) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.update(id, input).await?;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Delete tenant
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.tenant_service.delete(id).await?;
    Ok(Json(MessageResponse::new("Tenant deleted successfully")))
}
