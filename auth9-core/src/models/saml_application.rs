//! SAML Application (IdP outbound) domain model
//!
//! Represents external Service Provider registrations for SAML SSO,
//! where Auth9 acts as the Identity Provider.

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use validator::Validate;

/// SAML Application entity — represents an external SP registered with Auth9 as IdP
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct SamlApplication {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub name: String,
    pub entity_id: String,
    pub acs_url: String,
    pub slo_url: Option<String>,
    pub name_id_format: String,
    pub sign_assertions: bool,
    pub sign_responses: bool,
    pub encrypt_assertions: bool,
    pub sp_certificate: Option<String>,
    #[sqlx(json)]
    pub attribute_mappings: Vec<AttributeMapping>,
    pub backend_client_id: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SAML attribute mapping: maps an Auth9 user attribute to a SAML Assertion attribute
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttributeMapping {
    pub source: String,
    pub saml_attribute: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendly_name: Option<String>,
}

/// NameID format shorthand enum
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NameIdFormat {
    #[default]
    Email,
    Persistent,
    Transient,
    Unspecified,
}

impl NameIdFormat {
    /// Convert to full SAML URN
    pub fn to_urn(&self) -> &'static str {
        match self {
            NameIdFormat::Email => "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress",
            NameIdFormat::Persistent => "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent",
            NameIdFormat::Transient => "urn:oasis:names:tc:SAML:2.0:nameid-format:transient",
            NameIdFormat::Unspecified => "urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified",
        }
    }

    /// Parse from full URN or shorthand
    pub fn from_str_flexible(s: &str) -> Self {
        match s {
            "email" | "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress" => {
                NameIdFormat::Email
            }
            "persistent" | "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent" => {
                NameIdFormat::Persistent
            }
            "transient" | "urn:oasis:names:tc:SAML:2.0:nameid-format:transient" => {
                NameIdFormat::Transient
            }
            _ => NameIdFormat::Unspecified,
        }
    }
}

/// Input for creating a new SAML Application
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct CreateSamlApplicationInput {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 512))]
    pub entity_id: String,
    #[validate(url)]
    pub acs_url: String,
    #[validate(url)]
    pub slo_url: Option<String>,
    #[serde(default)]
    pub name_id_format: Option<NameIdFormat>,
    #[serde(default = "default_true")]
    pub sign_assertions: bool,
    #[serde(default = "default_true")]
    pub sign_responses: bool,
    #[serde(default)]
    pub encrypt_assertions: bool,
    pub sp_certificate: Option<String>,
    #[serde(default)]
    pub attribute_mappings: Vec<AttributeMapping>,
}

/// Input for updating an existing SAML Application
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateSamlApplicationInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(url)]
    pub acs_url: Option<String>,
    #[validate(url)]
    pub slo_url: Option<String>,
    pub name_id_format: Option<NameIdFormat>,
    pub sign_assertions: Option<bool>,
    pub sign_responses: Option<bool>,
    pub encrypt_assertions: Option<bool>,
    pub sp_certificate: Option<String>,
    pub attribute_mappings: Option<Vec<AttributeMapping>>,
    pub enabled: Option<bool>,
}

/// SAML Application creation result including computed metadata URL
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SamlApplicationResponse {
    #[serde(flatten)]
    pub app: SamlApplication,
    pub sso_url: String,
}

/// IdP signing certificate information with expiry details
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CertificateInfo {
    pub certificate_pem: String,
    pub expires_at: DateTime<Utc>,
    pub expires_soon: bool,
    pub days_until_expiry: i64,
}

fn default_true() -> bool {
    true
}

/// Allowed attribute mapping source fields
pub const VALID_ATTRIBUTE_SOURCES: &[&str] = &[
    "email",
    "display_name",
    "first_name",
    "last_name",
    "user_id",
    "tenant_roles",
    "tenant_permissions",
];

