//! Branding configuration service

use crate::domain::{BrandingConfig, SettingCategory, SystemSettingRow, UpsertSystemSettingInput};
use crate::error::{AppError, Result};
use crate::repository::SystemSettingsRepository;
use std::sync::Arc;
use validator::Validate;

/// Setting key for branding configuration
const BRANDING_CONFIG_KEY: &str = "config";

/// Service for managing branding configuration
pub struct BrandingService<R: SystemSettingsRepository> {
    repo: Arc<R>,
}

impl<R: SystemSettingsRepository> BrandingService<R> {
    /// Create a new branding service
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// Get branding configuration
    ///
    /// Returns the stored configuration or default values if not configured
    pub async fn get_branding(&self) -> Result<BrandingConfig> {
        let row = self
            .repo
            .get(SettingCategory::Branding.as_str(), BRANDING_CONFIG_KEY)
            .await?;

        match row {
            Some(row) => self.parse_branding_config(&row),
            None => Ok(BrandingConfig::default()),
        }
    }

    /// Update branding configuration
    ///
    /// Validates and stores the new configuration
    pub async fn update_branding(&self, config: BrandingConfig) -> Result<BrandingConfig> {
        // Validate the configuration
        self.validate_branding(&config)?;

        // Convert to JSON value
        let value =
            serde_json::to_value(&config).map_err(|e| AppError::Internal(e.into()))?;

        // Store in system_settings
        let input = UpsertSystemSettingInput {
            category: SettingCategory::Branding.as_str().to_string(),
            setting_key: BRANDING_CONFIG_KEY.to_string(),
            value,
            encrypted: false, // Branding config has no sensitive data
            description: Some("Login page branding configuration".to_string()),
        };

        self.repo.upsert(&input).await?;

        // Return the updated config
        Ok(config)
    }

    /// Validate branding configuration
    pub fn validate_branding(&self, config: &BrandingConfig) -> Result<()> {
        config
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))
    }

    /// Parse branding config from database row
    fn parse_branding_config(&self, row: &SystemSettingRow) -> Result<BrandingConfig> {
        serde_json::from_value(row.value.clone()).map_err(|e| AppError::Internal(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_get_branding_default() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("branding"), eq("config"))
            .returning(|_, _| Ok(None));

        let service = BrandingService::new(Arc::new(mock));

        let config = service.get_branding().await.unwrap();
        assert!(config.is_default());
        assert_eq!(config.primary_color, "#007AFF");
    }

    #[tokio::test]
    async fn test_get_branding_custom() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("branding"), eq("config"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "branding".to_string(),
                    setting_key: "config".to_string(),
                    value: serde_json::json!({
                        "primary_color": "#FF0000",
                        "secondary_color": "#00FF00",
                        "background_color": "#0000FF",
                        "text_color": "#FFFFFF",
                        "company_name": "Test Corp"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = BrandingService::new(Arc::new(mock));

        let config = service.get_branding().await.unwrap();
        assert_eq!(config.primary_color, "#FF0000");
        assert_eq!(config.company_name, Some("Test Corp".to_string()));
    }

    #[tokio::test]
    async fn test_update_branding() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_upsert().returning(|input| {
            assert_eq!(input.category, "branding");
            assert_eq!(input.setting_key, "config");
            assert!(!input.encrypted);

            Ok(SystemSettingRow {
                id: 1,
                category: input.category.clone(),
                setting_key: input.setting_key.clone(),
                value: input.value.clone(),
                encrypted: input.encrypted,
                description: input.description.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        });

        let service = BrandingService::new(Arc::new(mock));

        let config = BrandingConfig {
            logo_url: Some("https://example.com/logo.png".to_string()),
            primary_color: "#123456".to_string(),
            secondary_color: "#654321".to_string(),
            background_color: "#FFFFFF".to_string(),
            text_color: "#000000".to_string(),
            custom_css: None,
            company_name: Some("My Company".to_string()),
            favicon_url: None,
            allow_registration: false,
        };

        let result = service.update_branding(config.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().primary_color, "#123456");
    }

    #[tokio::test]
    async fn test_update_branding_invalid_color() {
        let mock = MockSystemSettingsRepository::new();
        let service = BrandingService::new(Arc::new(mock));

        let config = BrandingConfig {
            primary_color: "invalid".to_string(),
            ..Default::default()
        };

        let result = service.update_branding(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_branding_invalid_url() {
        let mock = MockSystemSettingsRepository::new();
        let service = BrandingService::new(Arc::new(mock));

        let config = BrandingConfig {
            logo_url: Some("not-a-url".to_string()),
            ..Default::default()
        };

        let result = service.update_branding(config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_branding_valid() {
        let mock = MockSystemSettingsRepository::new();
        let service = BrandingService::new(Arc::new(mock));

        let config = BrandingConfig::default();
        assert!(service.validate_branding(&config).is_ok());
    }

    #[test]
    fn test_validate_branding_with_all_fields() {
        let mock = MockSystemSettingsRepository::new();
        let service = BrandingService::new(Arc::new(mock));

        let config = BrandingConfig {
            logo_url: Some("https://example.com/logo.png".to_string()),
            primary_color: "#AABBCC".to_string(),
            secondary_color: "#112233".to_string(),
            background_color: "#DDEEFF".to_string(),
            text_color: "#445566".to_string(),
            custom_css: Some(".login { color: blue; }".to_string()),
            company_name: Some("Test Company".to_string()),
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
            allow_registration: true,
        };

        assert!(service.validate_branding(&config).is_ok());
    }

    #[tokio::test]
    async fn test_get_branding_with_partial_config() {
        let mut mock = MockSystemSettingsRepository::new();

        // Config with only some fields set
        mock.expect_get()
            .with(eq("branding"), eq("config"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "branding".to_string(),
                    setting_key: "config".to_string(),
                    value: serde_json::json!({
                        "primary_color": "#FF0000",
                        "secondary_color": "#00FF00",
                        "background_color": "#0000FF",
                        "text_color": "#FFFFFF"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = BrandingService::new(Arc::new(mock));

        let config = service.get_branding().await.unwrap();
        assert_eq!(config.primary_color, "#FF0000");
        // Optional fields should be None
        assert!(config.logo_url.is_none());
        assert!(config.company_name.is_none());
    }
}
