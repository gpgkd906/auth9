use crate::domains::security_observability::api as secobs_api;
use crate::domains::security_observability::context::SecurityObservabilityContext;
use crate::state::HasServices;
use axum::{
    routing::{get, post},
    Router,
};

pub fn public_routes<S>() -> Router<S>
where
    S: HasServices,
{
    Router::new()
        .route("/health", get(secobs_api::health::health))
        .route("/ready", get(secobs_api::health::ready::<S>))
}

pub fn protected_routes<S>() -> Router<S>
where
    S: SecurityObservabilityContext,
{
    Router::new()
        .route("/api/v1/audit-logs", get(secobs_api::audit::list::<S>))
        .route(
            "/api/v1/analytics/login-stats",
            get(secobs_api::analytics::get_stats::<S>),
        )
        .route(
            "/api/v1/analytics/login-events",
            get(secobs_api::analytics::list_events::<S>),
        )
        .route(
            "/api/v1/analytics/daily-trend",
            get(secobs_api::analytics::get_daily_trend::<S>),
        )
        .route(
            "/api/v1/security/alerts",
            get(secobs_api::security_alert::list_alerts::<S>),
        )
        .route(
            "/api/v1/security/alerts/{id}/resolve",
            post(secobs_api::security_alert::resolve_alert::<S>),
        )
}
