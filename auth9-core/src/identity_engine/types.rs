use crate::models::email::SmtpServerConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Neutral identity credential input for user lifecycle operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityCredentialInput {
    pub credential_type: String,
    pub value: String,
    pub temporary: bool,
}

/// Neutral user creation input exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityUserCreateInput {
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    pub email_verified: bool,
    pub credentials: Option<Vec<IdentityCredentialInput>>,
}

/// Neutral user update input exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IdentityUserUpdateInput {
    pub username: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: Option<bool>,
    pub email_verified: Option<bool>,
    pub required_actions: Option<Vec<String>>,
}

/// Neutral user representation exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityUserRepresentation {
    pub id: Option<String>,
    pub username: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    pub email_verified: bool,
    pub attributes: HashMap<String, Vec<String>>,
}

/// Neutral credential representation exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityCredentialRepresentation {
    pub id: String,
    pub credential_type: String,
    pub user_label: Option<String>,
    pub created_date: Option<i64>,
}

/// Neutral identity provider representation exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityProviderRepresentation {
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub enabled: bool,
    pub trust_email: bool,
    pub store_token: bool,
    pub link_only: bool,
    /// First login merge policy: auto_merge, prompt_confirm, create_new
    #[serde(default = "default_first_login_policy")]
    pub first_login_policy: String,
    pub first_broker_login_flow_alias: Option<String>,
    pub config: HashMap<String, String>,
    /// Preserve backend-specific passthrough fields so adapter round-trips do not lose state.
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_first_login_policy() -> String {
    "auto_merge".to_string()
}

/// Neutral SAML protocol mapper representation exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityProtocolMapperRepresentation {
    pub name: String,
    pub protocol: String,
    pub protocol_mapper: String,
    pub config: HashMap<String, String>,
}

/// Neutral pending action information exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingActionInfo {
    pub id: String,
    pub action_type: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Neutral email verification token info exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationTokenInfo {
    pub id: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Neutral OIDC client representation for IdentityClientStore operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OidcClientRepresentation {
    pub id: Option<String>,
    pub client_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub public_client: bool,
    pub redirect_uris: Vec<String>,
    pub web_origins: Vec<String>,
    pub secret: Option<String>,
    pub protocol: Option<String>,
    pub base_url: Option<String>,
    pub root_url: Option<String>,
    pub admin_url: Option<String>,
    pub attributes: Option<HashMap<String, String>>,
}

/// Neutral realm settings update for IdentityEngine::update_realm().
#[derive(Debug, Clone, Default)]
pub struct RealmSettingsUpdate {
    pub registration_allowed: Option<bool>,
    pub reset_password_allowed: Option<bool>,
    pub smtp_server: Option<SmtpServerConfig>,
    pub password_policy: Option<String>,
    pub brute_force_protected: Option<bool>,
    pub max_failure_wait_seconds: Option<i32>,
    pub failure_factor: Option<i32>,
    pub wait_increment_seconds: Option<i32>,
}

/// Neutral SAML client representation exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentitySamlClientRepresentation {
    pub id: Option<String>,
    pub client_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub protocol: String,
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub protocol_mappers: Vec<IdentityProtocolMapperRepresentation>,
}
