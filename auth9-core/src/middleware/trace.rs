//! Custom TraceLayer span maker that sanitizes sensitive query parameters.
//!
//! Prevents JWT tokens and other secrets from leaking into application logs
//! by redacting known sensitive query parameter values.

use axum::http::Request;
use tower_http::trace::MakeSpan;
use tracing::Span;

/// Query parameter names whose values must be redacted in logs.
const SENSITIVE_PARAMS: &[&str] = &[
    "id_token_hint",
    "access_token",
    "token",
    "refresh_token",
    "code",
    "client_secret",
    "password",
    "api_key",
];

/// A `MakeSpan` implementation that redacts sensitive query parameters from the
/// logged URI, preventing JWT tokens and credentials from appearing in logs.
#[derive(Clone, Debug)]
pub struct SanitizedMakeSpan;

impl<B> MakeSpan<B> for SanitizedMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let method = request.method();
        let uri = request.uri();
        let sanitized = sanitize_uri(uri);

        tracing::info_span!(
            "request",
            method = %method,
            uri = %sanitized,
            version = ?request.version(),
        )
    }
}

/// Sanitize a URI by redacting the values of sensitive query parameters.
///
/// Example: `/api/v1/auth/logout?client_id=app&id_token_hint=eyJhbG...`
/// becomes: `/api/v1/auth/logout?client_id=app&id_token_hint=[REDACTED]`
fn sanitize_uri(uri: &axum::http::Uri) -> String {
    let query = match uri.query() {
        Some(q) => q,
        None => return uri.path().to_string(),
    };

    let sanitized_pairs: Vec<String> = query
        .split('&')
        .map(|pair| {
            if let Some((key, _value)) = pair.split_once('=') {
                let key_lower = key.to_ascii_lowercase();
                if SENSITIVE_PARAMS.iter().any(|s| key_lower == *s) {
                    format!("{key}=[REDACTED]")
                } else {
                    pair.to_string()
                }
            } else {
                pair.to_string()
            }
        })
        .collect();

    format!("{}?{}", uri.path(), sanitized_pairs.join("&"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Uri;

    #[test]
    fn test_no_query_params() {
        let uri: Uri = "/api/v1/tenants".parse().unwrap();
        assert_eq!(sanitize_uri(&uri), "/api/v1/tenants");
    }

    #[test]
    fn test_no_sensitive_params() {
        let uri: Uri = "/api/v1/tenants?page=1&limit=10".parse().unwrap();
        assert_eq!(sanitize_uri(&uri), "/api/v1/tenants?page=1&limit=10");
    }

    #[test]
    fn test_id_token_hint_redacted() {
        let uri: Uri =
            "/api/v1/auth/logout?client_id=app&id_token_hint=eyJhbGciOiJSUzI1NiJ9.long.token"
                .parse()
                .unwrap();
        assert_eq!(
            sanitize_uri(&uri),
            "/api/v1/auth/logout?client_id=app&id_token_hint=[REDACTED]"
        );
    }

    #[test]
    fn test_access_token_redacted() {
        let uri: Uri = "/api/v1/users?access_token=eyJhbG.secret.token"
            .parse()
            .unwrap();
        assert_eq!(sanitize_uri(&uri), "/api/v1/users?access_token=[REDACTED]");
    }

    #[test]
    fn test_multiple_sensitive_params() {
        let uri: Uri = "/callback?code=abc123&token=xyz&state=ok".parse().unwrap();
        assert_eq!(
            sanitize_uri(&uri),
            "/callback?code=[REDACTED]&token=[REDACTED]&state=ok"
        );
    }
}
