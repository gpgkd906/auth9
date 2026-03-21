//! Auth9 Core - Identity Service Backend
//!
//! This crate provides the core functionality for the Auth9 identity service,
//! including REST API, gRPC services, and identity engine integration.

pub mod cache;
pub mod config;
pub mod crypto;
pub mod domains;
pub mod email;
pub mod error;
pub mod grpc;
pub mod http_support;
pub mod identity_engine;
pub mod jwt;
pub mod middleware;
pub mod migration;
pub mod models;
pub mod openapi;
pub mod policy;
pub mod repository;
pub mod server;
pub mod state;
pub mod telemetry;

// Legacy public alias kept to avoid breaking downstream imports abruptly.
pub use models as domain;

// Re-export commonly used types
pub use config::Config;
pub use error::{AppError, Result};
