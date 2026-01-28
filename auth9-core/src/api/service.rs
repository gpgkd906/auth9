//! Service/Client API handlers

use crate::api::{PaginatedResponse, PaginationQuery, SuccessResponse, MessageResponse};
use crate::domain::{CreateServiceInput, UpdateServiceInput};
use crate::error::Result;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ListServicesQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,
    pub tenant_id: Option<Uuid>,
}

/// List services
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListServicesQuery>,
) -> Result<impl IntoResponse> {
    let (services, total) = state
        .client_service
        .list(query.tenant_id, query.pagination.page, query.pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        services,
        query.pagination.page,
        query.pagination.per_page,
        total,
    )))
}

/// Get service by ID
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service.get(id).await?;
    Ok(Json(SuccessResponse::new(service)))
}

/// Create service
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateServiceInput>,
) -> Result<impl IntoResponse> {
    let service_with_secret = state.client_service.create(input).await?;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(service_with_secret))))
}

/// Update service
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateServiceInput>,
) -> Result<impl IntoResponse> {
    let service = state.client_service.update(id, input).await?;
    Ok(Json(SuccessResponse::new(service)))
}

/// Regenerate client secret
#[derive(Serialize)]
pub struct SecretResponse {
    pub client_secret: String,
}

pub async fn regenerate_secret(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let secret = state.client_service.regenerate_secret(id).await?;
    Ok(Json(SuccessResponse::new(SecretResponse {
        client_secret: secret,
    })))
}

/// Delete service
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    state.client_service.delete(id).await?;
    Ok(Json(MessageResponse::new("Service deleted successfully")))
}
