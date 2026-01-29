//! Tenant API handlers

use crate::api::{
    write_audit_log, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::domain::{CreateTenantInput, UpdateTenantInput};
use crate::error::Result;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
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
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.get(id).await?;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Create tenant
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateTenantInput>,
) -> Result<impl IntoResponse> {
    let tenant = state.tenant_service.create(input).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "tenant.create",
        "tenant",
        Some(tenant.id),
        None,
        serde_json::to_value(&tenant).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant))))
}

/// Update tenant
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTenantInput>,
) -> Result<impl IntoResponse> {
    let before = state.tenant_service.get(id).await?;
    let tenant = state.tenant_service.update(id, input).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "tenant.update",
        "tenant",
        Some(tenant.id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&tenant).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(tenant)))
}

/// Delete tenant
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let before = state.tenant_service.get(id).await?;
    let tenant = state.tenant_service.disable(id).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "tenant.disable",
        "tenant",
        Some(id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&tenant).ok(),
    )
    .await;
    Ok(Json(MessageResponse::new("Tenant disabled successfully")))
}
