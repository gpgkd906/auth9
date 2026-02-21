//! Branding configuration service

use crate::domain::{
    BrandingConfig, ServiceBranding, SettingCategory, SystemSettingRow, UpsertSystemSettingInput,
};
use crate::domains::platform::service::KeycloakSyncService;
use crate::error::{AppError, Result};
use crate::repository::{ServiceBrandingRepository, ServiceRepository, SystemSettingsRepository};
use std::sync::Arc;
use validator::Validate;

/// Setting key for branding configuration
const BRANDING_CONFIG_KEY: &str = "config";

/// Service for managing branding configuration
pub struct BrandingService<R: SystemSettingsRepository, SBR: ServiceBrandingRepository> {
    repo: Arc<R>,
    service_branding_repo: Arc<SBR>,
    sync_service: Option<Arc<KeycloakSyncService>>,
    /// Allowed domains for branding resource URLs (logo, favicon).
    /// When non-empty, only URLs from these domains are accepted.
    allowed_domains: Vec<String>,
    /// Service repository for resolving client_id -> service_id
    service_repo: Option<Arc<dyn ServiceRepository>>,
}

impl<R: SystemSettingsRepository, SBR: ServiceBrandingRepository> BrandingService<R, SBR> {
    /// Create a new branding service
    pub fn new(repo: Arc<R>, service_branding_repo: Arc<SBR>) -> Self {
        Self {
            repo,
            service_branding_repo,
            sync_service: None,
            allowed_domains: vec![],
            service_repo: None,
        }
    }

    /// Create a new branding service with Keycloak sync
    pub fn with_sync_service(
        repo: Arc<R>,
        service_branding_repo: Arc<SBR>,
        sync_service: Arc<KeycloakSyncService>,
    ) -> Self {
        Self {
            repo,
            service_branding_repo,
            sync_service: Some(sync_service),
            allowed_domains: vec![],
            service_repo: None,
        }
    }

