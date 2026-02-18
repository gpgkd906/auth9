//! Authentication enforcement middleware for REST API
//!
//! This middleware ensures that protected routes require valid JWT authentication.
//! It validates the Bearer token in the Authorization header and rejects
//! requests without valid tokens.

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::cache::CacheOperations;
use crate::jwt::JwtManager;
use std::sync::Arc;

/// Shared state for authentication middleware
#[derive(Clone)]
pub struct AuthMiddlewareState {
    jwt_manager: JwtManager,
    cache: Option<Arc<dyn CacheOperations>>,
    tenant_access_allowed_audiences: Vec<String>,
    is_production: bool,
}

impl AuthMiddlewareState {
    pub fn new(
        jwt_manager: JwtManager,
        tenant_access_allowed_audiences: Vec<String>,
        is_production: bool,
    ) -> Self {
        Self {
            jwt_manager,
            cache: None,
            tenant_access_allowed_audiences,
            is_production,
        }
    }

    pub fn with_cache(mut self, cache: Arc<dyn CacheOperations>) -> Self {
        self.cache = Some(cache);
        self
    }
}

/// Authentication enforcement middleware
///
/// This middleware validates JWT tokens on protected routes.
/// Requests without valid tokens are rejected with 401 Unauthorized.
///
/// The middleware checks for:
/// - Presence of Authorization header
/// - Bearer token scheme
/// - Valid JWT signature and claims
pub async fn require_auth_middleware(
    State(auth_state): State<AuthMiddlewareState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let request_path = request.uri().path().to_string();
    // Extract the Authorization header
    let auth_header = match request.headers().get(AUTHORIZATION) {
        Some(header) => header,
        None => {
            return unauthorized_response("Missing authorization token");
        }
    };

    // Parse the header value
    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => {
            return unauthorized_response("Invalid authorization header encoding");
        }
    };

    // Check for Bearer scheme
    let token = match auth_str.strip_prefix("Bearer ") {
        Some(t) => t,
        None => {
            return unauthorized_response("Authorization header must use Bearer scheme");
        }
    };

    // Validate the token (service client, identity, then tenant access token)
    // Also extract session ID for blacklist check.
    let mut session_id: Option<String> = None;
    let token_kind = if let Ok(claims) = auth_state.jwt_manager.verify_service_client_token(token) {
        session_id = Some(claims.sub.clone());
        Some("service_client")
    } else if let Ok(claims) = auth_state.jwt_manager.verify_identity_token(token) {
        session_id = claims.sid.clone();
        Some("identity")
    } else if !auth_state.tenant_access_allowed_audiences.is_empty() {
        if let Ok(claims) = auth_state
            .jwt_manager
            .verify_tenant_access_token_strict(token, &auth_state.tenant_access_allowed_audiences)
        {
            session_id = Some(claims.sub.clone());
            Some("tenant_access")
        } else {
            None
        }
    } else if auth_state.is_production {
        None
    } else if let Ok(claims) = {
        #[allow(deprecated)]
        auth_state
            .jwt_manager
            .verify_tenant_access_token(token, None)
    } {
        session_id = Some(claims.sub.clone());
        Some("tenant_access")
    } else {
        None
    };

    let Some(token_kind) = token_kind else {
        return unauthorized_response("Invalid or expired token");
    };

    if token_kind == "identity" && !is_identity_token_path_allowed(&request_path) {
        return forbidden_response(
            "Identity token is only allowed for tenant selection and exchange",
        );
    }

    // Check token blacklist (e.g., after logout)
    // Fail-Closed: if Redis is unavailable, reject the request with 503 to prevent
    // revoked tokens from being used during cache outages.
    if let (Some(ref cache), Some(ref sid)) = (&auth_state.cache, &session_id) {
        match cache.is_token_blacklisted(sid).await {
            Ok(true) => {
                return unauthorized_response("Token has been revoked");
            }
            Ok(false) => {}
            Err(_) => {
                // One quick retry before failing
                match cache.is_token_blacklisted(sid).await {
                    Ok(true) => {
                        return unauthorized_response("Token has been revoked");
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::error!(error = %e, "Token blacklist check failed after retry, rejecting request (fail-closed)");
                        return service_unavailable_response(
                            "Authentication service temporarily unavailable",
                        );
                    }
                }
            }
        }
    }

    // Token is valid, proceed with the request
    next.run(request).await
}

/// Generate a 401 Unauthorized response
fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": message,
            "code": "UNAUTHORIZED"
        })),
    )
        .into_response()
}

fn forbidden_response(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": message,
            "code": "FORBIDDEN"
        })),
    )
        .into_response()
}

fn is_identity_token_path_allowed(path: &str) -> bool {
    path.starts_with("/api/v1/auth/")
        || path == "/api/v1/users/me/tenants"
        || path == "/api/v1/organizations"
        || path == "/api/v1/users/me"
        || path.starts_with("/api/v1/users/me/sessions")
        || path.starts_with("/api/v1/users/me/passkeys")
}

