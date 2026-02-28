//! Audit log API handlers

use crate::api::PaginatedResponse;
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::repository::audit::AuditLogQuery;
use crate::repository::AuditRepository;
use crate::state::HasServices;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};

/// List audit logs with actor information (email, display_name)
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs",
    tag = "Security & Observability",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(query): Query<AuditLogQuery>,
) -> Result<impl IntoResponse> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::AuditRead,
            scope: ResourceScope::Global,
        },
    )?;

    // Use find_with_actor to include actor email/display_name in response
    let logs = state.audit_repo().find_with_actor(&query).await?;
    let total = state.audit_repo().count(&query).await?;

    let per_page = query.limit.unwrap_or(50).min(crate::api::MAX_PER_PAGE);
    let page = calculate_page(query.offset, Some(per_page));

    Ok(Json(PaginatedResponse::new(logs, page, per_page, total)))
}

/// Calculate pagination page from offset and limit
fn calculate_page(offset: Option<i64>, limit: Option<i64>) -> i64 {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(50);
    offset / limit + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_page_defaults() {
        assert_eq!(calculate_page(None, None), 1);
    }

    #[test]
    fn test_calculate_page_first_page() {
        assert_eq!(calculate_page(Some(0), Some(50)), 1);
    }

    #[test]
    fn test_calculate_page_second_page() {
        assert_eq!(calculate_page(Some(50), Some(50)), 2);
    }

    #[test]
    fn test_calculate_page_custom_limit() {
        assert_eq!(calculate_page(Some(20), Some(10)), 3);
    }

    #[test]
    fn test_calculate_page_large_offset() {
        assert_eq!(calculate_page(Some(200), Some(50)), 5);
    }
}
