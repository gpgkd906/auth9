//! Tenant domain model

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// Tenant status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    #[default]
    Active,
    Inactive,
    Suspended,
}

impl std::str::FromStr for TenantStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(TenantStatus::Active),
            "inactive" => Ok(TenantStatus::Inactive),
            "suspended" => Ok(TenantStatus::Suspended),
            _ => Err(format!("Unknown tenant status: {}", s)),
        }
    }
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantStatus::Active => write!(f, "active"),
            TenantStatus::Inactive => write!(f, "inactive"),
            TenantStatus::Suspended => write!(f, "suspended"),
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for TenantStatus {
    fn decode(
        value: sqlx::mysql::MySqlValueRef<'r>,
    ) -> std::result::Result<Self, sqlx::error::BoxDynError> {
        let s: String = sqlx::Decode::<'r, sqlx::MySql>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Type<sqlx::MySql> for TenantStatus {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for TenantStatus {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = match self {
            TenantStatus::Active => "active",
            TenantStatus::Inactive => "inactive",
            TenantStatus::Suspended => "suspended",
        };
        <&str as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&s, buf)
    }
}

/// Tenant settings stored as JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSettings {
    /// Whether MFA is required for all users
    #[serde(default)]
    pub require_mfa: bool,
    /// Allowed authentication methods
    #[serde(default)]
    pub allowed_auth_methods: Vec<String>,
    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: i64,
    /// Custom branding colors
    #[serde(default)]
    pub branding: TenantBranding,
}

fn default_session_timeout() -> i64 {
    3600 // 1 hour
}

impl Default for TenantSettings {
    fn default() -> Self {
        Self {
            require_mfa: false,
            allowed_auth_methods: Vec::new(),
            session_timeout_secs: default_session_timeout(),
            branding: TenantBranding::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantBranding {
    pub primary_color: Option<String>,
    pub logo_url: Option<String>,
}

/// Tenant entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: StringUuid,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
    #[sqlx(json)]
    pub settings: TenantSettings,
    pub status: TenantStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Tenant {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            name: String::new(),
            slug: String::new(),
            logo_url: None,
            settings: TenantSettings::default(),
            status: TenantStatus::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for creating a new tenant
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateTenantInput {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 63), custom(function = "validate_slug"))]
    pub slug: String,
    pub logo_url: Option<String>,
    pub settings: Option<TenantSettings>,
}

/// Validate slug format (lowercase alphanumeric with hyphens)
fn validate_slug(slug: &str) -> Result<(), validator::ValidationError> {
    if SLUG_REGEX.is_match(slug) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_slug"))
    }
}

/// Input for updating a tenant
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateTenantInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub settings: Option<TenantSettings>,
    pub status: Option<TenantStatus>,
}

// Regex for slug validation
lazy_static::lazy_static! {
    pub static ref SLUG_REGEX: regex::Regex = regex::Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap();
}

