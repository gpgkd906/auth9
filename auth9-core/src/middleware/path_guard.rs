//! Path traversal guard middleware
//!
//! Rejects requests containing path traversal sequences (`..`) or bare dot
//! segments (`.`) to prevent attackers from manipulating URL paths to access
//! unintended endpoints or cause unexpected query behaviour.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// Returns `true` if any path segment is `.` or `..`.
fn has_dot_segments(path: &str) -> bool {
    path.split('/').any(|seg| seg == "." || seg == "..")
}

/// Middleware that rejects requests with path traversal sequences in the URI.
pub async fn path_guard_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    if has_dot_segments(path) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_dot_segments() {
        assert!(has_dot_segments("/api/v1/tenants/../users"));
        assert!(has_dot_segments("/api/v1/users/././some-id"));
        assert!(has_dot_segments("/api/./v1/users"));
        assert!(!has_dot_segments("/api/v1/tenants/123/users"));
        assert!(!has_dot_segments("/api/v1/users"));
        assert!(!has_dot_segments("/api/v1/users/some.file.txt")); // dots within segment are fine
    }
}
