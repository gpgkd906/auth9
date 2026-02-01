//! Linked identity domain models for social/SSO identity tracking

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Linked identity entity (tracks external IdP connections)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LinkedIdentity {
    pub id: StringUuid,
    pub user_id: StringUuid,
    pub provider_type: String,
    pub provider_alias: String,
    pub external_user_id: String,
    pub external_email: Option<String>,
    pub linked_at: DateTime<Utc>,
}

impl Default for LinkedIdentity {
    fn default() -> Self {
        Self {
            id: StringUuid::new_v4(),
            user_id: StringUuid::new_v4(),
            provider_type: String::new(),
            provider_alias: String::new(),
            external_user_id: String::new(),
            external_email: None,
            linked_at: Utc::now(),
        }
    }
}

/// Input for creating a linked identity
#[derive(Debug, Clone)]
pub struct CreateLinkedIdentityInput {
    pub user_id: StringUuid,
    pub provider_type: String,
    pub provider_alias: String,
    pub external_user_id: String,
    pub external_email: Option<String>,
}

/// Keycloak federated identity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakFederatedIdentity {
    pub identity_provider: String,
    pub user_id: String,
    pub user_name: Option<String>,
}

/// Linked identity info returned to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedIdentityInfo {
    pub id: String,
    pub provider_type: String,
    pub provider_alias: String,
    pub provider_display_name: Option<String>,
    pub external_email: Option<String>,
    pub linked_at: DateTime<Utc>,
}

impl From<LinkedIdentity> for LinkedIdentityInfo {
    fn from(identity: LinkedIdentity) -> Self {
        let provider_display_name = match identity.provider_type.as_str() {
            "google" => Some("Google".to_string()),
            "github" => Some("GitHub".to_string()),
            "microsoft" => Some("Microsoft".to_string()),
            "facebook" => Some("Facebook".to_string()),
            "linkedin" => Some("LinkedIn".to_string()),
            "twitter" => Some("Twitter/X".to_string()),
            "oidc" => Some("OpenID Connect".to_string()),
            "saml" => Some("SAML".to_string()),
            _ => None,
        };

        Self {
            id: identity.id.to_string(),
            provider_type: identity.provider_type,
            provider_alias: identity.provider_alias,
            provider_display_name,
            external_email: identity.external_email,
            linked_at: identity.linked_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linked_identity_default() {
        let identity = LinkedIdentity::default();
        assert!(!identity.id.is_nil());
        assert!(!identity.user_id.is_nil());
        assert!(identity.provider_type.is_empty());
        assert!(identity.external_email.is_none());
    }

    #[test]
    fn test_linked_identity_info_from_identity() {
        let identity = LinkedIdentity {
            provider_type: "google".to_string(),
            provider_alias: "google".to_string(),
            external_email: Some("user@gmail.com".to_string()),
            ..Default::default()
        };

        let info: LinkedIdentityInfo = identity.into();
        assert_eq!(info.provider_type, "google");
        assert_eq!(info.provider_display_name, Some("Google".to_string()));
        assert_eq!(info.external_email, Some("user@gmail.com".to_string()));
    }

    #[test]
    fn test_linked_identity_info_github() {
        let identity = LinkedIdentity {
            provider_type: "github".to_string(),
            provider_alias: "github".to_string(),
            external_email: Some("user@github.com".to_string()),
            ..Default::default()
        };

        let info: LinkedIdentityInfo = identity.into();
        assert_eq!(info.provider_display_name, Some("GitHub".to_string()));
    }

    #[test]
    fn test_linked_identity_info_unknown_provider() {
        let identity = LinkedIdentity {
            provider_type: "custom".to_string(),
            provider_alias: "custom-idp".to_string(),
            ..Default::default()
        };

        let info: LinkedIdentityInfo = identity.into();
        assert_eq!(info.provider_display_name, None);
    }

    #[test]
    fn test_keycloak_federated_identity_deserialization() {
        let json = r#"{
            "identityProvider": "google",
            "userId": "12345",
            "userName": "john@gmail.com"
        }"#;

        let identity: KeycloakFederatedIdentity = serde_json::from_str(json).unwrap();
        assert_eq!(identity.identity_provider, "google");
        assert_eq!(identity.user_id, "12345");
        assert_eq!(identity.user_name, Some("john@gmail.com".to_string()));
    }

    #[test]
    fn test_keycloak_federated_identity_minimal() {
        let json = r#"{
            "identityProvider": "github",
            "userId": "67890"
        }"#;

        let identity: KeycloakFederatedIdentity = serde_json::from_str(json).unwrap();
        assert_eq!(identity.identity_provider, "github");
        assert!(identity.user_name.is_none());
    }

    #[test]
    fn test_linked_identity_info_serialization() {
        let info = LinkedIdentityInfo {
            id: "id-123".to_string(),
            provider_type: "microsoft".to_string(),
            provider_alias: "azure-ad".to_string(),
            provider_display_name: Some("Microsoft".to_string()),
            external_email: Some("user@outlook.com".to_string()),
            linked_at: Utc::now(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("microsoft"));
        assert!(json.contains("azure-ad"));
        assert!(json.contains("Microsoft"));
    }
}
