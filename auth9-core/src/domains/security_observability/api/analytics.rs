//! Analytics API handlers

use crate::error::AppError;
use crate::http_support::{
    default_page, default_per_page, deserialize_page, deserialize_per_page, PaginatedResponse,
    PaginationQuery, SuccessResponse,
};
use crate::models::analytics::{DailyTrendPoint, LoginEvent, LoginStats};
use crate::models::common::StringUuid;
use crate::state::HasAnalytics;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

/// Get login statistics
#[utoipa::path(
    get,
    path = "/api/v1/analytics/login-stats",
    tag = "Security & Observability",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_stats<S: HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<SuccessResponse<LoginStats>>, AppError> {
    let tenant_id = params.tenant_id;

    // First check for start/end date parameters (ISO 8601 or YYYY-MM-DD format)
    if let (Some(start), Some(end)) = (&params.start, &params.end) {
        if let (Some(start_dt), Some(end_dt)) =
            (parse_date_param(start, false), parse_date_param(end, true))
        {
            let stats = state
                .analytics_service()
                .get_stats_for_range(tenant_id, start_dt, end_dt)
                .await?;
            return Ok(Json(SuccessResponse::new(stats)));
        }
    }

    // Fallback to period/days parameters
    let stats = match params.period.as_deref() {
        Some("daily") | Some("day") => state.analytics_service().get_daily_stats(tenant_id).await?,
        Some("weekly") | Some("week") => {
            state
                .analytics_service()
                .get_weekly_stats(tenant_id)
                .await?
        }
        Some("monthly") | Some("month") => {
            state
                .analytics_service()
                .get_monthly_stats(tenant_id)
                .await?
        }
        _ => {
            let days = params.days.unwrap_or(7);
            state
                .analytics_service()
                .get_stats_for_days(tenant_id, days)
                .await?
        }
    };

    Ok(Json(SuccessResponse::new(stats)))
}

/// Parse a date string that may be either ISO 8601 datetime or date-only (YYYY-MM-DD).
/// For date-only start, returns midnight UTC. For date-only end, returns end of day UTC.
fn parse_date_param(s: &str, is_end: bool) -> Option<DateTime<Utc>> {
    // Try full ISO 8601 first
    if let Ok(dt) = s.parse::<DateTime<Utc>>() {
        return Some(dt);
    }
    // Try date-only format (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let time = if is_end {
            date.and_hms_opt(23, 59, 59)?
        } else {
            date.and_hms_opt(0, 0, 0)?
        };
        return Some(time.and_utc());
    }
    None
}

/// Query parameters for stats endpoint
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// Predefined period: "daily", "weekly", or "monthly"
    pub period: Option<String>,
    /// Custom number of days (overrides period)
    pub days: Option<i64>,
    /// Start date in ISO 8601 format (e.g., "2024-01-01T00:00:00Z")
    pub start: Option<String>,
    /// End date in ISO 8601 format (e.g., "2024-01-31T23:59:59Z")
    pub end: Option<String>,
    /// Filter by tenant ID
    pub tenant_id: Option<StringUuid>,
}

/// Query parameters for list_events endpoint
/// Note: pagination fields are inlined because serde_urlencoded (used by axum's Query)
/// does not support #[serde(flatten)].
#[derive(Debug, Deserialize)]
pub struct ListEventsQuery {
    #[serde(default = "default_page", deserialize_with = "deserialize_page")]
    pub page: i64,
    #[serde(
        default = "default_per_page",
        deserialize_with = "deserialize_per_page",
        alias = "limit"
    )]
    pub per_page: i64,
    /// Filter events by email address
    pub email: Option<String>,
    /// Filter events by tenant ID
    pub tenant_id: Option<StringUuid>,
}

/// List login events with pagination
#[utoipa::path(
    get,
    path = "/api/v1/analytics/login-events",
    tag = "Security & Observability",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list_events<S: HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<ListEventsQuery>,
) -> Result<Json<PaginatedResponse<LoginEvent>>, AppError> {
    let (events, total) = if let Some(email) = params.email {
        state
            .analytics_service()
            .list_events_by_email(&email, params.page, params.per_page)
            .await?
    } else if let Some(tenant_id) = params.tenant_id {
        state
            .analytics_service()
            .list_tenant_events(tenant_id, params.page, params.per_page)
            .await?
    } else {
        state
            .analytics_service()
            .list_events(params.page, params.per_page)
            .await?
    };

    Ok(Json(PaginatedResponse::new(
        events,
        params.page,
        params.per_page,
        total,
    )))
}

