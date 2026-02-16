//! Identity domain API facade.
//!
//! During incremental refactor this module re-exports legacy handlers.
//! Route modules should depend on this facade instead of `crate::api::*` directly.

pub mod auth;
pub mod identity_provider;
pub mod password;
pub mod session;
pub mod webauthn;
