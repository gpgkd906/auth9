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
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> sqlx::encode::IsNull {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_default() {
        let tenant = Tenant::default();
        assert!(!tenant.id.is_nil());
        assert_eq!(tenant.status, TenantStatus::Active);
    }

    #[test]
    fn test_tenant_settings_default() {
        let settings = TenantSettings::default();
        assert!(!settings.require_mfa);
        assert_eq!(settings.session_timeout_secs, 3600);
    }

    #[test]
    fn test_slug_regex() {
        assert!(SLUG_REGEX.is_match("my-tenant"));
        assert!(SLUG_REGEX.is_match("tenant123"));
        assert!(!SLUG_REGEX.is_match("My Tenant"));
        assert!(!SLUG_REGEX.is_match("tenant_name"));
    }
}
