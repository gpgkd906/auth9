//! OpenTelemetry tracing setup

use crate::config::TelemetryConfig;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime::Tokio;
use tracing_opentelemetry::OpenTelemetryLayer;

/// Create an OpenTelemetry tracing layer if tracing is enabled.
pub fn create_otel_layer<S>(
    config: &TelemetryConfig,
) -> Option<OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    if !config.tracing_enabled {
        return None;
    }

    let endpoint = match &config.otlp_endpoint {
        Some(ep) => ep.clone(),
        None => {
            eprintln!("WARN: OTEL_TRACING_ENABLED=true but OTEL_EXPORTER_OTLP_ENDPOINT not set, skipping");
            return None;
        }
    };

    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
    {
        Ok(e) => e,
        Err(err) => {
            eprintln!("ERROR: Failed to create OTLP exporter: {}", err);
            return None;
        }
    };

    let resource = opentelemetry_sdk::Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
    ]);

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, Tokio)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(config.service_name.clone());

    // Set the global provider so it can be shut down later
    opentelemetry::global::set_tracer_provider(provider);

    Some(tracing_opentelemetry::layer().with_tracer(tracer))
}
