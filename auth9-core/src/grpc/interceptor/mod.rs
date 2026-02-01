//! gRPC authentication interceptors
//!
//! This module provides authentication interceptors for gRPC services:
//! - API Key authentication (for development/simple deployments)
//! - mTLS authentication (for production)

pub mod api_key;
pub mod auth;

pub use api_key::ApiKeyAuthenticator;
pub use auth::{AuthContext, AuthInterceptor, GrpcAuthenticator};