/// Generate a 503 Service Unavailable response
///
/// Used when a critical backing service (e.g. Redis for token blacklist) is down.
/// Returns 503 instead of 401 so clients don't discard their (potentially valid) tokens.
fn service_unavailable_response(message: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({
            "error": message,
            "code": "SERVICE_UNAVAILABLE"
        })),
    )
        .into_response()
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

    use crate::config::JwtConfig;

    async fn protected_handler() -> &'static str {
        "Protected content"
    }

    fn create_test_jwt_manager() -> JwtManager {
        let config = JwtConfig {
            secret: "test-secret-key-for-jwt-signing-must-be-long".to_string(),
            issuer: "https://auth9.test".to_string(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 86400,
            private_key_pem: None,
            public_key_pem: None,
            previous_public_key_pem: None,
        };
        JwtManager::new(config)
    }

    #[tokio::test]
    async fn test_missing_auth_header_returns_401() {
        let jwt_manager = create_test_jwt_manager();
        let auth_state = AuthMiddlewareState::new(jwt_manager, vec![], false);

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_bearer_scheme_returns_401() {
        let jwt_manager = create_test_jwt_manager();
        let auth_state = AuthMiddlewareState::new(jwt_manager, vec![], false);

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .header("Authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_token_returns_401() {
        let jwt_manager = create_test_jwt_manager();
        let auth_state = AuthMiddlewareState::new(jwt_manager, vec![], false);

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .header("Authorization", "Bearer invalid.token.here")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_valid_token_allows_request() {
        let jwt_manager = create_test_jwt_manager();

        // Generate a valid identity token
        let user_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let valid_token = jwt_manager
            .create_identity_token(user_id, "test@example.com", Some("Test User"))
            .unwrap();

        let auth_state = AuthMiddlewareState::new(jwt_manager, vec![], false);

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .header("Authorization", format!("Bearer {}", valid_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_blacklist_redis_error_returns_503_fail_closed() {
        use crate::cache::MockCacheOperations;

        let jwt_manager = create_test_jwt_manager();

        let user_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let session_id = uuid::Uuid::parse_str("660e8400-e29b-41d4-a716-446655440001").unwrap();
        let valid_token = jwt_manager
            .create_identity_token_with_session(
                user_id,
                "test@example.com",
                Some("Test User"),
                Some(session_id),
            )
            .unwrap();

        let mut mock_cache = MockCacheOperations::new();
        // Both the initial check and the retry return errors
        mock_cache
            .expect_is_token_blacklisted()
            .times(2)
            .returning(|_| Err(anyhow::anyhow!("Redis connection refused").into()));

        let auth_state =
            AuthMiddlewareState::new(jwt_manager, vec![], false).with_cache(Arc::new(mock_cache));

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .header("Authorization", format!("Bearer {}", valid_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Fail-closed: should return 503, NOT 200
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_blacklist_redis_error_retry_succeeds() {
        use crate::cache::MockCacheOperations;
        use std::sync::atomic::{AtomicU32, Ordering};

        let jwt_manager = create_test_jwt_manager();

        let user_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let session_id = uuid::Uuid::parse_str("660e8400-e29b-41d4-a716-446655440001").unwrap();
        let valid_token = jwt_manager
            .create_identity_token_with_session(
                user_id,
                "test@example.com",
                Some("Test User"),
                Some(session_id),
            )
            .unwrap();

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let mut mock_cache = MockCacheOperations::new();
        // First call fails, second call (retry) succeeds
        mock_cache
            .expect_is_token_blacklisted()
            .times(2)
            .returning(move |_| {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    Err(anyhow::anyhow!("Redis connection refused").into())
                } else {
                    Ok(false)
                }
            });

        let auth_state =
            AuthMiddlewareState::new(jwt_manager, vec![], false).with_cache(Arc::new(mock_cache));

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .header("Authorization", format!("Bearer {}", valid_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Retry succeeded, so request should pass through
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_blacklisted_token_returns_401() {
        use crate::cache::MockCacheOperations;

        let jwt_manager = create_test_jwt_manager();

        let user_id = uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let session_id = uuid::Uuid::parse_str("660e8400-e29b-41d4-a716-446655440001").unwrap();
        let valid_token = jwt_manager
            .create_identity_token_with_session(
                user_id,
                "test@example.com",
                Some("Test User"),
                Some(session_id),
            )
            .unwrap();

        let mut mock_cache = MockCacheOperations::new();
        mock_cache
            .expect_is_token_blacklisted()
            .returning(|_| Ok(true));

        let auth_state =
            AuthMiddlewareState::new(jwt_manager, vec![], false).with_cache(Arc::new(mock_cache));

        let app = Router::new()
            .route("/api/v1/auth/userinfo", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/api/v1/auth/userinfo")
            .header("Authorization", format!("Bearer {}", valid_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