/// List login events for a specific user
pub async fn list_user_events<S: HasAnalytics>(
    State(state): State<S>,
    Path(user_id): Path<StringUuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<LoginEvent>>, AppError> {
    let (events, total) = state
        .analytics_service()
        .list_user_events(user_id, pagination.page, pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        events,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// List login events for a specific tenant
pub async fn list_tenant_events<S: HasAnalytics>(
    State(state): State<S>,
    Path(tenant_id): Path<StringUuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<LoginEvent>>, AppError> {
    let (events, total) = state
        .analytics_service()
        .list_tenant_events(tenant_id, pagination.page, pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        events,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// Query parameters for daily trend endpoint
#[derive(Debug, Deserialize)]
pub struct DailyTrendQuery {
    /// Number of days to show (default: 7)
    pub days: Option<i64>,
    /// Start date in ISO 8601 format (e.g., "2024-01-01T00:00:00Z")
    pub start: Option<String>,
    /// End date in ISO 8601 format (e.g., "2024-01-31T23:59:59Z")
    pub end: Option<String>,
    /// Filter by tenant ID
    pub tenant_id: Option<StringUuid>,
}

/// Get daily login trend data
#[utoipa::path(
    get,
    path = "/api/v1/analytics/daily-trend",
    tag = "Security & Observability",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_daily_trend<S: HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<DailyTrendQuery>,
) -> Result<Json<SuccessResponse<Vec<DailyTrendPoint>>>, AppError> {
    let tenant_id = params.tenant_id;

    // First check for start/end date parameters (ISO 8601 or YYYY-MM-DD format)
    if let (Some(start), Some(end)) = (&params.start, &params.end) {
        if let (Some(start_dt), Some(end_dt)) =
            (parse_date_param(start, false), parse_date_param(end, true))
        {
            let trend = state
                .analytics_service()
                .get_daily_trend_for_range(tenant_id, start_dt, end_dt)
                .await?;
            return Ok(Json(SuccessResponse::new(trend)));
        }
    }

    // Fallback to days parameter
    let days = params.days.unwrap_or(7);
    let trend = state
        .analytics_service()
        .get_daily_trend(tenant_id, days)
        .await?;
    Ok(Json(SuccessResponse::new(trend)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_query_deserialization() {
        let query: StatsQuery = serde_json::from_str(r#"{"period": "weekly"}"#).unwrap();
        assert_eq!(query.period, Some("weekly".to_string()));
        assert_eq!(query.days, None);
    }

    #[test]
    fn test_stats_query_with_days() {
        let query: StatsQuery = serde_json::from_str(r#"{"days": 30}"#).unwrap();
        assert_eq!(query.days, Some(30));
    }

    #[test]
    fn test_stats_query_with_start_end() {
        let query: StatsQuery = serde_json::from_str(
            r#"{"start": "2024-01-01T00:00:00Z", "end": "2024-01-31T23:59:59Z"}"#,
        )
        .unwrap();
        assert_eq!(query.start, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(query.end, Some("2024-01-31T23:59:59Z".to_string()));
    }

    #[test]
    fn test_parse_date_param_iso8601() {
        let dt = parse_date_param("2024-01-15T10:30:00Z", false);
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2024-01-15 10:30:00");
    }

    #[test]
    fn test_parse_date_param_date_only_start() {
        let dt = parse_date_param("2027-01-01", false);
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2027-01-01 00:00:00");
    }

    #[test]
    fn test_parse_date_param_date_only_end() {
        let dt = parse_date_param("2027-01-31", true);
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2027-01-31 23:59:59");
    }

    #[test]
    fn test_parse_date_param_invalid() {
        assert!(parse_date_param("invalid", false).is_none());
        assert!(parse_date_param("", false).is_none());
        assert!(parse_date_param("2024-13-01", false).is_none());
    }
}
