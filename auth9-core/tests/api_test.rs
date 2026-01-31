//! API integration tests entry point
//!
//! This crate tests the API handlers using mock repositories.
//! No external dependencies (database, Redis, etc.) are required.

mod api;
mod grpc;

// Re-export the API test module's tests
// The tests defined in api/mod.rs will be automatically discovered
