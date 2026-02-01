//! Security Alert API handlers

use crate::api::{PaginatedResponse, SuccessResponse};
use crate::domain::{SecurityAlert, StringUuid};
use crate::error::AppError;
use crate::state::HasSecurityAlerts;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;

/// List security alerts with pagination
pub async fn list_alerts<S: HasSecurityAlerts>(
    State(state): State<S>,
    Query(params): Query<AlertsQuery>,
) -> Result<Json<PaginatedResponse<SecurityAlert>>, AppError> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20);

    let (alerts, total) = if params.unresolved_only.unwrap_or(false) {
        state
            .security_detection_service()
            .list_unresolved(page, per_page)
            .await?
    } else {
        state
            .security_detection_service()
            .list(page, per_page)
            .await?
    };

    Ok(Json(PaginatedResponse::new(alerts, page, per_page, total)))
}

/// Query parameters for alerts endpoint
#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    /// If true, only return unresolved alerts
    pub unresolved_only: Option<bool>,
}

/// Get a security alert by ID
pub async fn get_alert<S: HasSecurityAlerts>(
    State(state): State<S>,
    Path(alert_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<SecurityAlert>>, AppError> {
    let alert = state.security_detection_service().get(alert_id).await?;
    Ok(Json(SuccessResponse::new(alert)))
}

/// Resolve a security alert
pub async fn resolve_alert<S: HasSecurityAlerts>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(alert_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<SecurityAlert>>, AppError> {
    let resolved_by = extract_user_id(&state, &headers)?;

    let alert = state
        .security_detection_service()
        .resolve(alert_id, resolved_by)
        .await?;

    Ok(Json(SuccessResponse::new(alert)))
}

/// Get count of unresolved alerts (for dashboard badge)
pub async fn get_unresolved_count<S: HasSecurityAlerts>(
    State(state): State<S>,
) -> Result<Json<SuccessResponse<UnresolvedCountResponse>>, AppError> {
    let (_, total) = state
        .security_detection_service()
        .list_unresolved(1, 1)
        .await?;

    Ok(Json(SuccessResponse::new(UnresolvedCountResponse {
        unresolved_count: total,
    })))
}

/// Response for unresolved count
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnresolvedCountResponse {
    pub unresolved_count: i64,
}

/// Extract user ID from JWT token
fn extract_user_id<S: HasSecurityAlerts>(
    state: &S,
    headers: &HeaderMap,
) -> Result<StringUuid, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".to_string()))?;

    if let Ok(claims) = state.jwt_manager().verify_identity_token(token) {
        return StringUuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
    }

    Err(AppError::Unauthorized(
        "Invalid or expired token".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alerts_query_defaults() {
        let query: AlertsQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.page, None);
        assert_eq!(query.per_page, None);
        assert_eq!(query.unresolved_only, None);
    }

    #[test]
    fn test_alerts_query_with_values() {
        let query: AlertsQuery =
            serde_json::from_str(r#"{"page": 2, "per_page": 50, "unresolved_only": true}"#)
                .unwrap();
        assert_eq!(query.page, Some(2));
        assert_eq!(query.per_page, Some(50));
        assert_eq!(query.unresolved_only, Some(true));
    }

    #[test]
    fn test_unresolved_count_response() {
        let response = UnresolvedCountResponse {
            unresolved_count: 5,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"unresolved_count\":5"));
    }
}