/// Validate attribute mapping sources
pub fn validate_attribute_mappings(
    mappings: &[AttributeMapping],
) -> Result<(), validator::ValidationError> {
    for mapping in mappings {
        if !VALID_ATTRIBUTE_SOURCES.contains(&mapping.source.as_str()) {
            let mut err = validator::ValidationError::new("invalid_source");
            err.message = Some(
                format!(
                    "Invalid attribute mapping source: '{}'. Valid sources: {:?}",
                    mapping.source, VALID_ATTRIBUTE_SOURCES
                )
                .into(),
            );
            return Err(err);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_name_id_format_to_urn() {
        assert_eq!(
            NameIdFormat::Email.to_urn(),
            "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"
        );
        assert_eq!(
            NameIdFormat::Persistent.to_urn(),
            "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent"
        );
        assert_eq!(
            NameIdFormat::Transient.to_urn(),
            "urn:oasis:names:tc:SAML:2.0:nameid-format:transient"
        );
        assert_eq!(
            NameIdFormat::Unspecified.to_urn(),
            "urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified"
        );
    }

    #[test]
    fn test_name_id_format_from_str_flexible() {
        assert_eq!(
            NameIdFormat::from_str_flexible("email"),
            NameIdFormat::Email
        );
        assert_eq!(
            NameIdFormat::from_str_flexible(
                "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress"
            ),
            NameIdFormat::Email
        );
        assert_eq!(
            NameIdFormat::from_str_flexible("persistent"),
            NameIdFormat::Persistent
        );
        assert_eq!(
            NameIdFormat::from_str_flexible("transient"),
            NameIdFormat::Transient
        );
        assert_eq!(
            NameIdFormat::from_str_flexible("unknown"),
            NameIdFormat::Unspecified
        );
    }

    #[test]
    fn test_name_id_format_default() {
        assert_eq!(NameIdFormat::default(), NameIdFormat::Email);
    }

    #[test]
    fn test_create_input_valid() {
        let input = CreateSamlApplicationInput {
            name: "Salesforce SSO".to_string(),
            entity_id: "https://salesforce.example.com".to_string(),
            acs_url: "https://salesforce.example.com/saml/acs".to_string(),
            slo_url: None,
            name_id_format: Some(NameIdFormat::Email),
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_input_empty_name() {
        let input = CreateSamlApplicationInput {
            name: "".to_string(),
            entity_id: "https://sp.example.com".to_string(),
            acs_url: "https://sp.example.com/acs".to_string(),
            slo_url: None,
            name_id_format: None,
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_input_empty_entity_id() {
        let input = CreateSamlApplicationInput {
            name: "Test".to_string(),
            entity_id: "".to_string(),
            acs_url: "https://sp.example.com/acs".to_string(),
            slo_url: None,
            name_id_format: None,
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_input_invalid_acs_url() {
        let input = CreateSamlApplicationInput {
            name: "Test".to_string(),
            entity_id: "https://sp.example.com".to_string(),
            acs_url: "not-a-url".to_string(),
            slo_url: None,
            name_id_format: None,
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_input_defaults() {
        let json = r#"{
            "name": "Test SP",
            "entity_id": "https://sp.example.com",
            "acs_url": "https://sp.example.com/acs"
        }"#;
        let input: CreateSamlApplicationInput = serde_json::from_str(json).unwrap();
        assert!(input.sign_assertions);
        assert!(input.sign_responses);
        assert!(!input.encrypt_assertions);
        assert!(input.attribute_mappings.is_empty());
    }

    #[test]
    fn test_attribute_mapping_serialization() {
        let mapping = AttributeMapping {
            source: "email".to_string(),
            saml_attribute: "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress"
                .to_string(),
            friendly_name: Some("email".to_string()),
        };
        let json = serde_json::to_string(&mapping).unwrap();
        assert!(json.contains("email"));
        assert!(json.contains("friendly_name"));
    }

    #[test]
    fn test_attribute_mapping_without_friendly_name() {
        let mapping = AttributeMapping {
            source: "email".to_string(),
            saml_attribute: "urn:oid:0.9.2342.19200300.100.1.3".to_string(),
            friendly_name: None,
        };
        let json = serde_json::to_string(&mapping).unwrap();
        assert!(!json.contains("friendly_name"));
    }

    #[test]
    fn test_validate_attribute_mappings_valid() {
        let mappings = vec![
            AttributeMapping {
                source: "email".to_string(),
                saml_attribute: "urn:oid:email".to_string(),
                friendly_name: None,
            },
            AttributeMapping {
                source: "tenant_roles".to_string(),
                saml_attribute: "urn:oid:roles".to_string(),
                friendly_name: None,
            },
        ];
        assert!(validate_attribute_mappings(&mappings).is_ok());
    }

    #[test]
    fn test_validate_attribute_mappings_invalid_source() {
        let mappings = vec![AttributeMapping {
            source: "invalid_field".to_string(),
            saml_attribute: "urn:oid:something".to_string(),
            friendly_name: None,
        }];
        assert!(validate_attribute_mappings(&mappings).is_err());
    }

    #[test]
    fn test_saml_application_serialization() {
        let app = SamlApplication {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            name: "Test SP".to_string(),
            entity_id: "https://sp.example.com".to_string(),
            acs_url: "https://sp.example.com/acs".to_string(),
            slo_url: None,
            name_id_format: "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress".to_string(),
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
            backend_client_id: "kc-uuid-123".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&app).unwrap();
        assert!(json.contains("Test SP"));
        assert!(json.contains("sp.example.com"));
        assert!(json.contains("kc-uuid-123"));
    }

    #[test]
    fn test_name_id_format_serde_roundtrip() {
        let format = NameIdFormat::Persistent;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, "\"persistent\"");
        let parsed: NameIdFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, NameIdFormat::Persistent);
    }
}
