//! JWT Authentication middleware and extractors
//!
//! Provides:
//! - `AuthUser` extractor for handlers requiring authenticated users
//! - `RequireAuth` middleware layer for protecting routes

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::jwt::{IdentityClaims, TenantAccessClaims};
use crate::state::HasServices;

/// Authenticated user information extracted from JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    /// User ID from the token's `sub` claim
    pub user_id: Uuid,
    /// User's email address
    pub email: String,
    /// Token type: either Identity or TenantAccess
    pub token_type: TokenType,
    /// Tenant ID (only present for TenantAccess tokens)
    pub tenant_id: Option<Uuid>,
    /// Roles (only present for TenantAccess tokens)
    pub roles: Vec<String>,
    /// Permissions (only present for TenantAccess tokens)
    pub permissions: Vec<String>,
}

/// Type of JWT token used for authentication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenType {
    /// Identity token (issued after initial authentication)
    Identity,
    /// Tenant access token (issued after token exchange)
    TenantAccess,
}

impl AuthUser {
    /// Create AuthUser from identity token claims
    pub fn from_identity_claims(claims: IdentityClaims) -> Result<Self, AuthError> {
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AuthError::InvalidToken("Invalid user ID in token".to_string()))?;

        Ok(Self {
            user_id,
            email: claims.email,
            token_type: TokenType::Identity,
            tenant_id: None,
            roles: vec![],
            permissions: vec![],
        })
    }

    /// Create AuthUser from tenant access token claims
    pub fn from_tenant_access_claims(claims: TenantAccessClaims) -> Result<Self, AuthError> {
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AuthError::InvalidToken("Invalid user ID in token".to_string()))?;

        let tenant_id = Uuid::parse_str(&claims.tenant_id)
            .map_err(|_| AuthError::InvalidToken("Invalid tenant ID in token".to_string()))?;

        Ok(Self {
            user_id,
            email: claims.email,
            token_type: TokenType::TenantAccess,
            tenant_id: Some(tenant_id),
            roles: claims.roles,
            permissions: claims.permissions,
        })
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|p| self.has_permission(p))
    }
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    /// No Authorization header present
    MissingToken,
    /// Invalid Authorization header format
    InvalidHeader(String),
    /// Token validation failed
    InvalidToken(String),
    /// Token has expired
    TokenExpired,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidHeader(_) => (StatusCode::UNAUTHORIZED, "Invalid authorization header"),
            AuthError::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AuthError::TokenExpired => (StatusCode::UNAUTHORIZED, "Token has expired"),
        };

        let body = serde_json::json!({
            "error": message,
            "code": "UNAUTHORIZED"
        });

        (status, axum::Json(body)).into_response()
    }
}

/// Extract and validate Bearer token from Authorization header
fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Result<&str, AuthError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingToken)?
        .to_str()
        .map_err(|_| AuthError::InvalidHeader("Invalid header encoding".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AuthError::InvalidHeader(
            "Authorization header must use Bearer scheme".to_string(),
        ));
    }

    Ok(&auth_header[7..])
}

/// Axum extractor for authenticated users
///
/// This extractor validates the JWT token from the Authorization header
/// and provides the authenticated user information to handlers.
///
/// # Example
///
/// ```ignore
/// async fn protected_handler(
///     auth: AuthUser,
///     State(state): State<AppState>,
/// ) -> impl IntoResponse {
///     format!("Hello, {}!", auth.email)
/// }
/// ```
impl<S> FromRequestParts<S> for AuthUser
where
    S: HasServices + Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_bearer_token(&parts.headers)?;
        let jwt_manager = state.jwt_manager();

        // Try to validate as identity token first
        if let Ok(claims) = jwt_manager.verify_identity_token(token) {
            return AuthUser::from_identity_claims(claims);
        }

        // Try to validate as tenant access token
        if let Ok(claims) = jwt_manager.verify_tenant_access_token(token, None) {
            return AuthUser::from_tenant_access_claims(claims);
        }

        Err(AuthError::InvalidToken(
            "Token validation failed".to_string(),
        ))
    }
}

/// Optional authentication extractor
///
/// Returns `Some(AuthUser)` if a valid token is present, `None` otherwise.
/// Useful for endpoints that have optional authentication.
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: HasServices + Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

