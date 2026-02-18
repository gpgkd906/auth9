//! Identity Provider domain models
//!
//! Note: IdP configuration is stored in Keycloak.
//! These models map Keycloak's IdP structures for Auth9 Portal.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use validator::Validate;

/// Identity Provider types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum IdentityProviderType {
    Google,
    GitHub,
    Microsoft,
    Facebook,
    LinkedIn,
    Twitter,
    Oidc,
    Saml,
}

impl std::fmt::Display for IdentityProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityProviderType::Google => write!(f, "google"),
            IdentityProviderType::GitHub => write!(f, "github"),
            IdentityProviderType::Microsoft => write!(f, "microsoft"),
            IdentityProviderType::Facebook => write!(f, "facebook"),
            IdentityProviderType::LinkedIn => write!(f, "linkedin"),
            IdentityProviderType::Twitter => write!(f, "twitter"),
            IdentityProviderType::Oidc => write!(f, "oidc"),
            IdentityProviderType::Saml => write!(f, "saml"),
        }
    }
}

/// Identity Provider representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentityProvider {
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub enabled: bool,
    pub trust_email: bool,
    pub store_token: bool,
    pub link_only: bool,
    pub first_broker_login_flow_alias: Option<String>,
    pub config: HashMap<String, String>,
}

/// Keycloak IdP representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakIdentityProvider {
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub enabled: bool,
    #[serde(default)]
    pub trust_email: bool,
    #[serde(default)]
    pub store_token: bool,
    #[serde(default)]
    pub link_only: bool,
    pub first_broker_login_flow_alias: Option<String>,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

impl From<KeycloakIdentityProvider> for IdentityProvider {
    fn from(kc: KeycloakIdentityProvider) -> Self {
        Self {
            alias: kc.alias,
            display_name: kc.display_name,
            provider_id: kc.provider_id,
            enabled: kc.enabled,
            trust_email: kc.trust_email,
            store_token: kc.store_token,
            link_only: kc.link_only,
            first_broker_login_flow_alias: kc.first_broker_login_flow_alias,
            config: kc.config,
        }
    }
}

impl From<IdentityProvider> for KeycloakIdentityProvider {
    fn from(idp: IdentityProvider) -> Self {
        Self {
            alias: idp.alias,
            display_name: idp.display_name,
            provider_id: idp.provider_id,
            enabled: idp.enabled,
            trust_email: idp.trust_email,
            store_token: idp.store_token,
            link_only: idp.link_only,
            first_broker_login_flow_alias: idp.first_broker_login_flow_alias,
            config: idp.config,
        }
    }
}

// Also implement From for the keycloak module's type
impl From<crate::keycloak::KeycloakIdentityProvider> for IdentityProvider {
    fn from(kc: crate::keycloak::KeycloakIdentityProvider) -> Self {
        Self {
            alias: kc.alias,
            display_name: kc.display_name,
            provider_id: kc.provider_id,
            enabled: kc.enabled,
            trust_email: kc.trust_email,
            store_token: kc.store_token,
            link_only: kc.link_only,
            first_broker_login_flow_alias: kc.first_broker_login_flow_alias,
            config: kc.config,
        }
    }
}

/// Input for creating an IdP
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateIdentityProviderInput {
    #[validate(length(min = 1, max = 255))]
    pub alias: String,
    pub display_name: Option<String>,
    #[validate(length(min = 1))]
    pub provider_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub trust_email: bool,
    #[serde(default)]
    pub store_token: bool,
    #[serde(default)]
    pub link_only: bool,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

/// Input for updating an IdP
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateIdentityProviderInput {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub trust_email: Option<bool>,
    pub store_token: Option<bool>,
    pub link_only: Option<bool>,
    pub config: Option<HashMap<String, String>>,
}

/// IdP configuration templates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentityProviderTemplate {
    pub provider_id: String,
    pub name: String,
    pub description: String,
    pub required_config: Vec<String>,
    pub optional_config: Vec<String>,
}

