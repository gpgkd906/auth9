//! Keycloak sync service for Auth9 ↔ Keycloak state synchronization
//!
//! This service manages the synchronization of configuration between Auth9 and Keycloak.
//! When Auth9 settings change (e.g., branding configuration, email settings), this service
//! ensures the corresponding Keycloak realm settings are updated.

use crate::error::Result;
use crate::identity_engine::IdentityEngine;
use crate::keycloak::RealmUpdate;
use crate::keycloak::SmtpServerConfig;
use crate::models::branding::BrandingConfig;
use crate::models::password::PasswordPolicy;
use std::sync::Arc;
use tracing::{error, info};

/// Service for synchronizing Auth9 configuration with Keycloak realm settings
pub struct KeycloakSyncService {
    identity_engine: Arc<dyn IdentityEngine>,
}

impl KeycloakSyncService {
    /// Create a new KeycloakSyncService
    pub fn new(identity_engine: Arc<dyn IdentityEngine>) -> Self {
        Self { identity_engine }
    }

    /// Synchronize realm settings to Keycloak
    ///
    /// This method updates the Keycloak realm configuration to match the
    /// provided settings. Only non-None fields in the update will be applied.
    pub async fn sync_realm_settings(&self, settings: RealmUpdate) -> Result<()> {
        info!("Syncing realm settings to Keycloak: {:?}", settings);
        self.identity_engine.update_realm(&settings).await?;
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

    /// Convert a PasswordPolicy to a Keycloak password policy string
    ///
    /// Keycloak uses a specific format: `length(N) and upperCase(N) and ...`
    pub fn to_keycloak_policy_string(policy: &PasswordPolicy) -> String {
        let mut parts = Vec::new();

        parts.push(format!("length({})", policy.min_length));

        if policy.require_uppercase {
            parts.push("upperCase(1)".to_string());
        }
        if policy.require_lowercase {
            parts.push("lowerCase(1)".to_string());
        }
        if policy.require_numbers {
            parts.push("digits(1)".to_string());
        }
        if policy.require_symbols {
            parts.push("specialChars(1)".to_string());
        }
        if policy.history_count > 0 {
            parts.push(format!("passwordHistory({})", policy.history_count));
        }
        if policy.max_age_days > 0 {
            parts.push(format!(
                "forceExpiredPasswordChange({})",
                policy.max_age_days
            ));
        }

        parts.push("notUsername()".to_string());

        parts.join(" and ")
    }

    /// Sync password policy to Keycloak realm
    ///
    /// This method updates the Keycloak realm's password policy configuration
    /// and brute force protection settings.
    /// Errors are logged but not propagated to avoid blocking the main policy update flow.
    pub async fn sync_password_policy(&self, policy: &PasswordPolicy) {
        let policy_string = Self::to_keycloak_policy_string(policy);
        info!("Syncing password policy to Keycloak: {}", policy_string);

        let mut realm_update = RealmUpdate {
            password_policy: Some(policy_string),
            ..Default::default()
        };

        // Sync brute force protection settings
        if policy.lockout_threshold > 0 {
            realm_update.brute_force_protected = Some(true);
            realm_update.max_failure_wait_seconds =
                Some((policy.lockout_duration_mins * 60) as i32);
            realm_update.failure_factor = Some(policy.lockout_threshold as i32);
            realm_update.wait_increment_seconds = Some((policy.lockout_duration_mins * 60) as i32);
        } else {
            realm_update.brute_force_protected = Some(false);
        }

        if let Err(e) = self.sync_realm_settings(realm_update).await {
            error!("Failed to sync password policy to Keycloak: {}", e);
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
    use crate::error::AppError;
    use crate::identity_engine::{
        FederatedIdentityRepresentation, FederationBroker, IdentityClientStore,
        IdentityCredentialStore, IdentityEventSource, IdentityProviderRepresentation,
        IdentitySessionStore, IdentityUserStore,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct FakeIdentityEngine {
        failure: Option<String>,
        updates: Mutex<Vec<RealmUpdate>>,
    }

    impl FakeIdentityEngine {
        fn succeeds() -> Self {
            Self {
                failure: None,
                updates: Mutex::new(Vec::new()),
            }
        }

        fn fails(error: AppError) -> Self {
            Self {
                failure: Some(error.to_string()),
                updates: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl IdentityUserStore for FakeIdentityEngine {}

    #[async_trait]
    impl IdentityClientStore for FakeIdentityEngine {}

    #[async_trait]
    impl IdentityCredentialStore for FakeIdentityEngine {}

    #[async_trait]
    impl IdentityEventSource for FakeIdentityEngine {}

    #[async_trait]
    impl IdentitySessionStore for FakeIdentityEngine {
        async fn delete_user_session(&self, _session_id: &str) -> Result<()> {
            Ok(())
        }

        async fn logout_user(&self, _user_id: &str) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl FederationBroker for FakeIdentityEngine {
        async fn list_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>> {
            Ok(Vec::new())
        }

        async fn get_identity_provider(
            &self,
            _alias: &str,
        ) -> Result<IdentityProviderRepresentation> {
            Err(AppError::NotFound("not used".to_string()))
        }

        async fn create_identity_provider(
            &self,
            _provider: &IdentityProviderRepresentation,
        ) -> Result<()> {
            Ok(())
        }

        async fn update_identity_provider(
            &self,
            _alias: &str,
            _provider: &IdentityProviderRepresentation,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete_identity_provider(&self, _alias: &str) -> Result<()> {
            Ok(())
        }

        async fn get_user_federated_identities(
            &self,
            _user_id: &str,
        ) -> Result<Vec<FederatedIdentityRepresentation>> {
            Ok(Vec::new())
        }

        async fn remove_user_federated_identity(
            &self,
            _user_id: &str,
            _provider_alias: &str,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl IdentityEngine for FakeIdentityEngine {
        fn user_store(&self) -> &dyn IdentityUserStore {
            self
        }

        fn client_store(&self) -> &dyn IdentityClientStore {
            self
        }

        fn session_store(&self) -> &dyn IdentitySessionStore {
            self
        }

        fn credential_store(&self) -> &dyn IdentityCredentialStore {
            self
        }

        fn federation_broker(&self) -> &dyn FederationBroker {
            self
        }

        fn event_source(&self) -> &dyn IdentityEventSource {
            self
        }

        async fn update_realm(&self, settings: &RealmUpdate) -> Result<()> {
            self.updates.lock().unwrap().push(settings.clone());

            if let Some(message) = &self.failure {
                Err(AppError::Keycloak(message.clone()))
            } else {
                Ok(())
            }
        }
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
        let engine = Arc::new(FakeIdentityEngine::succeeds());
        let service = KeycloakSyncService::new(engine.clone());

        let settings = RealmUpdate {
            registration_allowed: Some(true),
            ..Default::default()
        };

        let result = service.sync_realm_settings(settings).await;
        assert!(result.is_ok());
        assert_eq!(engine.updates.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_sync_realm_settings_error() {
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::fails(
            crate::error::AppError::Keycloak("access_denied".to_string()),
        )));

        let settings = RealmUpdate {
            registration_allowed: Some(true),
            ..Default::default()
        };

        let result = service.sync_realm_settings(settings).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sync_branding_config_success() {
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::succeeds()));

        let branding = BrandingConfig {
            allow_registration: true,
            ..Default::default()
        };

        // This should not panic even on success
        service.sync_branding_config(&branding).await;
    }

    #[tokio::test]
    async fn test_sync_branding_config_error_does_not_propagate() {
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::fails(
            crate::error::AppError::Keycloak("internal_error".to_string()),
        )));

        let branding = BrandingConfig {
            allow_registration: true,
            ..Default::default()
        };

        // This should not panic even on error - errors are logged but not propagated
        service.sync_branding_config(&branding).await;
    }

    #[tokio::test]
    async fn test_sync_email_config_success() {
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::succeeds()));

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
        let engine = Arc::new(FakeIdentityEngine::succeeds());
        let service = KeycloakSyncService::new(engine.clone());

        // This should not panic and should not make any HTTP requests
        service.sync_email_config(None).await;
        assert!(engine.updates.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_sync_email_config_error_does_not_propagate() {
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::fails(
            crate::error::AppError::Keycloak("internal_error".to_string()),
        )));

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
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::succeeds()));

        // Empty config should be sent to clear SMTP settings
        let empty_smtp = SmtpServerConfig::default();
        service.sync_email_config(Some(empty_smtp)).await;
    }

    #[test]
    fn test_to_keycloak_policy_string_default() {
        let policy = PasswordPolicy::default();
        let result = KeycloakSyncService::to_keycloak_policy_string(&policy);
        assert!(result.contains("length(12)"));
        assert!(result.contains("upperCase(1)"));
        assert!(result.contains("lowerCase(1)"));
        assert!(result.contains("digits(1)"));
        assert!(result.contains("specialChars(1)"));
        assert!(result.contains("notUsername()"));
        assert!(result.contains("passwordHistory(5)"));
    }

    #[test]
    fn test_to_keycloak_policy_string_with_history() {
        let policy = PasswordPolicy {
            min_length: 12,
            require_uppercase: true,
            require_lowercase: true,
            require_numbers: true,
            require_symbols: true,
            history_count: 5,
            ..Default::default()
        };
        let result = KeycloakSyncService::to_keycloak_policy_string(&policy);
        assert!(result.contains("length(12)"));
        assert!(result.contains("specialChars(1)"));
        assert!(result.contains("passwordHistory(5)"));
        assert!(result.contains("notUsername()"));
    }

    #[test]
    fn test_to_keycloak_policy_string_minimal() {
        let policy = PasswordPolicy {
            min_length: 6,
            require_uppercase: false,
            require_lowercase: false,
            require_numbers: false,
            require_symbols: false,
            history_count: 0,
            ..Default::default()
        };
        let result = KeycloakSyncService::to_keycloak_policy_string(&policy);
        assert_eq!(result, "length(6) and notUsername()");
    }

    #[tokio::test]
    async fn test_sync_password_policy_success() {
        let engine = Arc::new(FakeIdentityEngine::succeeds());
        let service = KeycloakSyncService::new(engine.clone());

        let policy = PasswordPolicy {
            min_length: 12,
            history_count: 5,
            ..Default::default()
        };
        service.sync_password_policy(&policy).await;
        assert!(engine.updates.lock().unwrap()[0].password_policy.is_some());
    }

    #[tokio::test]
    async fn test_sync_password_policy_error_does_not_propagate() {
        let service = KeycloakSyncService::new(Arc::new(FakeIdentityEngine::fails(
            crate::error::AppError::Keycloak("internal_error".to_string()),
        )));

        let policy = PasswordPolicy::default();
        // Should not panic even on error
        service.sync_password_policy(&policy).await;
    }
}
