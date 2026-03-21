//! Linked identity domain models for social/SSO identity tracking

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

/// Linked identity entity (tracks external IdP connections)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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

/// First login merge policy for external identity providers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FirstLoginPolicy {
    /// Automatically link to existing user by email match
    #[default]
    AutoMerge,
    /// Show confirmation page before linking to existing account
    PromptConfirm,
    /// Always create a new account, never auto-link by email
    CreateNew,
}

impl std::fmt::Display for FirstLoginPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AutoMerge => write!(f, "auto_merge"),
            Self::PromptConfirm => write!(f, "prompt_confirm"),
            Self::CreateNew => write!(f, "create_new"),
        }
    }
}

impl std::str::FromStr for FirstLoginPolicy {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "auto_merge" => Ok(Self::AutoMerge),
            "prompt_confirm" => Ok(Self::PromptConfirm),
            "create_new" => Ok(Self::CreateNew),
            _ => Err(format!("Unknown first login policy: {}", s)),
        }
    }
}

/// Pending merge data stored in cache when first_login_policy = prompt_confirm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingMergeData {
    pub existing_user_id: String,
    pub existing_email: String,
    pub external_user_id: String,
    pub provider_alias: String,
    pub provider_type: String,
    pub external_email: Option<String>,
    pub display_name: Option<String>,
    pub login_challenge_id: String,
    /// Tenant ID (for enterprise SSO flows)
    pub tenant_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Provider federated identity representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderFederatedIdentity {
    pub identity_provider: String,
    pub user_id: String,
    pub user_name: Option<String>,
}

/// Linked identity info returned to clients
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    fn test_provider_federated_identity_deserialization() {
        let json = r#"{
            "identityProvider": "google",
            "userId": "12345",
            "userName": "john@gmail.com"
        }"#;

        let identity: ProviderFederatedIdentity = serde_json::from_str(json).unwrap();
        assert_eq!(identity.identity_provider, "google");
        assert_eq!(identity.user_id, "12345");
        assert_eq!(identity.user_name, Some("john@gmail.com".to_string()));
    }

    #[test]
    fn test_provider_federated_identity_minimal() {
        let json = r#"{
            "identityProvider": "github",
            "userId": "67890"
        }"#;

        let identity: ProviderFederatedIdentity = serde_json::from_str(json).unwrap();
        assert_eq!(identity.identity_provider, "github");
        assert!(identity.user_name.is_none());
    }

    #[test]
    fn test_first_login_policy_from_str() {
        assert_eq!(
            "auto_merge".parse::<FirstLoginPolicy>().unwrap(),
            FirstLoginPolicy::AutoMerge
        );
        assert_eq!(
            "prompt_confirm".parse::<FirstLoginPolicy>().unwrap(),
            FirstLoginPolicy::PromptConfirm
        );
        assert_eq!(
            "create_new".parse::<FirstLoginPolicy>().unwrap(),
            FirstLoginPolicy::CreateNew
        );
        assert!("invalid".parse::<FirstLoginPolicy>().is_err());
    }

    #[test]
    fn test_first_login_policy_display() {
        assert_eq!(FirstLoginPolicy::AutoMerge.to_string(), "auto_merge");
        assert_eq!(
            FirstLoginPolicy::PromptConfirm.to_string(),
            "prompt_confirm"
        );
        assert_eq!(FirstLoginPolicy::CreateNew.to_string(), "create_new");
    }

    #[test]
    fn test_first_login_policy_serde_roundtrip() {
        let policy = FirstLoginPolicy::PromptConfirm;
        let json = serde_json::to_string(&policy).unwrap();
        assert_eq!(json, "\"prompt_confirm\"");
        let parsed: FirstLoginPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, FirstLoginPolicy::PromptConfirm);
    }

    #[test]
    fn test_pending_merge_data_roundtrip() {
        let data = PendingMergeData {
            existing_user_id: "user-123".to_string(),
            existing_email: "user@example.com".to_string(),
            external_user_id: "ext-456".to_string(),
            provider_alias: "google".to_string(),
            provider_type: "google".to_string(),
            external_email: Some("user@gmail.com".to_string()),
            display_name: Some("Test User".to_string()),
            login_challenge_id: "challenge-789".to_string(),
            tenant_id: None,
            ip_address: None,
            user_agent: None,
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: PendingMergeData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.existing_user_id, "user-123");
        assert_eq!(parsed.provider_alias, "google");
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