impl IdentityProviderTemplate {
    pub fn google() -> Self {
        Self {
            provider_id: "google".to_string(),
            name: "Google".to_string(),
            description: "Sign in with Google".to_string(),
            required_config: vec!["clientId".to_string(), "clientSecret".to_string()],
            optional_config: vec!["hostedDomain".to_string()],
        }
    }

    pub fn github() -> Self {
        Self {
            provider_id: "github".to_string(),
            name: "GitHub".to_string(),
            description: "Sign in with GitHub".to_string(),
            required_config: vec!["clientId".to_string(), "clientSecret".to_string()],
            optional_config: vec![],
        }
    }

    pub fn microsoft() -> Self {
        Self {
            provider_id: "microsoft".to_string(),
            name: "Microsoft".to_string(),
            description: "Sign in with Microsoft".to_string(),
            required_config: vec!["clientId".to_string(), "clientSecret".to_string()],
            optional_config: vec!["tenant".to_string()],
        }
    }

    pub fn oidc() -> Self {
        Self {
            provider_id: "oidc".to_string(),
            name: "OpenID Connect".to_string(),
            description: "Generic OIDC provider".to_string(),
            required_config: vec![
                "clientId".to_string(),
                "clientSecret".to_string(),
                "authorizationUrl".to_string(),
                "tokenUrl".to_string(),
            ],
            optional_config: vec![
                "userInfoUrl".to_string(),
                "logoutUrl".to_string(),
                "issuer".to_string(),
            ],
        }
    }

    pub fn saml() -> Self {
        Self {
            provider_id: "saml".to_string(),
            name: "SAML 2.0".to_string(),
            description: "Enterprise SSO via SAML".to_string(),
            required_config: vec!["entityId".to_string(), "singleSignOnServiceUrl".to_string()],
            optional_config: vec![
                "signingCertificate".to_string(),
                "singleLogoutServiceUrl".to_string(),
                "nameIDPolicyFormat".to_string(),
            ],
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::google(),
            Self::github(),
            Self::microsoft(),
            Self::oidc(),
            Self::saml(),
        ]
    }

