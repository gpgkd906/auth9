//! Keycloak integration module
//!
//! This module provides integration with Keycloak for authentication and
//! identity management. It includes:
//!
//! - [`KeycloakClient`]: Admin API client for user and client management
//! - [`KeycloakSeeder`]: Initialization and seeding utilities
//! - Type definitions for Keycloak API requests and responses

mod client;
mod seeder;
mod types;

pub use client::KeycloakClient;
pub use seeder::KeycloakSeeder;
pub use types::*;
