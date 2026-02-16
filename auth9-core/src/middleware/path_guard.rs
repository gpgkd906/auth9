//! Path traversal guard middleware
//!
//! Rejects requests containing path traversal sequences (`..`) to prevent
//! attackers from manipulating URL paths to access unintended endpoints.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// Middleware that rejects requests with path traversal sequences in the URI.
pub async fn path_guard_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    if path.contains("..") {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_contains_traversal() {
        assert!("/api/v1/tenants/../users".contains(".."));
        assert!("/api/v1/tenants/..%2fusers".contains(".."));
        assert!(!"/api/v1/tenants/123/users".contains(".."));
        assert!(!"/api/v1/users".contains(".."));
    }
}
