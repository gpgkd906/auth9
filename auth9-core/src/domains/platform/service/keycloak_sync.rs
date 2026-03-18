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
        FederationBroker, IdentityClientStore, IdentityCredentialRepresentation,
        IdentityCredentialStore, IdentityEventSource, IdentityProviderRepresentation,
        IdentitySamlClientRepresentation, IdentitySessionStore, IdentityUserCreateInput,
        IdentityUserRepresentation, IdentityUserStore, IdentityUserUpdateInput,
    };
    use crate::keycloak::KeycloakOidcClient;
    use async_trait::async_trait;
    use std::collections::HashMap;
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
    impl IdentityUserStore for FakeIdentityEngine {
        async fn create_user(&self, _input: &IdentityUserCreateInput) -> Result<String> {
            Ok("user-1".to_string())
        }

        async fn get_user(&self, user_id: &str) -> Result<IdentityUserRepresentation> {
            Ok(IdentityUserRepresentation {
                id: Some(user_id.to_string()),
                username: user_id.to_string(),
                email: None,
                first_name: None,
                last_name: None,
                enabled: true,
                email_verified: false,
                attributes: HashMap::new(),
            })
        }

        async fn update_user(
            &self,
            _user_id: &str,
            _input: &IdentityUserUpdateInput,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete_user(&self, _user_id: &str) -> Result<()> {
            Ok(())
        }

        async fn set_user_password(
            &self,
            _user_id: &str,
            _password: &str,
            _temporary: bool,
        ) -> Result<()> {
            Ok(())
        }

        async fn admin_set_user_password(
            &self,
            _user_id: &str,
            _password: &str,
            _temporary: bool,
        ) -> Result<()> {
            Ok(())
        }

        async fn validate_user_password(&self, _user_id: &str, _password: &str) -> Result<bool> {
            Ok(true)
        }
    }

    #[async_trait]
    impl IdentityClientStore for FakeIdentityEngine {
        async fn create_oidc_client(&self, _client: &KeycloakOidcClient) -> Result<String> {
            Ok("client-1".to_string())
        }

        async fn get_client_secret(&self, _client_uuid: &str) -> Result<String> {
            Ok("secret".to_string())
        }

        async fn regenerate_client_secret(&self, _client_uuid: &str) -> Result<String> {
            Ok("secret".to_string())
        }

        async fn get_client_uuid_by_client_id(&self, client_id: &str) -> Result<String> {
            Ok(format!("uuid-{client_id}"))
        }

        async fn get_client_by_client_id(&self, client_id: &str) -> Result<KeycloakOidcClient> {
            Ok(KeycloakOidcClient {
                id: Some(format!("uuid-{client_id}")),
                client_id: client_id.to_string(),
                name: None,
                enabled: true,
                protocol: "openid-connect".to_string(),
                base_url: None,
                root_url: None,
                admin_url: None,
                redirect_uris: Vec::new(),
                web_origins: Vec::new(),
                attributes: None,
                public_client: false,
                secret: None,
            })
        }

        async fn update_oidc_client(
            &self,
            _client_uuid: &str,
            _client: &KeycloakOidcClient,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete_oidc_client(&self, _client_uuid: &str) -> Result<()> {
            Ok(())
        }

        async fn create_saml_client(
            &self,
            _client: &IdentitySamlClientRepresentation,
        ) -> Result<String> {
            Ok("saml-client-1".to_string())
        }

        async fn update_saml_client(
            &self,
            _client_uuid: &str,
            _client: &IdentitySamlClientRepresentation,
        ) -> Result<()> {
            Ok(())
        }

        async fn delete_saml_client(&self, _client_uuid: &str) -> Result<()> {
            Ok(())
        }

        async fn get_saml_idp_descriptor(&self) -> Result<String> {
            Ok("<EntityDescriptor />".to_string())
        }

        async fn get_active_signing_certificate(&self) -> Result<String> {
            Ok("cert-base64".to_string())
        }

        fn saml_sso_url(&self) -> String {
            "http://localhost:8080/realms/auth9/protocol/saml".to_string()
        }
    }

    #[async_trait]
    impl IdentityCredentialStore for FakeIdentityEngine {
        async fn list_user_credentials(
            &self,
            _user_id: &str,
        ) -> Result<Vec<IdentityCredentialRepresentation>> {
            Ok(Vec::new())
        }

        async fn remove_totp_credentials(&self, _user_id: &str) -> Result<()> {
            Ok(())
        }

        async fn list_webauthn_credentials(
            &self,
            _user_id: &str,
        ) -> Result<Vec<IdentityCredentialRepresentation>> {
            Ok(Vec::new())
        }

        async fn delete_user_credential(&self, _user_id: &str, _credential_id: &str) -> Result<()> {
            Ok(())
        }
    }

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
    }

    #[async_trait]
    impl crate::identity_engine::IdentityActionStore for FakeIdentityEngine {
        async fn get_pending_actions(
            &self,
            _user_id: &str,
        ) -> Result<Vec<crate::identity_engine::PendingActionInfo>> {
            Ok(Vec::new())
        }

        async fn create_action(
            &self,
            _user_id: &str,
            _action_type: &str,
            _metadata: Option<serde_json::Value>,
        ) -> Result<String> {
            Ok("fake-action".to_string())
        }

        async fn complete_action(&self, _action_id: &str) -> Result<()> {
            Ok(())
        }

        async fn cancel_action(&self, _action_id: &str) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl crate::identity_engine::IdentityVerificationStore for FakeIdentityEngine {
        async fn get_verification_status(&self, _user_id: &str) -> Result<bool> {
            Ok(true)
        }

        async fn set_email_verified(&self, _user_id: &str, _verified: bool) -> Result<()> {
            Ok(())
        }

        async fn create_verification_token(
            &self,
            _user_id: &str,
            _token_hash: &str,
            _expires_at: chrono::DateTime<chrono::Utc>,
        ) -> Result<crate::identity_engine::VerificationTokenInfo> {
            Ok(crate::identity_engine::VerificationTokenInfo {
                id: "fake-token".to_string(),
                user_id: "fake-user".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
                used_at: None,
                created_at: chrono::Utc::now(),
            })
        }

        async fn find_valid_token(
            &self,
            _token_hash: &str,
        ) -> Result<Option<crate::identity_engine::VerificationTokenInfo>> {
            Ok(None)
        }

        async fn mark_token_used(&self, _token_id: &str) -> Result<()> {
            Ok(())
        }

        async fn invalidate_user_tokens(&self, _user_id: &str) -> Result<u64> {
            Ok(0)
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

        fn action_store(&self) -> &dyn crate::identity_engine::IdentityActionStore {
            self
        }

        fn verification_store(&self) -> &dyn crate::identity_engine::IdentityVerificationStore {
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
