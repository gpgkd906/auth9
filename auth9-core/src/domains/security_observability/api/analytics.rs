//! Analytics API handlers

use crate::api::{
    default_page, default_per_page, deserialize_page, deserialize_per_page, PaginatedResponse,
    PaginationQuery, SuccessResponse,
};
use crate::domain::{DailyTrendPoint, LoginEvent, LoginStats, StringUuid};
use crate::error::AppError;
use crate::state::HasAnalytics;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Get login statistics
pub async fn get_stats<S: HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<SuccessResponse<LoginStats>>, AppError> {
    // First check for start/end date parameters (ISO 8601 format)
    if let (Some(start), Some(end)) = (&params.start, &params.end) {
        if let (Ok(start_dt), Ok(end_dt)) =
            (start.parse::<DateTime<Utc>>(), end.parse::<DateTime<Utc>>())
        {
            let stats = state
                .analytics_service()
                .get_stats_for_range(start_dt, end_dt)
                .await?;
            return Ok(Json(SuccessResponse::new(stats)));
        }
    }

    // Fallback to period/days parameters
    let stats = match params.period.as_deref() {
        Some("daily") | Some("day") => state.analytics_service().get_daily_stats().await?,
        Some("weekly") | Some("week") => state.analytics_service().get_weekly_stats().await?,
        Some("monthly") | Some("month") => state.analytics_service().get_monthly_stats().await?,
        _ => {
            // Default to weekly
            let days = params.days.unwrap_or(7);
            state.analytics_service().get_stats_for_days(days).await?
        }
    };

    Ok(Json(SuccessResponse::new(stats)))
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
}

/// Get daily login trend data
pub async fn get_daily_trend<S: HasAnalytics>(
    State(state): State<S>,
    Query(params): Query<DailyTrendQuery>,
) -> Result<Json<SuccessResponse<Vec<DailyTrendPoint>>>, AppError> {
    // First check for start/end date parameters
    if let (Some(start), Some(end)) = (&params.start, &params.end) {
        if let (Ok(start_dt), Ok(end_dt)) =
            (start.parse::<DateTime<Utc>>(), end.parse::<DateTime<Utc>>())
        {
            let trend = state
                .analytics_service()
                .get_daily_trend_for_range(start_dt, end_dt)
                .await?;
            return Ok(Json(SuccessResponse::new(trend)));
        }
    }

    // Fallback to days parameter
    let days = params.days.unwrap_or(7);
    let trend = state.analytics_service().get_daily_trend(days).await?;
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
}
