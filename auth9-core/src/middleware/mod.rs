//! HTTP middleware for Auth9 Core
//!
//! This module provides middleware components for the REST API:
//! - JWT authentication middleware and AuthUser extractor
//! - Rate limiting middleware

pub mod auth;
pub mod rate_limit;

pub use auth::{AuthUser, RequireAuth};
pub use rate_limit::{RateLimitLayer, RateLimitState};
