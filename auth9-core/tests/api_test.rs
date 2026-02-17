//! API integration tests entry point
//!
//! This crate tests the API handlers using mock repositories.
//! No external dependencies (database, Redis, etc.) are required.

mod domains;
mod grpc;
mod support;
