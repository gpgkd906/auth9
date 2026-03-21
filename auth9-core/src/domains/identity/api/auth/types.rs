//! Request/response types for authentication API.

use crate::models::common::StringUuid;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// OIDC Authorization request
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorizeRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    /// State parameter is required for CSRF protection
    pub state: String,
    pub nonce: Option<String>,
    pub connector_alias: Option<String>,
    pub kc_action: Option<String>,
    pub ui_locales: Option<String>,
    /// PKCE code challenge (RFC 7636)
    pub code_challenge: Option<String>,
    /// PKCE code challenge method (e.g. "S256")
    pub code_challenge_method: Option<String>,
}

/// OIDC callback handler
#[derive(Debug, Deserialize)]
pub struct CallbackRequest {
    pub code: String,
    pub state: Option<String>,
}

/// Token endpoint (for client credentials, etc.)
#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub refresh_token: Option<String>,
    /// PKCE code verifier (RFC 7636)
    pub code_verifier: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TenantTokenExchangeRequest {
    pub tenant_id: String,
    pub service_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
}

/// Request to complete authorization after hosted login
#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthorizeCompleteRequest {
    pub login_challenge_id: String,
}

/// Response from authorize_complete with redirect URL
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthorizeCompleteResponse {
    pub redirect_url: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnterpriseSsoDiscoveryResponse {
    pub tenant_id: StringUuid,
    pub tenant_slug: String,
    pub connector_alias: String,
    pub authorize_url: String,
}
