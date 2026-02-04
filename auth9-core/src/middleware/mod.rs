//! HTTP middleware for Auth9 Core
//!
//! This module provides middleware components for the REST API:
//! - JWT authentication middleware and AuthUser extractor
//! - Rate limiting middleware
//! - Security headers middleware
//! - Authentication enforcement middleware

pub mod auth;
pub mod rate_limit;
pub mod require_auth;
pub mod security_headers;

pub use auth::{AuthUser, OptionalAuth, RequireAuth};
pub use rate_limit::{RateLimitLayer, RateLimitState};
pub use require_auth::{require_auth_middleware, AuthMiddlewareState};
pub use security_headers::security_headers_middleware;
