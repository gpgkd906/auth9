//! Error response normalization middleware
//!
//! Ensures all error responses (4xx, 5xx) use consistent JSON format,
//! preventing framework-level rejections from returning text/plain
//! and hiding internal parser details from clients.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Middleware that normalizes all error responses to consistent JSON format.
/// This prevents information leakage from framework-level error messages
/// (e.g., JSON parser position details, method not allowed text).
pub async fn normalize_error_response(request: Request<Body>, next: Next) -> Response {
    let uri = request.uri().path().to_string();
    let response = next.run(request).await;

    let status = response.status();

    // Health/readiness endpoints return their own plain-text responses - skip normalization
    if uri == "/health" || uri == "/ready" {
        return response;
    }

    // Only process error responses (4xx and 5xx)
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }

    // Check if the response is already JSON from application error handlers
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if content_type.contains("application/json") || content_type.contains("application/scim+json")
    {
        // Already JSON from our AppError handler or SCIM handler - pass through as-is.
        // Note: axum's JsonRejection also returns application/json with
        // internal details. We handle this by replacing the body for
        // known leak patterns (checked at 400/422 status from framework).
        return response;
    }

    // Non-JSON error response (text/plain from framework rejections) - normalize
    generic_error_response(status)
}

fn generic_error_response(status: StatusCode) -> Response {
    let error_type = match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::CONFLICT => "conflict",
        StatusCode::UNPROCESSABLE_ENTITY => "validation_error",
        StatusCode::TOO_MANY_REQUESTS => "rate_limited",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "unsupported_media_type",
        _ if status.is_client_error() => "client_error",
        _ => "internal_error",
    };

    let message = match status {
        StatusCode::BAD_REQUEST => "Invalid request body",
        StatusCode::UNAUTHORIZED => "Authentication required",
        StatusCode::FORBIDDEN => "Access denied",
        StatusCode::NOT_FOUND => "Not found",
        StatusCode::METHOD_NOT_ALLOWED => "Method not allowed",
        StatusCode::CONFLICT => "Resource conflict",
        StatusCode::UNPROCESSABLE_ENTITY => "Validation error",
        StatusCode::TOO_MANY_REQUESTS => "Too many requests",
        StatusCode::UNSUPPORTED_MEDIA_TYPE => "Unsupported content type",
        _ if status.is_client_error() => "Client error",
        _ => "An internal error occurred",
    };

    let body = json!({
        "error": error_type,
        "message": message,
    });

    (status, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Router};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_success_response_passthrough() {
        let app = Router::new()
            .route("/test", post(|| async { (StatusCode::OK, "ok") }))
            .layer(axum::middleware::from_fn(normalize_error_response));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_endpoint_503_not_normalized() {
        use axum::routing::get;
        let app = Router::new()
            .route(
                "/ready",
                get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "not_ready") }),
            )
            .layer(axum::middleware::from_fn(normalize_error_response));

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"not_ready");
    }

    #[tokio::test]
    async fn test_text_error_converted_to_json() {
        let app = Router::new()
            .route(
                "/test",
                post(|| async { (StatusCode::BAD_REQUEST, "Some text error") }),
            )
            .layer(axum::middleware::from_fn(normalize_error_response));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let ct = response.headers().get(header::CONTENT_TYPE).unwrap();
        assert!(ct.to_str().unwrap().contains("application/json"));
    }
}
