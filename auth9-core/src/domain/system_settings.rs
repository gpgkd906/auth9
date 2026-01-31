//! System settings domain types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// System setting row from the database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemSettingRow {
    pub id: i32,
    pub category: String,
    pub setting_key: String,
    #[sqlx(json)]
    pub value: serde_json::Value,
    pub encrypted: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// System setting categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingCategory {
    /// Email configuration
    Email,
    /// Authentication settings
    Auth,
    /// Branding/UI settings
    Branding,
}

impl SettingCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Auth => "auth",
            Self::Branding => "branding",
        }
    }
}

impl std::fmt::Display for SettingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SettingCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email" => Ok(Self::Email),
            "auth" => Ok(Self::Auth),
            "branding" => Ok(Self::Branding),
            _ => Err(format!("Unknown setting category: {}", s)),
        }
    }
}

/// Well-known setting keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKey {
    /// Email provider configuration
    EmailProvider,
}

impl SettingKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EmailProvider => "provider",
        }
    }

    pub fn category(&self) -> SettingCategory {
        match self {
            Self::EmailProvider => SettingCategory::Email,
        }
    }
}

/// Input for creating/updating a system setting
#[derive(Debug, Clone, Deserialize)]
pub struct UpsertSystemSettingInput {
    pub category: String,
    pub setting_key: String,
    pub value: serde_json::Value,
    pub encrypted: bool,
    pub description: Option<String>,
}

/// API response for system settings (with sensitive data masked)
#[derive(Debug, Clone, Serialize)]
pub struct SystemSettingResponse {
    pub category: String,
    pub setting_key: String,
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<SystemSettingRow> for SystemSettingResponse {
    fn from(row: SystemSettingRow) -> Self {
        Self {
            category: row.category,
            setting_key: row.setting_key,
            value: row.value,
            description: row.description,
            updated_at: row.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setting_category_as_str() {
        assert_eq!(SettingCategory::Email.as_str(), "email");
        assert_eq!(SettingCategory::Auth.as_str(), "auth");
        assert_eq!(SettingCategory::Branding.as_str(), "branding");
    }

    #[test]
    fn test_setting_category_display() {
        assert_eq!(format!("{}", SettingCategory::Email), "email");
        assert_eq!(format!("{}", SettingCategory::Auth), "auth");
    }

    #[test]
    fn test_setting_category_from_str() {
        assert_eq!(
            "email".parse::<SettingCategory>().unwrap(),
            SettingCategory::Email
        );
        assert_eq!(
            "EMAIL".parse::<SettingCategory>().unwrap(),
            SettingCategory::Email
        );
        assert!("invalid".parse::<SettingCategory>().is_err());
    }

    #[test]
    fn test_setting_key() {
        assert_eq!(SettingKey::EmailProvider.as_str(), "provider");
        assert_eq!(SettingKey::EmailProvider.category(), SettingCategory::Email);
    }

    #[test]
    fn test_setting_category_serialization() {
        let category = SettingCategory::Email;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"email\"");

        let parsed: SettingCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, category);
    }
}
