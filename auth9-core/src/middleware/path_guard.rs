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

/// URL-decode percent-encoded bytes in a path string.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                hex_val(bytes[i + 1]),
                hex_val(bytes[i + 2]),
            ) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Middleware that rejects requests with path traversal sequences in the URI.
///
/// Checks both the raw path and the percent-decoded path to block encoded
/// traversal attempts like `%2e%2e`.
pub async fn path_guard_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();
    if has_dot_segments(path) {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Also check percent-decoded path to catch encoded traversal (%2e%2e, %2E%2E)
    let decoded = percent_decode(path);
    if decoded != path && has_dot_segments(&decoded) {
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

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("/api/v1/%2e%2e/admin"), "/api/v1/../admin");
        assert_eq!(percent_decode("/api/v1/%2E%2E/admin"), "/api/v1/../admin");
        assert_eq!(percent_decode("/normal/path"), "/normal/path");
        assert_eq!(percent_decode("/api/%2e/%2e%2e"), "/api/./..");
    }

    #[test]
    fn test_encoded_dot_segments_detected() {
        // Encoded ".." decodes to ".." which has_dot_segments catches
        let decoded = percent_decode("/api/v1/users/%2e%2e/admin");
        assert!(has_dot_segments(&decoded));

        // Mixed-case encoding
        let decoded = percent_decode("/api/v1/%2E%2E/api/v1/tenants");
        assert!(has_dot_segments(&decoded));

        // Encoded single dot
        let decoded = percent_decode("/api/%2e/v1/users");
        assert!(has_dot_segments(&decoded));

        // Normal path stays clean
        let decoded = percent_decode("/api/v1/users/123");
        assert!(!has_dot_segments(&decoded));
    }
}
