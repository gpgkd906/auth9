//! Security Alert API handlers

use crate::api::{PaginatedResponse, SuccessResponse};
use crate::domain::{AlertSeverity, SecurityAlert, SecurityAlertType, StringUuid};
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::state::{HasSecurityAlerts, HasServices};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// List security alerts with pagination
pub async fn list_alerts<S: HasSecurityAlerts + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(params): Query<AlertsQuery>,
) -> Result<Json<PaginatedResponse<SecurityAlert>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SecurityAlertRead,
            scope: ResourceScope::Global,
        },
    )?;

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20);
    let unresolved_only = params.unresolved_only.unwrap_or(false);

    let (alerts, total) = state
        .security_detection_service()
        .list_filtered(
            page,
            per_page,
            unresolved_only,
            params.severity,
            params.alert_type,
        )
        .await?;

    Ok(Json(PaginatedResponse::new(alerts, page, per_page, total)))
}

/// Query parameters for alerts endpoint
#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    /// If true, only return unresolved alerts
    pub unresolved_only: Option<bool>,
    /// Filter by severity: low, medium, high, critical
    pub severity: Option<AlertSeverity>,
    /// Filter by alert type: brute_force, new_device, impossible_travel, suspicious_ip
    pub alert_type: Option<SecurityAlertType>,
}

/// Get a security alert by ID
pub async fn get_alert<S: HasSecurityAlerts + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(alert_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<SecurityAlert>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SecurityAlertRead,
            scope: ResourceScope::Global,
        },
    )?;

    let alert = state.security_detection_service().get(alert_id).await?;
    Ok(Json(SuccessResponse::new(alert)))
}

/// Resolve a security alert
pub async fn resolve_alert<S: HasSecurityAlerts + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(alert_id): Path<StringUuid>,
) -> Result<Json<SuccessResponse<SecurityAlert>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SecurityAlertResolve,
            scope: ResourceScope::Global,
        },
    )?;

    let resolved_by = StringUuid::from(auth.user_id);

    let alert = state
        .security_detection_service()
        .resolve(alert_id, resolved_by)
        .await?;

    Ok(Json(SuccessResponse::new(alert)))
}

/// Get count of unresolved alerts (for dashboard badge)
pub async fn get_unresolved_count<S: HasSecurityAlerts + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<Json<SuccessResponse<UnresolvedCountResponse>>, AppError> {
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::SecurityAlertRead,
            scope: ResourceScope::Global,
        },
    )?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alerts_query_defaults() {
        let query: AlertsQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.page, None);
        assert_eq!(query.per_page, None);
        assert_eq!(query.unresolved_only, None);
        assert!(query.severity.is_none());
        assert!(query.alert_type.is_none());
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
    fn test_alerts_query_with_filters() {
        let query: AlertsQuery = serde_json::from_str(
            r#"{"severity": "high", "alert_type": "brute_force"}"#,
        )
        .unwrap();
        assert_eq!(query.severity, Some(AlertSeverity::High));
        assert_eq!(query.alert_type, Some(SecurityAlertType::BruteForce));
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
