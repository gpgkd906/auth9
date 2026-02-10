//! Telemetry initialization: metrics, tracing, and structured logging

pub mod metrics;
pub mod tracing_setup;

use crate::config::TelemetryConfig;
use metrics_exporter_prometheus::PrometheusHandle;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialise the full telemetry stack.
///
/// Returns `Some(PrometheusHandle)` when metrics are enabled so the HTTP
/// server can expose a `/metrics` endpoint.
pub fn init(config: &TelemetryConfig) -> Option<PrometheusHandle> {
    // 1. Build the env filter (same logic as the old init_tracing)
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "auth9_core=info,tower_http=debug".into());

    // 2. Optionally install Prometheus recorder
    let prometheus_handle = if config.metrics_enabled {
        let handle = metrics::install_prometheus_recorder();
        metrics::describe_metrics();
        Some(handle)
    } else {
        None
    };

    // 3. Build the subscriber with conditional layers
    let registry = tracing_subscriber::registry().with(env_filter);

    let is_json = config.log_format == "json";

    // We need to build the full subscriber in one go because the OpenTelemetry
    // layer's `S` type parameter must match the actual composed subscriber.
    if is_json {
        // By default, tracing-subscriber nests event fields under `fields`.
        // For a more conventional JSON log shape (and to align with our QA docs),
        // flatten event fields so `message` is consistently top-level.
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true);
        let otel_layer = tracing_setup::create_otel_layer(config);
        registry.with(fmt_layer).with(otel_layer).init();
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer();
        let otel_layer = tracing_setup::create_otel_layer(config);
        registry.with(fmt_layer).with(otel_layer).init();
    }

    prometheus_handle
}
