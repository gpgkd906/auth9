//! Audit log API handlers

use crate::api::{PaginatedResponse, SuccessResponse};
use crate::error::Result;
use crate::repository::audit::AuditLogQuery;
use crate::server::AppState;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};

/// List audit logs
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<AuditLogQuery>,
) -> Result<impl IntoResponse> {
    let logs = state.audit_repo.find(&query).await?;
    let total = state.audit_repo.count(&query).await?;
    
    let page = query.offset.unwrap_or(0) / query.limit.unwrap_or(50) + 1;
    let per_page = query.limit.unwrap_or(50);
    
    Ok(Json(PaginatedResponse::new(logs, page, per_page, total)))
}
