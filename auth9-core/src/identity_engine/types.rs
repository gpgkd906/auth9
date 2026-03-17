use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub first_broker_login_flow_alias: Option<String>,
    pub config: HashMap<String, String>,
    /// Preserve backend-specific passthrough fields so adapter round-trips do not lose state.
    pub extra: HashMap<String, serde_json::Value>,
}

/// Neutral federated identity representation exposed to business services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FederatedIdentityRepresentation {
    pub identity_provider: String,
    pub user_id: String,
    pub user_name: Option<String>,
}