    /// Find template by provider_id
    pub fn find(provider_id: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|t| t.provider_id == provider_id)
    }

    /// Validate that all required config fields are present
    pub fn validate_config(
        &self,
        config: &HashMap<String, String>,
    ) -> std::result::Result<(), Vec<String>> {
        let missing: Vec<String> = self
            .required_config
            .iter()
            .filter(|key| {
                config
                    .get(key.as_str())
                    .map_or(true, |v| v.trim().is_empty())
            })
            .cloned()
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_provider_type_display() {
        assert_eq!(format!("{}", IdentityProviderType::Google), "google");
        assert_eq!(format!("{}", IdentityProviderType::GitHub), "github");
        assert_eq!(format!("{}", IdentityProviderType::Oidc), "oidc");
        assert_eq!(format!("{}", IdentityProviderType::Saml), "saml");
    }

    #[test]
    fn test_keycloak_idp_to_identity_provider() {
        let mut config = HashMap::new();
        config.insert("clientId".to_string(), "test-client".to_string());

        let kc_idp = KeycloakIdentityProvider {
            alias: "google".to_string(),
            display_name: Some("Google Login".to_string()),
            provider_id: "google".to_string(),
            enabled: true,
            trust_email: true,
            store_token: false,
            link_only: false,
            first_broker_login_flow_alias: None,
            config,
        };

        let idp: IdentityProvider = kc_idp.into();
        assert_eq!(idp.alias, "google");
        assert_eq!(idp.display_name, Some("Google Login".to_string()));
        assert!(idp.enabled);
        assert!(idp.trust_email);
        assert!(idp.config.contains_key("clientId"));
    }

    #[test]
    fn test_create_idp_input_valid() {
        let input = CreateIdentityProviderInput {
            alias: "google".to_string(),
            display_name: Some("Google".to_string()),
            provider_id: "google".to_string(),
            enabled: true,
            trust_email: true,
            store_token: false,
            link_only: false,
            config: HashMap::new(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_idp_input_empty_alias() {
        let input = CreateIdentityProviderInput {
            alias: "".to_string(),
            display_name: None,
            provider_id: "google".to_string(),
            enabled: true,
            trust_email: false,
            store_token: false,
            link_only: false,
            config: HashMap::new(),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_idp_template_google() {
        let template = IdentityProviderTemplate::google();
        assert_eq!(template.provider_id, "google");
        assert!(template.required_config.contains(&"clientId".to_string()));
        assert!(template
            .required_config
            .contains(&"clientSecret".to_string()));
    }

    #[test]
    fn test_idp_template_saml() {
        let template = IdentityProviderTemplate::saml();
        assert_eq!(template.provider_id, "saml");
        assert!(template.required_config.contains(&"entityId".to_string()));
    }

    #[test]
    fn test_all_templates() {
        let templates = IdentityProviderTemplate::all();
        assert_eq!(templates.len(), 5);

        let provider_ids: Vec<_> = templates.iter().map(|t| t.provider_id.as_str()).collect();
        assert!(provider_ids.contains(&"google"));
        assert!(provider_ids.contains(&"github"));
        assert!(provider_ids.contains(&"microsoft"));
        assert!(provider_ids.contains(&"oidc"));
        assert!(provider_ids.contains(&"saml"));
    }

    #[test]
    fn test_template_find() {
        assert!(IdentityProviderTemplate::find("google").is_some());
        assert!(IdentityProviderTemplate::find("github").is_some());
        assert!(IdentityProviderTemplate::find("unknown").is_none());
    }

    #[test]
    fn test_validate_config_missing_required() {
        let template = IdentityProviderTemplate::google();
        let config = HashMap::new();
        let result = template.validate_config(&config);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert!(missing.contains(&"clientId".to_string()));
        assert!(missing.contains(&"clientSecret".to_string()));
    }

    #[test]
    fn test_validate_config_partial() {
        let template = IdentityProviderTemplate::google();
        let mut config = HashMap::new();
        config.insert("clientId".to_string(), "id".to_string());
        let result = template.validate_config(&config);
        assert!(result.is_err());
        let missing = result.unwrap_err();
        assert_eq!(missing, vec!["clientSecret".to_string()]);
    }

    #[test]
    fn test_validate_config_complete() {
        let template = IdentityProviderTemplate::google();
        let mut config = HashMap::new();
        config.insert("clientId".to_string(), "id".to_string());
        config.insert("clientSecret".to_string(), "secret".to_string());
        assert!(template.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_empty_value() {
        let template = IdentityProviderTemplate::google();
        let mut config = HashMap::new();
        config.insert("clientId".to_string(), "  ".to_string());
        config.insert("clientSecret".to_string(), "secret".to_string());
        let result = template.validate_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_keycloak_idp_deserialization() {
        let json = r#"{
            "alias": "github",
            "displayName": "GitHub",
            "providerId": "github",
            "enabled": true,
            "trustEmail": true,
            "config": {"clientId": "abc123"}
        }"#;

        let idp: KeycloakIdentityProvider = serde_json::from_str(json).unwrap();
        assert_eq!(idp.alias, "github");
        assert!(idp.enabled);
        assert!(idp.trust_email);
        assert_eq!(idp.config.get("clientId"), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_keycloak_idp_deserialization_defaults() {
        let json = r#"{
            "alias": "test",
            "providerId": "oidc",
            "enabled": false
        }"#;

        let idp: KeycloakIdentityProvider = serde_json::from_str(json).unwrap();
        assert_eq!(idp.alias, "test");
        assert!(!idp.enabled);
        assert!(!idp.trust_email); // default
        assert!(idp.config.is_empty()); // default
    }
}
