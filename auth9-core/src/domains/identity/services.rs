//! Identity domain service facade.
//!
//! This keeps domain-level imports stable while underlying implementations
//! are still located in `crate::service::*`.

pub use crate::domains::identity::service::identity_provider::IdentityProviderService;
pub use crate::domains::identity::service::keycloak_oidc::{
    AuthorizeParams, CallbackResult, KeycloakOidcService, OidcTokenResponse,
};
pub use crate::domains::identity::service::password::PasswordService;
pub use crate::domains::identity::service::session::SessionService;
pub use crate::domains::identity::service::webauthn::WebAuthnService;
