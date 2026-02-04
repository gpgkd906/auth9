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

use crate::jwt::JwtManager;

/// Shared state for authentication middleware
#[derive(Clone)]
pub struct AuthMiddlewareState {
    jwt_manager: JwtManager,
}

impl AuthMiddlewareState {
    pub fn new(jwt_manager: JwtManager) -> Self {
        Self { jwt_manager }
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

    // Validate the token (try identity token first, then tenant access token)
    let is_valid = auth_state.jwt_manager.verify_identity_token(token).is_ok()
        || auth_state
            .jwt_manager
            .verify_tenant_access_token(token, None)
            .is_ok();

    if !is_valid {
        return unauthorized_response("Invalid or expired token");
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
        };
        JwtManager::new(config)
    }

    #[tokio::test]
    async fn test_missing_auth_header_returns_401() {
        let jwt_manager = create_test_jwt_manager();
        let auth_state = AuthMiddlewareState::new(jwt_manager);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_bearer_scheme_returns_401() {
        let jwt_manager = create_test_jwt_manager();
        let auth_state = AuthMiddlewareState::new(jwt_manager);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/protected")
            .header("Authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_invalid_token_returns_401() {
        let jwt_manager = create_test_jwt_manager();
        let auth_state = AuthMiddlewareState::new(jwt_manager);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/protected")
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
            .create_identity_token(
                user_id,
                "test@example.com",
                Some("Test User"),
            )
            .unwrap();

        let auth_state = AuthMiddlewareState::new(jwt_manager);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth_state,
                require_auth_middleware,
            ));

        let request = Request::builder()
            .uri("/protected")
            .header("Authorization", format!("Bearer {}", valid_token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
