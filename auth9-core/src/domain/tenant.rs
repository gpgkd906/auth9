//! Tenant domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Tenant status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    #[default]
    Active,
    Inactive,
    Suspended,
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
    pub id: Uuid,
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
            id: Uuid::new_v4(),
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
