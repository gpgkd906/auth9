//! Keycloak sync service for Auth9 ↔ Keycloak state synchronization
//!
//! This service manages the synchronization of configuration between Auth9 and Keycloak.
//! When Auth9 settings change (e.g., branding configuration, email settings), this service
//! ensures the corresponding Keycloak realm settings are updated.

use crate::domain::BrandingConfig;
use crate::error::Result;
use crate::keycloak::{KeycloakClient, RealmUpdate, SmtpServerConfig};
use std::sync::Arc;
use tracing::{error, info};

/// Service for synchronizing Auth9 configuration with Keycloak realm settings
pub struct KeycloakSyncService {
    keycloak: Arc<KeycloakClient>,
}

impl KeycloakSyncService {
    /// Create a new KeycloakSyncService
    pub fn new(keycloak: Arc<KeycloakClient>) -> Self {
        Self { keycloak }
    }

    /// Synchronize realm settings to Keycloak
    ///
    /// This method updates the Keycloak realm configuration to match the
    /// provided settings. Only non-None fields in the update will be applied.
    pub async fn sync_realm_settings(&self, settings: RealmUpdate) -> Result<()> {
        info!("Syncing realm settings to Keycloak: {:?}", settings);
        self.keycloak.update_realm(&settings).await?;
        info!("Successfully synced realm settings");
        Ok(())
    }

    /// Extract realm settings from BrandingConfig
    ///
    /// This method extracts the Keycloak realm settings that should be
    /// synchronized based on the Auth9 branding configuration.
    pub fn extract_realm_settings(config: &BrandingConfig) -> RealmUpdate {
        RealmUpdate {
            registration_allowed: Some(config.allow_registration),
            ..Default::default()
        }
    }

    /// Sync branding configuration to Keycloak
    ///
    /// This is a convenience method that extracts settings from BrandingConfig
    /// and syncs them to Keycloak. Errors are logged but not propagated to
    /// avoid blocking the main branding update flow.
    pub async fn sync_branding_config(&self, config: &BrandingConfig) {
        let realm_settings = Self::extract_realm_settings(config);

        if let Err(e) = self.sync_realm_settings(realm_settings).await {
            error!("Failed to sync realm settings to Keycloak: {}", e);
            // Don't propagate error - Keycloak sync failure shouldn't block branding updates
        }
    }

    /// Sync email configuration to Keycloak realm
    ///
    /// This method updates the Keycloak realm's SMTP server configuration.
    /// When smtp_config is None, the sync is skipped (e.g., when SES lacks credentials).
    /// Errors are logged but not propagated to avoid blocking the main email config update.
    pub async fn sync_email_config(&self, smtp_config: Option<SmtpServerConfig>) {
        let Some(smtp) = smtp_config else {
            info!("Skipping Keycloak email sync - no SMTP config available");
            return;
        };

        let realm_update = RealmUpdate {
            smtp_server: Some(smtp),
            ..Default::default()
        };

        if let Err(e) = self.sync_realm_settings(realm_update).await {
            error!("Failed to sync email config to Keycloak: {}", e);
            // Don't propagate error - Keycloak sync failure shouldn't block email config updates
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeycloakConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_test_config(mock_server_url: &str) -> KeycloakConfig {
        KeycloakConfig {
            url: mock_server_url.to_string(),
            public_url: mock_server_url.to_string(),
            realm: "test-realm".to_string(),
            admin_client_id: "auth9-admin".to_string(),
            admin_client_secret: "test-secret".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        }
    }

    async fn setup_token_mock(mock_server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/realms/master/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-token",
                "expires_in": 300
            })))
            .mount(mock_server)
            .await;
    }

    #[test]
    fn test_extract_realm_settings_allow_registration_true() {
        let config = BrandingConfig {
            allow_registration: true,
            ..Default::default()
        };

        let settings = KeycloakSyncService::extract_realm_settings(&config);
        assert_eq!(settings.registration_allowed, Some(true));
    }

    #[test]
    fn test_extract_realm_settings_allow_registration_false() {
        let config = BrandingConfig {
            allow_registration: false,
            ..Default::default()
        };

        let settings = KeycloakSyncService::extract_realm_settings(&config);
        assert_eq!(settings.registration_allowed, Some(false));
    }

    #[test]
    fn test_extract_realm_settings_default() {
        let config = BrandingConfig::default();

        let settings = KeycloakSyncService::extract_realm_settings(&config);
        assert_eq!(settings.registration_allowed, Some(false));
        assert_eq!(settings.reset_password_allowed, None);
        assert_eq!(settings.ssl_required, None);
    }

    #[tokio::test]
    async fn test_sync_realm_settings_success() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        let settings = RealmUpdate {
            registration_allowed: Some(true),
            ..Default::default()
        };

        let result = service.sync_realm_settings(settings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sync_realm_settings_error() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "error": "access_denied"
            })))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        let settings = RealmUpdate {
            registration_allowed: Some(true),
            ..Default::default()
        };

        let result = service.sync_realm_settings(settings).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sync_branding_config_success() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        let branding = BrandingConfig {
            allow_registration: true,
            ..Default::default()
        };

        // This should not panic even on success
        service.sync_branding_config(&branding).await;
    }

    #[tokio::test]
    async fn test_sync_branding_config_error_does_not_propagate() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": "internal_error"
            })))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        let branding = BrandingConfig {
            allow_registration: true,
            ..Default::default()
        };

        // This should not panic even on error - errors are logged but not propagated
        service.sync_branding_config(&branding).await;
    }

    #[tokio::test]
    async fn test_sync_email_config_success() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        let smtp = SmtpServerConfig {
            host: Some("smtp.example.com".to_string()),
            port: Some("587".to_string()),
            from: Some("noreply@example.com".to_string()),
            from_display_name: Some("Auth9".to_string()),
            auth: Some("true".to_string()),
            user: Some("user@example.com".to_string()),
            password: Some("password".to_string()),
            ssl: Some("false".to_string()),
            starttls: Some("true".to_string()),
        };

        // This should not panic
        service.sync_email_config(Some(smtp)).await;
    }

    #[tokio::test]
    async fn test_sync_email_config_none_skips_sync() {
        let mock_server = MockServer::start().await;
        // Don't set up any mocks - sync should be skipped entirely

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        // This should not panic and should not make any HTTP requests
        service.sync_email_config(None).await;
    }

    #[tokio::test]
    async fn test_sync_email_config_error_does_not_propagate() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": "internal_error"
            })))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        let smtp = SmtpServerConfig {
            host: Some("smtp.example.com".to_string()),
            port: Some("587".to_string()),
            ..Default::default()
        };

        // This should not panic even on error - errors are logged but not propagated
        service.sync_email_config(Some(smtp)).await;
    }

    #[tokio::test]
    async fn test_sync_email_config_empty_clears_smtp() {
        let mock_server = MockServer::start().await;
        setup_token_mock(&mock_server).await;

        Mock::given(method("PUT"))
            .and(path("/admin/realms/test-realm"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let config = create_test_config(&mock_server.uri());
        let client = Arc::new(KeycloakClient::new(config));
        let service = KeycloakSyncService::new(client);

        // Empty config should be sent to clear SMTP settings
        let empty_smtp = SmtpServerConfig::default();
        service.sync_email_config(Some(empty_smtp)).await;
    }
}
