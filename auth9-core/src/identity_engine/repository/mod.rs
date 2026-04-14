//! Repository implementations for the identity engine's internal stores.
//!
//! These traits and impls back the auth9 identity engine (credentials,
//! pending actions, email verification). All errors funnel into
//! `crate::error::AppError` via the standard `?` conversions.

pub mod credential;
pub mod pending_action;
pub mod verification;
