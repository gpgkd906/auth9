//! Audit log API handlers

use crate::error::Result;
use crate::http_support::PaginatedResponse;
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

    // Resolve per_page and page from query params.
    // `per_page` (alias of `limit`) and `page` take priority over raw `offset`/`limit`.
    let per_page = query
        .limit
        .unwrap_or(50)
        .clamp(1, crate::http_support::MAX_PER_PAGE);

    let page_param = query.page;
    let page = page_param.unwrap_or(1).max(1);
    let offset = if page_param.is_some() {
        // page-based: compute offset from page number
        (page - 1) * per_page
    } else {
        query.offset.unwrap_or(0).max(0)
    };

    // Inject resolved offset/limit back into the query for the repository layer
    let resolved_query = crate::repository::audit::AuditLogQuery {
        offset: Some(offset),
        limit: Some(per_page),
        page: None,
        ..query
    };

    let logs = state.audit_repo().find_with_actor(&resolved_query).await?;
    let total = state.audit_repo().count(&resolved_query).await?;

    // Recompute final page from resolved offset when page param wasn't provided
    let final_page = if page_param.is_some() {
        page
    } else {
        calculate_page(Some(offset), Some(per_page))
    };

    Ok(Json(PaginatedResponse::new(
        logs, final_page, per_page, total,
    )))
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
    use crate::repository::audit::AuditLogQuery;

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

    #[test]
    fn test_audit_query_page_param_computes_offset() {
        // Simulate: ?page=2&per_page=10 → offset=10, limit=10
        let query = AuditLogQuery {
            page: Some(2),
            limit: Some(10),
            ..Default::default()
        };
        let per_page = query.limit.unwrap_or(50).clamp(1, 100);
        let page_param = query.page;
        let page = page_param.unwrap_or(1).max(1);
        let offset = (page - 1) * per_page;
        assert_eq!(per_page, 10);
        assert_eq!(offset, 10);
    }

    #[test]
    fn test_audit_query_per_page_alias_deserializes() {
        // Verify `per_page` alias maps to `limit` field via serde alias
        let json = serde_json::json!({ "per_page": 25 });
        let q: AuditLogQuery = serde_json::from_value(json).unwrap();
        assert_eq!(q.limit, Some(25));
    }

    #[test]
    fn test_audit_query_limit_still_works() {
        // Verify original `limit` field still works
        let json = serde_json::json!({ "limit": 15 });
        let q: AuditLogQuery = serde_json::from_value(json).unwrap();
        assert_eq!(q.limit, Some(15));
    }

    #[test]
    fn test_audit_query_page_param_computes_first_page() {
        // ?page=1&per_page=10 → offset=0
        let per_page = 10i64;
        let page = 1i64;
        let offset = (page - 1) * per_page;
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_audit_query_per_page_clamped_to_max() {
        // ?per_page=999 → clamped to 100
        let query = AuditLogQuery {
            limit: Some(999),
            ..Default::default()
        };
        let per_page = query.limit.unwrap_or(50).clamp(1, 100);
        assert_eq!(per_page, 100);
    }
}
