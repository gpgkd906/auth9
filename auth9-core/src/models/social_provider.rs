//! Social provider domain models for Auth9-managed social login configuration.

use super::common::StringUuid;
use crate::identity_engine::IdentityProviderRepresentation;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Social provider entity stored in Auth9 database.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SocialProvider {
    pub id: StringUuid,
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_type: String,
    pub enabled: bool,
    pub trust_email: bool,
    pub store_token: bool,
    pub link_only: bool,
    pub first_login_policy: String,
    #[sqlx(json)]
    pub config: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for SocialProvider {
    fn default() -> Self {
        Self {
            id: StringUuid::new_v4(),
            alias: String::new(),
            display_name: None,
            provider_type: String::new(),
            enabled: true,
            trust_email: false,
            store_token: false,
            link_only: false,
            first_login_policy: "auto_merge".to_string(),
            config: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

/// Input for creating a social provider.
#[derive(Debug, Clone)]
pub struct CreateSocialProviderInput {
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_type: String,
    pub enabled: bool,
    pub trust_email: bool,
    pub store_token: bool,
    pub link_only: bool,
    pub first_login_policy: String,
    pub config: HashMap<String, String>,
}

/// Input for updating a social provider.
#[derive(Debug, Clone)]
pub struct UpdateSocialProviderInput {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub trust_email: Option<bool>,
    pub store_token: Option<bool>,
    pub link_only: Option<bool>,
    pub first_login_policy: Option<String>,
    pub config: Option<HashMap<String, String>>,
}

// ── Conversions between SocialProvider and IdentityProviderRepresentation ──

impl From<SocialProvider> for IdentityProviderRepresentation {
    fn from(sp: SocialProvider) -> Self {
        Self {
            alias: sp.alias,
            display_name: sp.display_name,
            provider_id: sp.provider_type,
            enabled: sp.enabled,
            trust_email: sp.trust_email,
            store_token: sp.store_token,
            link_only: sp.link_only,
            first_login_policy: sp.first_login_policy,
            first_broker_login_flow_alias: None,
            config: sp.config,
            extra: HashMap::new(),
        }
    }
}

impl From<&IdentityProviderRepresentation> for CreateSocialProviderInput {
    fn from(repr: &IdentityProviderRepresentation) -> Self {
        Self {
            alias: repr.alias.clone(),
            display_name: repr.display_name.clone(),
            provider_type: repr.provider_id.clone(),
            enabled: repr.enabled,
            trust_email: repr.trust_email,
            store_token: repr.store_token,
            link_only: repr.link_only,
            first_login_policy: repr.first_login_policy.clone(),
            config: repr.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_social_provider_default() {
        let sp = SocialProvider::default();
        assert!(!sp.id.is_nil());
        assert!(sp.alias.is_empty());
        assert!(sp.enabled);
        assert!(!sp.trust_email);
        assert!(sp.config.is_empty());
    }

    #[test]
    fn test_social_provider_to_identity_provider_repr() {
        let mut config = HashMap::new();
        config.insert("clientId".to_string(), "test-id".to_string());

        let sp = SocialProvider {
            alias: "google".to_string(),
            display_name: Some("Google".to_string()),
            provider_type: "google".to_string(),
            enabled: true,
            trust_email: true,
            config,
            ..Default::default()
        };

        let repr: IdentityProviderRepresentation = sp.into();
        assert_eq!(repr.alias, "google");
        assert_eq!(repr.provider_id, "google");
        assert!(repr.trust_email);
        assert_eq!(repr.config.get("clientId"), Some(&"test-id".to_string()));
    }

    #[test]
    fn test_create_input_from_repr() {
        let repr = IdentityProviderRepresentation {
            alias: "github".to_string(),
            display_name: Some("GitHub".to_string()),
            provider_id: "github".to_string(),
            enabled: true,
            trust_email: false,
            store_token: false,
            link_only: false,
            first_login_policy: "auto_merge".to_string(),
            first_broker_login_flow_alias: None,
            config: HashMap::new(),
            extra: HashMap::new(),
        };

        let input = CreateSocialProviderInput::from(&repr);
        assert_eq!(input.alias, "github");
        assert_eq!(input.provider_type, "github");
    }
}
