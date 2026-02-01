//! Auth9 Core - Identity Service Backend
//!
//! This crate provides the core functionality for the Auth9 identity service,
//! including REST API, gRPC services, and integration with Keycloak.

pub mod api;
pub mod cache;
pub mod config;
pub mod crypto;
pub mod domain;
pub mod email;
pub mod error;
pub mod grpc;
pub mod jwt;
pub mod keycloak;
pub mod middleware;
pub mod migration;
pub mod repository;
pub mod server;
pub mod service;
pub mod state;

// Re-export commonly used types
pub use config::Config;
pub use error::{AppError, Result};