/// Middleware layer that requires authentication on all routes
///
/// This is a simple marker type that can be used with axum's layer system
/// to require authentication on a group of routes.
pub struct RequireAuth;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_from_identity_claims() {
        let claims = IdentityClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            iss: "https://auth9.test".to_string(),
            aud: "auth9".to_string(),
            iat: 1000000,
            exp: 1003600,
        };

        let user = AuthUser::from_identity_claims(claims).unwrap();

        assert_eq!(
            user.user_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.token_type, TokenType::Identity);
        assert!(user.tenant_id.is_none());
        assert!(user.roles.is_empty());
        assert!(user.permissions.is_empty());
    }

    #[test]
    fn test_auth_user_from_tenant_access_claims() {
        let claims = TenantAccessClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            email: "test@example.com".to_string(),
            iss: "https://auth9.test".to_string(),
            aud: "my-service".to_string(),
            tenant_id: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            roles: vec!["admin".to_string(), "user".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            iat: 1000000,
            exp: 1003600,
        };

        let user = AuthUser::from_tenant_access_claims(claims).unwrap();

        assert_eq!(
            user.user_id,
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
        );
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.token_type, TokenType::TenantAccess);
        assert_eq!(
            user.tenant_id,
            Some(Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap())
        );
        assert_eq!(user.roles, vec!["admin", "user"]);
        assert_eq!(user.permissions, vec!["read", "write"]);
    }

    #[test]
    fn test_auth_user_invalid_user_id() {
        let claims = IdentityClaims {
            sub: "not-a-uuid".to_string(),
            email: "test@example.com".to_string(),
            name: None,
            iss: "https://auth9.test".to_string(),
            aud: "auth9".to_string(),
            iat: 1000000,
            exp: 1003600,
        };

        let result = AuthUser::from_identity_claims(claims);
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_user_has_permission() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(Uuid::new_v4()),
            roles: vec!["admin".to_string()],
            permissions: vec!["user:read".to_string(), "user:write".to_string()],
        };

        assert!(user.has_permission("user:read"));
        assert!(user.has_permission("user:write"));
        assert!(!user.has_permission("user:delete"));
    }

    #[test]
    fn test_auth_user_has_role() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(Uuid::new_v4()),
            roles: vec!["admin".to_string(), "user".to_string()],
            permissions: vec![],
        };

        assert!(user.has_role("admin"));
        assert!(user.has_role("user"));
        assert!(!user.has_role("superadmin"));
    }

    #[test]
    fn test_auth_user_has_any_permission() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(Uuid::new_v4()),
            roles: vec![],
            permissions: vec!["user:read".to_string()],
        };

        assert!(user.has_any_permission(&["user:read", "user:write"]));
        assert!(!user.has_any_permission(&["user:delete", "user:admin"]));
    }

    #[test]
    fn test_auth_user_has_all_permissions() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(Uuid::new_v4()),
            roles: vec![],
            permissions: vec!["user:read".to_string(), "user:write".to_string()],
        };

        assert!(user.has_all_permissions(&["user:read", "user:write"]));
        assert!(!user.has_all_permissions(&["user:read", "user:delete"]));
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            "Bearer test-token-123".parse().unwrap(),
        );

        let token = extract_bearer_token(&headers).unwrap();
        assert_eq!(token, "test-token-123");
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let headers = axum::http::HeaderMap::new();
        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(AuthError::MissingToken)));
    }

    #[test]
    fn test_extract_bearer_token_wrong_scheme() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(AUTHORIZATION, "Basic dXNlcjpwYXNz".parse().unwrap());

        let result = extract_bearer_token(&headers);
        assert!(matches!(result, Err(AuthError::InvalidHeader(_))));
    }

    #[test]
    fn test_auth_error_into_response() {
        let errors = vec![
            AuthError::MissingToken,
            AuthError::InvalidHeader("test".to_string()),
            AuthError::InvalidToken("test".to_string()),
            AuthError::TokenExpired,
        ];

        for error in errors {
            let response = error.into_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[test]
    fn test_token_type_equality() {
        assert_eq!(TokenType::Identity, TokenType::Identity);
        assert_eq!(TokenType::TenantAccess, TokenType::TenantAccess);
        assert_ne!(TokenType::Identity, TokenType::TenantAccess);
    }

    #[test]
    fn test_auth_user_clone() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token_type: TokenType::Identity,
            tenant_id: None,
            roles: vec![],
            permissions: vec![],
        };

        let cloned = user.clone();
        assert_eq!(user.user_id, cloned.user_id);
        assert_eq!(user.email, cloned.email);
    }

    #[test]
    fn test_auth_user_debug() {
        let user = AuthUser {
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            token_type: TokenType::Identity,
            tenant_id: None,
            roles: vec![],
            permissions: vec![],
        };

        let debug_str = format!("{:?}", user);
        assert!(debug_str.contains("AuthUser"));
        assert!(debug_str.contains("test@example.com"));
    }
}
