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

    #[test]
    fn test_setting_category_branding_display() {
        assert_eq!(format!("{}", SettingCategory::Branding), "branding");
    }

    #[test]
    fn test_setting_category_branding_from_str() {
        assert_eq!(
            "branding".parse::<SettingCategory>().unwrap(),
            SettingCategory::Branding
        );
        assert_eq!(
            "BRANDING".parse::<SettingCategory>().unwrap(),
            SettingCategory::Branding
        );
    }

    #[test]
    fn test_setting_category_auth_from_str() {
        assert_eq!(
            "auth".parse::<SettingCategory>().unwrap(),
            SettingCategory::Auth
        );
        assert_eq!(
            "AUTH".parse::<SettingCategory>().unwrap(),
            SettingCategory::Auth
        );
    }

    #[test]
    fn test_system_setting_response_from_row() {
        use chrono::Utc;

        let now = Utc::now();
        let row = SystemSettingRow {
            id: 1,
            category: "email".to_string(),
            setting_key: "provider".to_string(),
            value: serde_json::json!({"type": "smtp"}),
            encrypted: false,
            description: Some("Email provider config".to_string()),
            created_at: now,
            updated_at: now,
        };

        let response: SystemSettingResponse = row.into();
        assert_eq!(response.category, "email");
        assert_eq!(response.setting_key, "provider");
        assert_eq!(response.value["type"], "smtp");
        assert_eq!(response.description.unwrap(), "Email provider config");
    }

    #[test]
    fn test_system_setting_response_from_row_no_description() {
        use chrono::Utc;

        let now = Utc::now();
        let row = SystemSettingRow {
            id: 2,
            category: "auth".to_string(),
            setting_key: "mfa".to_string(),
            value: serde_json::json!(true),
            encrypted: true,
            description: None,
            created_at: now,
            updated_at: now,
        };

        let response: SystemSettingResponse = row.into();
        assert_eq!(response.category, "auth");
        assert!(response.description.is_none());
    }

    #[test]
    fn test_upsert_system_setting_input_deserialize() {
        let json = r#"{
            "category": "email",
            "setting_key": "provider",
            "value": {"type": "ses"},
            "encrypted": false,
            "description": "AWS SES config"
        }"#;
        let input: UpsertSystemSettingInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.category, "email");
        assert_eq!(input.setting_key, "provider");
        assert!(!input.encrypted);
    }

    #[test]
    fn test_setting_category_all_serialization() {
        for (cat, expected) in [
            (SettingCategory::Email, "\"email\""),
            (SettingCategory::Auth, "\"auth\""),
            (SettingCategory::Branding, "\"branding\""),
        ] {
            let json = serde_json::to_string(&cat).unwrap();
            assert_eq!(json, expected);
            let parsed: SettingCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, cat);
        }
    }
}
