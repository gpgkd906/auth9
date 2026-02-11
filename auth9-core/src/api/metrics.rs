//! Prometheus /metrics endpoint

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;

/// GET /metrics — returns Prometheus text exposition format.
pub async fn metrics_handler(
    State(handle): State<Arc<Option<PrometheusHandle>>>,
) -> impl IntoResponse {
    match handle.as_ref() {
        Some(h) => (StatusCode::OK, h.render()),
        None => (StatusCode::NOT_FOUND, "Metrics not enabled".to_string()),
    }
}
