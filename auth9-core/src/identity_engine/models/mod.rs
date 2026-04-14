//! Identity engine internal data models.
//!
//! These types describe the rows backing the auth9 identity engine
//! (credentials, pending actions, email verification). They were originally
//! defined in the `auth9-oidc` crate and moved here when the standalone
//! service skeleton was retired.

pub mod credential;
pub mod pending_action;
pub mod verification;
