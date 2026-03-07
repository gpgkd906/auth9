//! Request/response types for authentication API.

use crate::domain::StringUuid;
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnterpriseSsoDiscoveryResponse {
    pub tenant_id: StringUuid,
    pub tenant_slug: String,
    pub connector_alias: String,
    pub authorize_url: String,
}
