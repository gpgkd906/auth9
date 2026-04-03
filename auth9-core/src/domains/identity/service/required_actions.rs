//! Required actions service.
//!
//! Manages pending/required actions for users (verify email, force password
//! update, complete profile) through the IdentityEngine abstraction.

use crate::error::Result;
use crate::identity_engine::IdentityEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Known action types that map to portal pages.
pub const ACTION_VERIFY_EMAIL: &str = "verify_email";
pub const ACTION_UPDATE_PASSWORD: &str = "update_password"; // pragma: allowlist secret
pub const ACTION_COMPLETE_PROFILE: &str = "complete_profile";
pub const ACTION_CONFIGURE_TOTP: &str = "CONFIGURE_TOTP";

/// Response object for a pending action with its redirect URL.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingActionResponse {
    pub id: String,
    pub action_type: String,
    pub redirect_url: String,
}

pub struct RequiredActionService {
    identity_engine: Arc<dyn IdentityEngine>,
}

impl RequiredActionService {
    pub fn new(identity_engine: Arc<dyn IdentityEngine>) -> Self {
        Self { identity_engine }
    }

    /// Get all pending actions for a user, with redirect URLs.
    pub async fn get_pending_actions(&self, user_id: &str) -> Result<Vec<PendingActionResponse>> {
        let actions = self
            .identity_engine
            .action_store()
            .get_pending_actions(user_id)
            .await?;

        Ok(actions
            .into_iter()
            .map(|a| PendingActionResponse {
                id: a.id,
                redirect_url: Self::action_redirect_url(&a.action_type),
                action_type: a.action_type,
            })
            .collect())
    }

    /// Check pending actions and also auto-detect implied actions
    /// (e.g., temporary password → force update password, MFA enabled
    /// without credential → force TOTP setup).
    pub async fn check_post_login_actions(
        &self,
        identity_subject: &str,
        mfa_enabled: bool,
        has_mfa_credential: bool,
    ) -> Result<Vec<PendingActionResponse>> {
        let mut actions = self.get_pending_actions(identity_subject).await?;

        // Check if password is temporary → auto-create update_password action
        if !actions
            .iter()
            .any(|a| a.action_type == ACTION_UPDATE_PASSWORD)
            && self.is_password_temporary(identity_subject).await?
        {
            let id = self
                .identity_engine
                .action_store()
                .create_action(identity_subject, ACTION_UPDATE_PASSWORD, None)
                .await?;
            actions.push(PendingActionResponse {
                id,
                action_type: ACTION_UPDATE_PASSWORD.to_string(),
                redirect_url: Self::action_redirect_url(ACTION_UPDATE_PASSWORD),
            });
        }

        // MFA enabled but no credential configured → force TOTP setup
        if mfa_enabled
            && !has_mfa_credential
            && !actions
                .iter()
                .any(|a| a.action_type == ACTION_CONFIGURE_TOTP)
        {
            let id = self
                .identity_engine
                .action_store()
                .create_action(identity_subject, ACTION_CONFIGURE_TOTP, None)
                .await?;
            actions.push(PendingActionResponse {
                id,
                action_type: ACTION_CONFIGURE_TOTP.to_string(),
                redirect_url: Self::action_redirect_url(ACTION_CONFIGURE_TOTP),
            });
        }

        Ok(actions)
    }

    /// Complete a pending action.
    pub async fn complete_action(&self, action_id: &str) -> Result<()> {
        self.identity_engine
            .action_store()
            .complete_action(action_id)
            .await
    }

