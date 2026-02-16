//! HTTP middleware for Auth9 Core
//!
//! This module provides middleware components for the REST API:
//! - JWT authentication middleware and AuthUser extractor
//! - Rate limiting middleware
//! - Security headers middleware
//! - Authentication enforcement middleware

pub mod auth;
pub mod error_response;
pub mod metrics;
pub mod path_guard;
pub mod rate_limit;
pub mod require_auth;
pub mod security_headers;

pub use auth::{AuthUser, OptionalAuth, RequireAuth};
pub use error_response::normalize_error_response;
pub use path_guard::path_guard_middleware;
pub use rate_limit::{RateLimitLayer, RateLimitState};
pub use require_auth::{require_auth_middleware, AuthMiddlewareState};
pub use security_headers::security_headers_middleware;
