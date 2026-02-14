//! Prometheus /metrics endpoint

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;

/// Shared state for the metrics endpoint
#[derive(Clone)]
pub struct MetricsState {
    pub handle: Arc<Option<PrometheusHandle>>,
    /// When set, requests must include `Authorization: Bearer <token>`
    pub required_token: Option<String>,
}

/// GET /metrics — returns Prometheus text exposition format.
///
/// When `METRICS_TOKEN` is configured, requests must include a matching
/// `Authorization: Bearer <token>` header. This prevents information
/// disclosure of internal system metrics in production.
pub async fn metrics_handler(
    State(state): State<MetricsState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Check bearer token if configured
    if let Some(ref expected) = state.required_token {
        let authorized = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|token| token == expected.as_str())
            .unwrap_or(false);

        if !authorized {
            return (StatusCode::NOT_FOUND, "Not Found".to_string());
        }
    }

    match state.handle.as_ref() {
        Some(h) => (StatusCode::OK, h.render()),
        None => (StatusCode::NOT_FOUND, "Metrics not enabled".to_string()),
    }
}
