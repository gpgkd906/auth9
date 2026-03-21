//! Authentication API handlers

mod action_helpers;
pub(crate) mod helpers;

pub mod discovery;
pub mod logout;
pub mod oidc_flow;
pub mod token_exchange;
pub mod types;

/// Allowed OIDC scopes whitelist
const ALLOWED_SCOPES: &[&str] = &["openid", "profile", "email"];
#[allow(dead_code)]
const OIDC_STATE_TTL_SECS: u64 = 300;
pub(crate) use helpers::LOGIN_CHALLENGE_TTL_SECS;

// Re-export all public items so that `auth::function_name` paths remain valid.

// Types
pub use types::{
    AuthorizeCompleteRequest, AuthorizeCompleteResponse, AuthorizeRequest, CallbackRequest,
    EnterpriseSsoDiscoveryResponse, TenantTokenExchangeRequest, TokenRequest, TokenResponse,
};

// Discovery
pub use discovery::{jwks, openid_configuration, OpenIdConfiguration};
// Re-export utoipa path structs for OpenAPI derive macro
pub use discovery::{__path_jwks, __path_openid_configuration};

// OIDC flow handlers
pub use oidc_flow::{
    __path_authorize, __path_authorize_complete, __path_callback, __path_enterprise_sso_discovery,
    __path_token,
};
pub use oidc_flow::{authorize, authorize_complete, callback, enterprise_sso_discovery, token};

// Token exchange
pub use token_exchange::{__path_tenant_token, __path_userinfo};
pub use token_exchange::{tenant_token, userinfo};

// Logout
pub use logout::{__path_logout, __path_logout_redirect};
pub use logout::{logout, logout_redirect, LogoutRequest};

// Helpers (public API surface)
pub use helpers::{build_callback_url, validate_redirect_uri};
