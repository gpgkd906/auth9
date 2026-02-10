//! HTTP observability middleware
//!
//! Implemented as a Tower Layer/Service to avoid axum's `from_fn` layer count limits.
//! Combines request ID propagation and metrics recording.

use axum::{body::Body, http::Request, response::Response};
use metrics::{counter, gauge, histogram};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};
use tower::{Layer, Service};
use tracing::Instrument;
use uuid::Uuid;

/// Tower Layer for HTTP observability (request ID + metrics).
#[derive(Clone)]
pub struct ObservabilityLayer;

impl<S> Layer<S> for ObservabilityLayer {
    type Service = ObservabilityMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ObservabilityMiddleware { inner }
    }
}

/// Tower Service that records HTTP metrics and propagates request IDs.
#[derive(Clone)]
pub struct ObservabilityMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for ObservabilityMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let method = request.method().clone().to_string();
        let path = normalize_path(request.uri().path());

        // Extract or generate request ID
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        gauge!("auth9_http_requests_in_flight").increment(1.0);
        let start = Instant::now();

        let mut inner = self.inner.clone();
        let span = tracing::info_span!("request", request_id = %request_id);

        Box::pin(
            async move {
                let response = inner.call(request).await?;

                let duration = start.elapsed().as_secs_f64();
                let status = response.status().as_u16().to_string();

                counter!("auth9_http_requests_total", "method" => method.clone(), "path" => path.clone(), "status" => status)
                    .increment(1);
                histogram!("auth9_http_request_duration_seconds", "method" => method, "path" => path)
                    .record(duration);
                gauge!("auth9_http_requests_in_flight").decrement(1.0);

                // Echo request ID in response headers
                let mut response = response;
                if let Ok(val) = request_id.parse() {
                    response.headers_mut().insert("x-request-id", val);
                }

                Ok(response)
            }
            .instrument(span),
        )
    }
}

/// Collapse UUID-like path segments to `{id}` to prevent high-cardinality labels.
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if looks_like_uuid(seg) {
                "{id}"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn looks_like_uuid(s: &str) -> bool {
    s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_with_uuid() {
        let path = "/api/v1/tenants/550e8400-e29b-41d4-a716-446655440000/users";
        assert_eq!(normalize_path(path), "/api/v1/tenants/{id}/users");
    }

    #[test]
    fn test_normalize_path_without_uuid() {
        let path = "/api/v1/tenants";
        assert_eq!(normalize_path(path), "/api/v1/tenants");
    }

    #[test]
    fn test_normalize_path_multiple_uuids() {
        let path = "/api/v1/users/550e8400-e29b-41d4-a716-446655440000/tenants/6ba7b810-9dad-11d1-80b4-00c04fd430c8";
        assert_eq!(
            normalize_path(path),
            "/api/v1/users/{id}/tenants/{id}"
        );
    }

    #[test]
    fn test_looks_like_uuid() {
        assert!(looks_like_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!looks_like_uuid("tenants"));
        assert!(!looks_like_uuid("v1"));
        assert!(!looks_like_uuid(""));
    }
}
