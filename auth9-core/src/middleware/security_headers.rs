//! Security headers middleware for REST API
//!
//! Adds standard security headers to all responses to protect against
//! common web vulnerabilities like XSS, clickjacking, and content sniffing.

use axum::{
    body::Body,
    http::{header, Request},
    middleware::Next,
    response::Response,
};

/// Security headers middleware function
///
/// Adds the following security headers to all responses:
/// - X-Content-Type-Options: nosniff
/// - X-Frame-Options: DENY
/// - X-XSS-Protection: 1; mode=block
/// - Referrer-Policy: strict-origin-when-cross-origin
/// - Cache-Control: no-store (for API responses)
/// - Permissions-Policy: geolocation=(), microphone=(), camera=()
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());

    // Prevent clickjacking
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());

    // XSS protection (legacy but still useful for older browsers)
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());

    // Control referrer information
    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // Prevent caching of sensitive API responses
    headers.insert(
        header::CACHE_CONTROL,
        "no-store, no-cache, must-revalidate, private"
            .parse()
            .unwrap(),
    );

    // Restrict browser features
    headers.insert(
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=()".parse().unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn dummy_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_security_headers_are_added() {
        let app = Router::new()
            .route("/test", get(dummy_handler))
            .layer(axum::middleware::from_fn(security_headers_middleware));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check security headers
        assert_eq!(
            response.headers().get("X-Content-Type-Options").unwrap(),
            "nosniff"
        );
        assert_eq!(response.headers().get("X-Frame-Options").unwrap(), "DENY");
        assert_eq!(
            response.headers().get("X-XSS-Protection").unwrap(),
            "1; mode=block"
        );
        assert_eq!(
            response.headers().get("Referrer-Policy").unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert!(response
            .headers()
            .get("Cache-Control")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("no-store"));
        assert!(response.headers().get("Permissions-Policy").is_some());
    }
}