    /// Set allowed domains for branding resource URLs
    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = domains;
        self
    }

    /// Set service repository for client_id resolution
    pub fn with_service_repo(mut self, service_repo: Arc<dyn ServiceRepository>) -> Self {
        self.service_repo = Some(service_repo);
        self
    }

    /// Get system-level branding configuration (default)
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

    /// Update system-level branding configuration
    pub async fn update_branding(&self, config: BrandingConfig) -> Result<BrandingConfig> {
        self.validate_branding(&config)?;

        let value = serde_json::to_value(&config).map_err(|e| AppError::Internal(e.into()))?;

        let input = UpsertSystemSettingInput {
            category: SettingCategory::Branding.as_str().to_string(),
            setting_key: BRANDING_CONFIG_KEY.to_string(),
            value,
            encrypted: false,
            description: Some("Login page branding configuration".to_string()),
        };

        self.repo.upsert(&input).await?;

        if let Some(sync) = &self.sync_service {
            sync.sync_branding_config(&config).await;
        }

        Ok(config)
    }

    /// Get branding for a specific service, falling back to system default
    pub async fn get_branding_for_service(
        &self,
        service_id: crate::domain::StringUuid,
    ) -> Result<BrandingConfig> {
        if let Some(sb) = self.service_branding_repo.get_by_service_id(service_id).await? {
            return Ok(sb.config);
        }
        self.get_branding().await
    }

    /// Get branding by client_id, resolving to service_id first
    pub async fn get_branding_by_client_id(&self, client_id: &str) -> Result<BrandingConfig> {
        let Some(service_repo) = &self.service_repo else {
            return self.get_branding().await;
        };

        match service_repo.find_by_client_id(client_id).await? {
            Some(service) => self.get_branding_for_service(service.id).await,
            None => self.get_branding().await,
        }
    }

    /// Update branding for a specific service
    pub async fn update_service_branding(
        &self,
        service_id: crate::domain::StringUuid,
        config: BrandingConfig,
    ) -> Result<ServiceBranding> {
        self.validate_branding(&config)?;
        self.service_branding_repo.upsert(service_id, &config).await
    }

    /// Delete service-level branding (revert to system default)
    pub async fn delete_service_branding(
        &self,
        service_id: crate::domain::StringUuid,
    ) -> Result<()> {
        self.service_branding_repo
            .delete_by_service_id(service_id)
            .await
    }

    /// Validate branding configuration
    pub fn validate_branding(&self, config: &BrandingConfig) -> Result<()> {
        config
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        if !self.allowed_domains.is_empty() {
            if let Some(url) = &config.logo_url {
                self.validate_url_domain(url, "logo_url")?;
            }
            if let Some(url) = &config.favicon_url {
                self.validate_url_domain(url, "favicon_url")?;
            }
        }

        Ok(())
    }

    /// Validate that a URL's domain is in the allowed domains list
    fn validate_url_domain(&self, url: &str, field_name: &str) -> Result<()> {
        let parsed = url::Url::parse(url)
            .map_err(|_| AppError::Validation(format!("{field_name}: invalid URL")))?;
        let host = parsed.host_str().unwrap_or("");

        let domain_allowed = self.allowed_domains.iter().any(|allowed| {
            host == allowed.as_str() || host.ends_with(&format!(".{}", allowed))
        });

        if !domain_allowed {
            return Err(AppError::Validation(format!(
                "{field_name}: domain '{}' is not in the allowed domains list",
                host
            )));
        }

        Ok(())
    }

    /// Parse branding config from database row
    fn parse_branding_config(&self, row: &SystemSettingRow) -> Result<BrandingConfig> {
        serde_json::from_value(row.value.clone()).map_err(|e| AppError::Internal(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::service_branding::MockServiceBrandingRepository;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use mockall::predicate::*;

    fn create_service(
    ) -> (MockSystemSettingsRepository, MockServiceBrandingRepository) {
        (
            MockSystemSettingsRepository::new(),
            MockServiceBrandingRepository::new(),
        )
    }

    #[tokio::test]
    async fn test_get_branding_default() {
        let (mut mock_sys, mock_sb) = create_service();

        mock_sys
            .expect_get()
            .with(eq("branding"), eq("config"))
            .returning(|_, _| Ok(None));

        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));

        let config = service.get_branding().await.unwrap();
        assert!(config.is_default());
        assert_eq!(config.primary_color, "#007AFF");
    }

    #[tokio::test]
    async fn test_get_branding_custom() {
        let (mut mock_sys, mock_sb) = create_service();

        mock_sys
            .expect_get()
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

        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));

        let config = service.get_branding().await.unwrap();
        assert_eq!(config.primary_color, "#FF0000");
        assert_eq!(config.company_name, Some("Test Corp".to_string()));
    }

    #[tokio::test]
    async fn test_update_branding() {
        let (mut mock_sys, mock_sb) = create_service();

        mock_sys.expect_upsert().returning(|input| {
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

        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));

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
        let (mock_sys, mock_sb) = create_service();
        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));

        let config = BrandingConfig {
            primary_color: "invalid".to_string(),
            ..Default::default()
        };

        let result = service.update_branding(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_branding_for_service_fallback() {
        let (mut mock_sys, mut mock_sb) = create_service();

        // No service-level branding
        mock_sb
            .expect_get_by_service_id()
            .returning(|_| Ok(None));

        // System-level returns default
        mock_sys
            .expect_get()
            .with(eq("branding"), eq("config"))
            .returning(|_, _| Ok(None));

        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));
        let service_id = crate::domain::StringUuid::new_v4();

        let config = service.get_branding_for_service(service_id).await.unwrap();
        assert!(config.is_default());
    }

    #[tokio::test]
    async fn test_get_branding_for_service_override() {
        let (mock_sys, mut mock_sb) = create_service();

        // Service-level branding exists
        mock_sb
            .expect_get_by_service_id()
            .returning(|_| {
                Ok(Some(ServiceBranding {
                    id: "sb-1".to_string(),
                    service_id: "svc-1".to_string(),
                    config: BrandingConfig {
                        primary_color: "#FF0000".to_string(),
                        ..Default::default()
                    },
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));
        let service_id = crate::domain::StringUuid::new_v4();

        let config = service.get_branding_for_service(service_id).await.unwrap();
        assert_eq!(config.primary_color, "#FF0000");
    }

    #[test]
    fn test_validate_branding_valid() {
        let (mock_sys, mock_sb) = create_service();
        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb));

        let config = BrandingConfig::default();
        assert!(service.validate_branding(&config).is_ok());
    }

    #[test]
    fn test_validate_branding_domain_whitelist_blocks_unknown_domain() {
        let (mock_sys, mock_sb) = create_service();
        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb))
            .with_allowed_domains(vec!["cdn.example.com".to_string()]);

        let config = BrandingConfig {
            logo_url: Some("https://evil.com/logo.png".to_string()),
            ..Default::default()
        };
        assert!(service.validate_branding(&config).is_err());
    }

    #[test]
    fn test_validate_branding_domain_whitelist_allows_matching_domain() {
        let (mock_sys, mock_sb) = create_service();
        let service = BrandingService::new(Arc::new(mock_sys), Arc::new(mock_sb))
            .with_allowed_domains(vec!["cdn.example.com".to_string()]);

        let config = BrandingConfig {
            logo_url: Some("https://cdn.example.com/logo.png".to_string()),
            ..Default::default()
        };
        assert!(service.validate_branding(&config).is_ok());
    }
}