/// Tenant-Service association entity
/// Represents which services are enabled for a tenant
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantServiceAssoc {
    pub tenant_id: StringUuid,
    pub service_id: StringUuid,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Service with enabled status for a tenant
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServiceWithStatus {
    pub id: StringUuid,
    pub name: String,
    pub base_url: Option<String>,
    pub status: String,
    pub enabled: bool,
}

/// Input for toggling service for a tenant
#[derive(Debug, Clone, Deserialize)]
pub struct ToggleServiceInput {
    pub service_id: uuid::Uuid,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_tenant_default() {
        let tenant = Tenant::default();
        assert!(!tenant.id.is_nil());
        assert_eq!(tenant.status, TenantStatus::Active);
        assert!(tenant.name.is_empty());
        assert!(tenant.slug.is_empty());
        assert!(tenant.logo_url.is_none());
    }

    #[test]
    fn test_tenant_with_values() {
        let tenant = Tenant {
            name: "My Tenant".to_string(),
            slug: "my-tenant".to_string(),
            logo_url: Some("https://example.com/logo.png".to_string()),
            ..Default::default()
        };

        assert_eq!(tenant.name, "My Tenant");
        assert_eq!(tenant.slug, "my-tenant");
        assert!(tenant.logo_url.is_some());
    }

    #[test]
    fn test_tenant_settings_default() {
        let settings = TenantSettings::default();
        assert!(!settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 3600);
        assert!(settings.allowed_auth_methods.is_empty());
        assert!(settings.branding.primary_color.is_none());
        assert!(settings.branding.logo_url.is_none());
    }

    #[test]
    fn test_tenant_settings_with_values() {
        let settings = TenantSettings {
            require_mfa: true,
            allowed_auth_methods: vec!["password".to_string(), "oidc".to_string()],
            session_timeout_secs: 7200,
            branding: TenantBranding {
                primary_color: Some("#FF5733".to_string()),
                logo_url: Some("https://example.com/logo.png".to_string()),
            },
        };

        assert!(settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 7200);
        assert_eq!(settings.allowed_auth_methods.len(), 2);
        assert!(settings.branding.primary_color.is_some());
    }

    #[test]
    fn test_tenant_branding_default() {
        let branding = TenantBranding::default();
        assert!(branding.primary_color.is_none());
        assert!(branding.logo_url.is_none());
    }

    #[test]
    fn test_slug_regex() {
        // Valid slugs
        assert!(SLUG_REGEX.is_match("my-tenant"));
        assert!(SLUG_REGEX.is_match("tenant123"));
        assert!(SLUG_REGEX.is_match("a"));
        assert!(SLUG_REGEX.is_match("abc-def-ghi"));
        assert!(SLUG_REGEX.is_match("tenant1-test2"));

        // Invalid slugs
        assert!(!SLUG_REGEX.is_match("My Tenant"));
        assert!(!SLUG_REGEX.is_match("tenant_name"));
        assert!(!SLUG_REGEX.is_match("UPPERCASE"));
        assert!(!SLUG_REGEX.is_match("-start-with-dash"));
        assert!(!SLUG_REGEX.is_match("end-with-dash-"));
        assert!(!SLUG_REGEX.is_match("double--dash"));
        assert!(!SLUG_REGEX.is_match(""));
    }

    #[test]
    fn test_validate_slug_valid() {
        let result = validate_slug("my-tenant");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_slug_invalid() {
        let result = validate_slug("Invalid Slug");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code.as_ref(), "invalid_slug");
    }

    #[test]
    fn test_tenant_status_from_str() {
        assert_eq!(
            "active".parse::<TenantStatus>().unwrap(),
            TenantStatus::Active
        );
        assert_eq!(
            "inactive".parse::<TenantStatus>().unwrap(),
            TenantStatus::Inactive
        );
        assert_eq!(
            "suspended".parse::<TenantStatus>().unwrap(),
            TenantStatus::Suspended
        );

        // Case insensitive
        assert_eq!(
            "ACTIVE".parse::<TenantStatus>().unwrap(),
            TenantStatus::Active
        );
        assert_eq!(
            "Active".parse::<TenantStatus>().unwrap(),
            TenantStatus::Active
        );
    }

    #[test]
    fn test_tenant_status_from_str_invalid() {
        let result = "invalid".parse::<TenantStatus>();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tenant status"));
    }

    #[test]
    fn test_tenant_status_display() {
        assert_eq!(format!("{}", TenantStatus::Active), "active");
        assert_eq!(format!("{}", TenantStatus::Inactive), "inactive");
        assert_eq!(format!("{}", TenantStatus::Suspended), "suspended");
    }

    #[test]
    fn test_tenant_status_default() {
        let status = TenantStatus::default();
        assert_eq!(status, TenantStatus::Active);
    }

    #[test]
    fn test_tenant_status_serialization() {
        assert_eq!(
            serde_json::to_string(&TenantStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&TenantStatus::Inactive).unwrap(),
            "\"inactive\""
        );
        assert_eq!(
            serde_json::to_string(&TenantStatus::Suspended).unwrap(),
            "\"suspended\""
        );
    }

    #[test]
    fn test_tenant_status_deserialization() {
        let active: TenantStatus = serde_json::from_str("\"active\"").unwrap();
        let inactive: TenantStatus = serde_json::from_str("\"inactive\"").unwrap();
        let suspended: TenantStatus = serde_json::from_str("\"suspended\"").unwrap();

        assert_eq!(active, TenantStatus::Active);
        assert_eq!(inactive, TenantStatus::Inactive);
        assert_eq!(suspended, TenantStatus::Suspended);
    }

    #[test]
    fn test_create_tenant_input_valid() {
        let input = CreateTenantInput {
            name: "My Tenant".to_string(),
            slug: "my-tenant".to_string(),
            logo_url: Some("https://example.com/logo.png".to_string()),
            settings: Some(TenantSettings::default()),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_tenant_input_minimal() {
        let input = CreateTenantInput {
            name: "T".to_string(),
            slug: "t".to_string(),
            logo_url: None,
            settings: None,
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_tenant_input_empty_name() {
        let input = CreateTenantInput {
            name: "".to_string(),
            slug: "valid-slug".to_string(),
            logo_url: None,
            settings: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_tenant_input_empty_slug() {
        let input = CreateTenantInput {
            name: "Valid Name".to_string(),
            slug: "".to_string(),
            logo_url: None,
            settings: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_tenant_input_invalid_slug() {
        let input = CreateTenantInput {
            name: "Valid Name".to_string(),
            slug: "Invalid Slug".to_string(),
            logo_url: None,
            settings: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_tenant_input_valid() {
        let input = UpdateTenantInput {
            name: Some("Updated Name".to_string()),
            logo_url: Some("https://example.com/new-logo.png".to_string()),
            settings: Some(TenantSettings::default()),
            status: Some(TenantStatus::Inactive),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_tenant_input_partial() {
        let input = UpdateTenantInput {
            name: None,
            logo_url: None,
            settings: None,
            status: Some(TenantStatus::Suspended),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_tenant_input_empty_name() {
        let input = UpdateTenantInput {
            name: Some("".to_string()),
            logo_url: None,
            settings: None,
            status: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_tenant_settings_serialization() {
        let settings = TenantSettings {
            require_mfa: true,
            allowed_auth_methods: vec!["password".to_string()],
            session_timeout_secs: 3600,
            branding: TenantBranding::default(),
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: TenantSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.require_mfa, settings.require_mfa);
        assert_eq!(
            deserialized.session_timeout_secs,
            settings.session_timeout_secs
        );
    }

    #[test]
    fn test_default_session_timeout() {
        assert_eq!(default_session_timeout(), 3600);
    }

    #[test]
    fn test_tenant_equality() {
        let tenant1 = Tenant::default();
        let tenant2 = Tenant {
            id: tenant1.id,
            ..Default::default()
        };

        // Same ID should have same identity
        assert_eq!(tenant1.id, tenant2.id);
    }

    #[test]
    fn test_tenant_status_encode_by_ref() {
        for status in [
            TenantStatus::Active,
            TenantStatus::Inactive,
            TenantStatus::Suspended,
        ] {
            let mut buf = Vec::new();
            let result = sqlx::Encode::<sqlx::MySql>::encode_by_ref(&status, &mut buf);
            assert!(result.is_ok());
            let encoded = String::from_utf8_lossy(&buf);
            assert!(encoded.contains(&status.to_string()));
        }
    }

    #[test]
    fn test_toggle_service_input_deserialize() {
        let json = r#"{"service_id": "550e8400-e29b-41d4-a716-446655440000", "enabled": true}"#;
        let input: ToggleServiceInput = serde_json::from_str(json).unwrap();
        assert!(input.enabled);
        assert_eq!(
            input.service_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_toggle_service_input_disable() {
        let json = r#"{"service_id": "550e8400-e29b-41d4-a716-446655440000", "enabled": false}"#;
        let input: ToggleServiceInput = serde_json::from_str(json).unwrap();
        assert!(!input.enabled);
    }

    #[test]
    fn test_service_with_status_serialize() {
        let sws = ServiceWithStatus {
            id: StringUuid::new_v4(),
            name: "Auth Service".to_string(),
            base_url: Some("https://auth.example.com".to_string()),
            status: "active".to_string(),
            enabled: true,
        };

        let json = serde_json::to_string(&sws).unwrap();
        assert!(json.contains("Auth Service"));
        assert!(json.contains("auth.example.com"));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_tenant_service_assoc_serialize() {
        let now = Utc::now();
        let assoc = TenantServiceAssoc {
            tenant_id: StringUuid::new_v4(),
            service_id: StringUuid::new_v4(),
            enabled: true,
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&assoc).unwrap();
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_tenant_settings_json_defaults() {
        // When fields are missing from JSON, defaults should be used
        let json = r#"{}"#;
        let settings: TenantSettings = serde_json::from_str(json).unwrap();
        assert!(!settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 3600);
        assert!(settings.allowed_auth_methods.is_empty());
    }

    #[test]
    fn test_tenant_settings_partial_json() {
        let json = r#"{"require_mfa": true}"#;
        let settings: TenantSettings = serde_json::from_str(json).unwrap();
        assert!(settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 3600); // default
    }
}
