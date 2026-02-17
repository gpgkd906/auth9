//! Enterprise SSO domain models (tenant-scoped connectors).

use crate::domain::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSsoConnector {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub keycloak_alias: String,
    pub config: HashMap<String, String>,
    pub domains: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "snake_case")]
pub struct CreateEnterpriseSsoConnectorInput {
    #[validate(length(min = 1, max = 100))]
    pub alias: String,
    pub display_name: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub provider_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default)]
    pub config: HashMap<String, String>,
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "snake_case")]
pub struct UpdateEnterpriseSsoConnectorInput {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub config: Option<HashMap<String, String>>,
    pub domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSsoDiscoveryInput {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSsoDiscoveryResult {
    pub tenant_id: StringUuid,
    pub tenant_slug: String,
    pub connector_alias: String,
    pub keycloak_alias: String,
    pub provider_type: String,
}

fn default_true() -> bool {
    true
}

fn default_priority() -> i32 {
    100
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use validator::Validate;

    // =========================================================================
    // CreateEnterpriseSsoConnectorInput
    // =========================================================================

    #[test]
    fn create_input_valid() {
        let input = CreateEnterpriseSsoConnectorInput {
            alias: "okta-saml".to_string(),
            display_name: Some("Okta SAML".to_string()),
            provider_type: "saml".to_string(),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
            domains: vec!["example.com".to_string()],
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn create_input_empty_alias_fails_validation() {
        let input = CreateEnterpriseSsoConnectorInput {
            alias: "".to_string(),
            display_name: None,
            provider_type: "saml".to_string(),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
            domains: vec![],
        };
        let err = input.validate().unwrap_err();
        assert!(err.field_errors().contains_key("alias"));
    }

    #[test]
    fn create_input_alias_too_long_fails_validation() {
        let input = CreateEnterpriseSsoConnectorInput {
            alias: "a".repeat(101),
            display_name: None,
            provider_type: "saml".to_string(),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
            domains: vec![],
        };
        let err = input.validate().unwrap_err();
        assert!(err.field_errors().contains_key("alias"));
    }

    #[test]
    fn create_input_empty_provider_type_fails_validation() {
        let input = CreateEnterpriseSsoConnectorInput {
            alias: "okta".to_string(),
            display_name: None,
            provider_type: "".to_string(),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
            domains: vec![],
        };
        let err = input.validate().unwrap_err();
        assert!(err.field_errors().contains_key("provider_type"));
    }

    #[test]
    fn create_input_provider_type_too_long_fails_validation() {
        let input = CreateEnterpriseSsoConnectorInput {
            alias: "okta".to_string(),
            display_name: None,
            provider_type: "x".repeat(21),
            enabled: true,
            priority: 100,
            config: HashMap::new(),
            domains: vec![],
        };
        let err = input.validate().unwrap_err();
        assert!(err.field_errors().contains_key("provider_type"));
    }

    #[test]
    fn create_input_serde_defaults() {
        let json = json!({
            "alias": "okta",
            "provider_type": "saml"
        });
        let input: CreateEnterpriseSsoConnectorInput =
            serde_json::from_value(json).unwrap();
        assert!(input.enabled); // default_true
        assert_eq!(input.priority, 100); // default_priority
        assert!(input.config.is_empty());
        assert!(input.domains.is_empty());
    }

    #[test]
    fn create_input_serde_snake_case_rename() {
        let json = json!({
            "alias": "okta",
            "display_name": "Okta SSO",
            "provider_type": "saml",
            "enabled": false,
            "priority": 50,
            "config": {"entity_id": "https://okta.example.com"},
            "domains": ["example.com"]
        });
        let input: CreateEnterpriseSsoConnectorInput =
            serde_json::from_value(json).unwrap();
        assert_eq!(input.alias, "okta");
        assert_eq!(input.display_name.as_deref(), Some("Okta SSO"));
        assert!(!input.enabled);
        assert_eq!(input.priority, 50);
        assert_eq!(input.config.get("entity_id").unwrap(), "https://okta.example.com");
        assert_eq!(input.domains, vec!["example.com"]);
    }

    // =========================================================================
    // UpdateEnterpriseSsoConnectorInput
    // =========================================================================

    #[test]
    fn update_input_all_none_valid() {
        let input = UpdateEnterpriseSsoConnectorInput {
            display_name: None,
            enabled: None,
            priority: None,
            config: None,
            domains: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn update_input_partial_fields() {
        let json = json!({
            "enabled": false,
            "priority": 200
        });
        let input: UpdateEnterpriseSsoConnectorInput =
            serde_json::from_value(json).unwrap();
        assert_eq!(input.enabled, Some(false));
        assert_eq!(input.priority, Some(200));
        assert!(input.display_name.is_none());
        assert!(input.config.is_none());
        assert!(input.domains.is_none());
    }

    // =========================================================================
    // EnterpriseSsoDiscoveryInput
    // =========================================================================

    #[test]
    fn discovery_input_valid_email() {
        let input = EnterpriseSsoDiscoveryInput {
            email: "user@example.com".to_string(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn discovery_input_invalid_email() {
        let input = EnterpriseSsoDiscoveryInput {
            email: "not-an-email".to_string(),
        };
        let err = input.validate().unwrap_err();
        assert!(err.field_errors().contains_key("email"));
    }

    // =========================================================================
    // EnterpriseSsoConnector serialization
    // =========================================================================

    #[test]
    fn connector_serialization_roundtrip() {
        let connector = EnterpriseSsoConnector {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            alias: "okta-saml".to_string(),
            display_name: Some("Okta SAML".to_string()),
            provider_type: "saml".to_string(),
            enabled: true,
            priority: 100,
            keycloak_alias: "acme--okta-saml".to_string(),
            config: HashMap::from([("entityId".to_string(), "https://okta.example.com".to_string())]),
            domains: vec!["example.com".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_value(&connector).unwrap();
        assert_eq!(json["alias"], "okta-saml");
        assert_eq!(json["display_name"], "Okta SAML");
        assert_eq!(json["provider_type"], "saml");
        assert_eq!(json["enabled"], true);
        assert_eq!(json["priority"], 100);
        assert_eq!(json["keycloak_alias"], "acme--okta-saml");

        let deserialized: EnterpriseSsoConnector =
            serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.alias, connector.alias);
        assert_eq!(deserialized.provider_type, connector.provider_type);
    }

    // =========================================================================
    // EnterpriseSsoDiscoveryResult serialization
    // =========================================================================

    #[test]
    fn discovery_result_serialization() {
        let result = EnterpriseSsoDiscoveryResult {
            tenant_id: StringUuid::new_v4(),
            tenant_slug: "acme".to_string(),
            connector_alias: "okta-saml".to_string(),
            keycloak_alias: "acme--okta-saml".to_string(),
            provider_type: "saml".to_string(),
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["tenant_slug"], "acme");
        assert_eq!(json["connector_alias"], "okta-saml");
        assert_eq!(json["keycloak_alias"], "acme--okta-saml");
        assert_eq!(json["provider_type"], "saml");
    }

    // =========================================================================
    // Default helpers
    // =========================================================================

    #[test]
    fn default_true_returns_true() {
        assert!(default_true());
    }

    #[test]
    fn default_priority_returns_100() {
        assert_eq!(default_priority(), 100);
    }
}