    /// Create a new pending action for a user.
    pub async fn create_action(
        &self,
        user_id: &str,
        action_type: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<String> {
        self.identity_engine
            .action_store()
            .create_action(user_id, action_type, metadata)
            .await
    }

    /// Cancel a pending action.
    pub async fn cancel_action(&self, action_id: &str) -> Result<()> {
        self.identity_engine
            .action_store()
            .cancel_action(action_id)
            .await
    }

    /// Map action type to portal redirect URL.
    fn action_redirect_url(action_type: &str) -> String {
        match action_type {
            ACTION_VERIFY_EMAIL => "/verify-email".to_string(),
            ACTION_UPDATE_PASSWORD => "/force-update-password".to_string(),
            ACTION_COMPLETE_PROFILE => "/complete-profile".to_string(),
            ACTION_CONFIGURE_TOTP => "/mfa/setup-totp".to_string(),
            other => format!("/pending-action?type={}", other),
        }
    }

    /// Check if the user's password credential has `temporary: true`.
    async fn is_password_temporary(&self, user_id: &str) -> Result<bool> {
        self.identity_engine
            .credential_store()
            .is_password_temporary(user_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_engine::{
        FederationBroker, IdentityActionStore, IdentityClientStore, IdentityCredentialStore,
        IdentityEngine, IdentityEventSource, IdentitySessionStore, IdentityUserStore,
        IdentityVerificationStore, PendingActionInfo, RealmSettingsUpdate,
    };
    use std::sync::Mutex;

    // -- Minimal mock stores for testing check_post_login_actions --

    struct MockActionStore {
        pending: Vec<PendingActionInfo>,
        next_id: String,
        create_called: Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl IdentityActionStore for MockActionStore {
        async fn get_pending_actions(&self, _user_id: &str) -> Result<Vec<PendingActionInfo>> {
            Ok(self.pending.clone())
        }
        async fn create_action(
            &self,
            _user_id: &str,
            action_type: &str,
            _metadata: Option<serde_json::Value>,
        ) -> Result<String> {
            self.create_called
                .lock()
                .unwrap()
                .push(action_type.to_string());
            Ok(self.next_id.clone())
        }
        async fn complete_action(&self, _action_id: &str) -> Result<()> {
            Ok(())
        }
        async fn cancel_action(&self, _action_id: &str) -> Result<()> {
            Ok(())
        }
    }

    struct MockCredentialStore {
        password_temporary: bool,
    }

    #[async_trait::async_trait]
    impl IdentityCredentialStore for MockCredentialStore {
        async fn list_user_credentials(
            &self,
            _user_id: &str,
        ) -> Result<Vec<crate::identity_engine::IdentityCredentialRepresentation>> {
            Ok(Vec::new())
        }
        async fn remove_totp_credentials(&self, _user_id: &str) -> Result<()> {
            Ok(())
        }
        async fn list_webauthn_credentials(
            &self,
            _user_id: &str,
        ) -> Result<Vec<crate::identity_engine::IdentityCredentialRepresentation>> {
            Ok(Vec::new())
        }
        async fn delete_user_credential(
            &self,
            _user_id: &str,
            _credential_id: &str,
        ) -> Result<()> {
            Ok(())
        }
        async fn is_password_temporary(&self, _user_id: &str) -> Result<bool> {
            Ok(self.password_temporary)
        }
    }

    struct MockEngine {
        action_store: MockActionStore,
        credential_store: MockCredentialStore,
    }

    // Stub traits that won't be called
    struct StubUserStore;
    #[async_trait::async_trait]
    impl IdentityUserStore for StubUserStore {
        async fn create_user(
            &self,
            _: &crate::identity_engine::IdentityUserCreateInput,
        ) -> Result<String> {
            unimplemented!()
        }
        async fn get_user(
            &self,
            _: &str,
        ) -> Result<crate::identity_engine::IdentityUserRepresentation> {
            unimplemented!()
        }
        async fn update_user(
            &self,
            _: &str,
            _: &crate::identity_engine::IdentityUserUpdateInput,
        ) -> Result<()> {
            unimplemented!()
        }
        async fn delete_user(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn set_user_password(&self, _: &str, _: &str, _: bool) -> Result<()> {
            unimplemented!()
        }
        async fn admin_set_user_password(&self, _: &str, _: &str, _: bool) -> Result<()> {
            unimplemented!()
        }
        async fn validate_user_password(&self, _: &str, _: &str) -> Result<bool> {
            unimplemented!()
        }
        async fn get_user_password_hash(&self, _: &str) -> Result<Option<String>> {
            unimplemented!()
        }
    }
    struct StubClientStore;
    #[async_trait::async_trait]
    impl IdentityClientStore for StubClientStore {
        async fn create_oidc_client(
            &self,
            _: &crate::identity_engine::OidcClientRepresentation,
        ) -> Result<String> {
            unimplemented!()
        }
        async fn get_client_secret(&self, _: &str) -> Result<String> {
            unimplemented!()
        }
        async fn regenerate_client_secret(&self, _: &str) -> Result<String> {
            unimplemented!()
        }
        async fn get_client_uuid_by_client_id(&self, _: &str) -> Result<String> {
            unimplemented!()
        }
        async fn get_client_by_client_id(
            &self,
            _: &str,
        ) -> Result<crate::identity_engine::OidcClientRepresentation> {
            unimplemented!()
        }
        async fn update_oidc_client(
            &self,
            _: &str,
            _: &crate::identity_engine::OidcClientRepresentation,
        ) -> Result<()> {
            unimplemented!()
        }
        async fn delete_oidc_client(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn create_saml_client(
            &self,
            _: &crate::identity_engine::IdentitySamlClientRepresentation,
        ) -> Result<String> {
            unimplemented!()
        }
        async fn update_saml_client(
            &self,
            _: &str,
            _: &crate::identity_engine::IdentitySamlClientRepresentation,
        ) -> Result<()> {
            unimplemented!()
        }
        async fn delete_saml_client(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn get_saml_idp_descriptor(&self) -> Result<String> {
            unimplemented!()
        }
        async fn get_active_signing_certificate(&self) -> Result<String> {
            unimplemented!()
        }
        fn saml_sso_url(&self) -> String {
            unimplemented!()
        }
    }
    struct StubSessionStore;
    #[async_trait::async_trait]
    impl IdentitySessionStore for StubSessionStore {
        async fn delete_user_session(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn logout_user(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
    }
    struct StubFederationBroker;
    #[async_trait::async_trait]
    impl FederationBroker for StubFederationBroker {
        async fn list_identity_providers(
            &self,
        ) -> Result<Vec<crate::identity_engine::IdentityProviderRepresentation>> {
            unimplemented!()
        }
        async fn get_identity_provider(
            &self,
            _: &str,
        ) -> Result<crate::identity_engine::IdentityProviderRepresentation> {
            unimplemented!()
        }
        async fn create_identity_provider(
            &self,
            _: &crate::identity_engine::IdentityProviderRepresentation,
        ) -> Result<()> {
            unimplemented!()
        }
        async fn update_identity_provider(
            &self,
            _: &str,
            _: &crate::identity_engine::IdentityProviderRepresentation,
        ) -> Result<()> {
            unimplemented!()
        }
        async fn delete_identity_provider(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
    }
    struct StubEventSource;
    #[async_trait::async_trait]
    impl IdentityEventSource for StubEventSource {}
    struct StubVerificationStore;
    #[async_trait::async_trait]
    impl IdentityVerificationStore for StubVerificationStore {
        async fn get_verification_status(&self, _: &str) -> Result<bool> {
            unimplemented!()
        }
        async fn set_email_verified(&self, _: &str, _: bool) -> Result<()> {
            unimplemented!()
        }
        async fn create_verification_token(
            &self,
            _: &str,
            _: &str,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<crate::identity_engine::VerificationTokenInfo> {
            unimplemented!()
        }
        async fn find_valid_token(
            &self,
            _: &str,
        ) -> Result<Option<crate::identity_engine::VerificationTokenInfo>> {
            unimplemented!()
        }
        async fn mark_token_used(&self, _: &str) -> Result<()> {
            unimplemented!()
        }
        async fn invalidate_user_tokens(&self, _: &str) -> Result<u64> {
            unimplemented!()
        }
    }

    #[async_trait::async_trait]
    impl IdentityEngine for MockEngine {
        fn user_store(&self) -> &dyn IdentityUserStore {
            &StubUserStore
        }
        fn client_store(&self) -> &dyn IdentityClientStore {
            &StubClientStore
        }
        fn session_store(&self) -> &dyn IdentitySessionStore {
            &StubSessionStore
        }
        fn credential_store(&self) -> &dyn IdentityCredentialStore {
            &self.credential_store
        }
        fn federation_broker(&self) -> &dyn FederationBroker {
            &StubFederationBroker
        }
        fn event_source(&self) -> &dyn IdentityEventSource {
            &StubEventSource
        }
        fn action_store(&self) -> &dyn IdentityActionStore {
            &self.action_store
        }
        fn verification_store(&self) -> &dyn IdentityVerificationStore {
            &StubVerificationStore
        }
        async fn update_realm(&self, _: &RealmSettingsUpdate) -> Result<()> {
            unimplemented!()
        }
    }

    fn make_service(
        pending: Vec<PendingActionInfo>,
        password_temporary: bool,
    ) -> (RequiredActionService, Arc<MockEngine>) {
        let engine = Arc::new(MockEngine {
            action_store: MockActionStore {
                pending,
                next_id: "new-action-id".to_string(),
                create_called: Mutex::new(Vec::new()),
            },
            credential_store: MockCredentialStore {
                password_temporary,
            },
        });
        let service = RequiredActionService::new(engine.clone() as Arc<dyn IdentityEngine>);
        (service, engine)
    }

    #[test]
    fn action_redirect_url_known_types() {
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_VERIFY_EMAIL),
            "/verify-email"
        );
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_UPDATE_PASSWORD),
            "/force-update-password"
        );
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_COMPLETE_PROFILE),
            "/complete-profile"
        );
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_CONFIGURE_TOTP),
            "/mfa/setup-totp"
        );
    }

    #[test]
    fn action_redirect_url_unknown_type() {
        let url = RequiredActionService::action_redirect_url("custom_action");
        assert_eq!(url, "/pending-action?type=custom_action");
    }

    #[test]
    fn pending_action_response_serialization() {
        let resp = PendingActionResponse {
            id: "act-1".to_string(),
            action_type: "verify_email".to_string(),
            redirect_url: "/verify-email".to_string(),
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["action_type"], "verify_email");
        assert_eq!(json["redirect_url"], "/verify-email");
    }

    // -- Async tests for check_post_login_actions --

    #[tokio::test]
    async fn check_creates_configure_totp_when_mfa_enabled_no_credential() {
        let (service, engine) = make_service(vec![], false);
        let actions = service
            .check_post_login_actions("user-1", true, false)
            .await
            .unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, ACTION_CONFIGURE_TOTP);
        assert_eq!(actions[0].redirect_url, "/mfa/setup-totp");
        assert_eq!(actions[0].id, "new-action-id");
        let created = engine.action_store.create_called.lock().unwrap();
        assert_eq!(created.as_slice(), &[ACTION_CONFIGURE_TOTP]);
    }

    #[tokio::test]
    async fn check_skips_configure_totp_when_has_credential() {
        let (service, engine) = make_service(vec![], false);
        let actions = service
            .check_post_login_actions("user-1", true, true)
            .await
            .unwrap();
        assert!(actions.is_empty());
        let created = engine.action_store.create_called.lock().unwrap();
        assert!(created.is_empty());
    }

    #[tokio::test]
    async fn check_skips_configure_totp_when_mfa_disabled() {
        let (service, engine) = make_service(vec![], false);
        let actions = service
            .check_post_login_actions("user-1", false, false)
            .await
            .unwrap();
        assert!(actions.is_empty());
        let created = engine.action_store.create_called.lock().unwrap();
        assert!(created.is_empty());
    }

    #[tokio::test]
    async fn check_no_duplicate_configure_totp() {
        let existing = vec![PendingActionInfo {
            id: "existing-action".to_string(),
            action_type: ACTION_CONFIGURE_TOTP.to_string(),
            metadata: None,
            created_at: chrono::Utc::now(),
        }];
        let (service, engine) = make_service(existing, false);
        let actions = service
            .check_post_login_actions("user-1", true, false)
            .await
            .unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, "existing-action");
        let created = engine.action_store.create_called.lock().unwrap();
        assert!(created.is_empty());
    }

    #[tokio::test]
    async fn check_both_password_and_totp_actions() {
        let (service, _engine) = make_service(vec![], true);
        let actions = service
            .check_post_login_actions("user-1", true, false)
            .await
            .unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].action_type, ACTION_UPDATE_PASSWORD);
        assert_eq!(actions[1].action_type, ACTION_CONFIGURE_TOTP);
    }
}
