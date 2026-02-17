//! Identity domain service facade.
//!
//! Re-exports from `crate::domains::identity::service::*` for convenience.

pub use crate::domains::identity::service::identity_provider::IdentityProviderService;
pub use crate::domains::identity::service::keycloak_oidc::{
    AuthorizeParams, CallbackResult, KeycloakOidcService, OidcTokenResponse,
};
pub use crate::domains::identity::service::password::PasswordService;
pub use crate::domains::identity::service::session::SessionService;
pub use crate::domains::identity::service::webauthn::WebAuthnService;
