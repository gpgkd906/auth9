//! Security headers middleware for REST API
//!
//! Adds standard security headers to all responses to protect against
//! common web vulnerabilities like XSS, clickjacking, and content sniffing.

use axum::{
    body::Body,
    extract::State,
    http::{header, Request},
    middleware::Next,
    response::Response,
};

use crate::config::SecurityHeadersConfig;

/// Security headers middleware function
///
/// Adds the following security headers to all responses:
/// - X-Content-Type-Options: nosniff
/// - X-Frame-Options: DENY
/// - X-XSS-Protection: 1; mode=block
/// - Referrer-Policy: strict-origin-when-cross-origin
/// - Cache-Control: no-store (for API responses)
/// - Permissions-Policy: geolocation=(), microphone=(), camera=()
pub async fn security_headers_middleware(
    State(config): State<SecurityHeadersConfig>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Decide whether this request should receive HSTS before we move `request` into `next.run`.
    let should_add_hsts = if config.hsts_enabled {
        if !config.hsts_https_only {
            true
        } else if config.hsts_trust_x_forwarded_proto {
            request
                .headers()
                .get("x-forwarded-proto")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.eq_ignore_ascii_case("https"))
                .unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

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

    // HTTP Strict Transport Security (browsers will upgrade HTTP to HTTPS)
    //
    // IMPORTANT: Only emit HSTS for HTTPS responses. Sending it on HTTP (or in local dev)
    // can cause long-lived usability issues in browsers.
    if should_add_hsts {
        let mut value = format!("max-age={}", config.hsts_max_age_secs);
        if config.hsts_include_subdomains {
            value.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            value.push_str("; preload");
        }
        headers.insert(header::STRICT_TRANSPORT_SECURITY, value.parse().unwrap());
    }

    // Content Security Policy for API responses
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'none'; frame-ancestors 'none'"
            .parse()
            .unwrap(),
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
        let cfg = SecurityHeadersConfig {
            hsts_enabled: true,
            hsts_https_only: false, // simplify for this test
            ..SecurityHeadersConfig::default()
        };
        let app = Router::new().route("/test", get(dummy_handler)).layer(
            axum::middleware::from_fn_with_state(cfg, security_headers_middleware),
        );

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
        assert_eq!(
            response.headers().get("Strict-Transport-Security").unwrap(),
            "max-age=31536000; includeSubDomains"
        );
        assert_eq!(
            response.headers().get("Content-Security-Policy").unwrap(),
            "default-src 'none'; frame-ancestors 'none'"
        );
    }

    #[tokio::test]
    async fn test_hsts_not_added_when_disabled() {
        let cfg = SecurityHeadersConfig {
            hsts_enabled: false,
            ..SecurityHeadersConfig::default()
        };
        let app = Router::new().route("/test", get(dummy_handler)).layer(
            axum::middleware::from_fn_with_state(cfg, security_headers_middleware),
        );

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert!(response
            .headers()
            .get("Strict-Transport-Security")
            .is_none());
    }

    #[tokio::test]
    async fn test_hsts_added_only_for_https_via_forwarded_proto() {
        let cfg = SecurityHeadersConfig {
            hsts_enabled: true,
            hsts_https_only: true,
            hsts_trust_x_forwarded_proto: true,
            ..SecurityHeadersConfig::default()
        };
        let app = Router::new().route("/test", get(dummy_handler)).layer(
            axum::middleware::from_fn_with_state(cfg, security_headers_middleware),
        );

        let request = Request::builder()
            .uri("/test")
            .header("x-forwarded-proto", "https")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert!(response
            .headers()
            .get("Strict-Transport-Security")
            .is_some());
    }
}
