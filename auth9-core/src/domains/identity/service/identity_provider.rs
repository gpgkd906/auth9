//! Identity Provider service
//!
//! Manages social login and enterprise SSO identity providers.
//! IdP configuration is stored in Keycloak, Auth9 provides the management UI.

use crate::domain::{
    CreateIdentityProviderInput, CreateLinkedIdentityInput, IdentityProvider,
    IdentityProviderTemplate, LinkedIdentity, LinkedIdentityInfo, StringUuid,
    UpdateIdentityProviderInput,
};
use crate::error::{AppError, Result};
use crate::keycloak::{KeycloakClient, KeycloakIdentityProvider};
use crate::repository::{LinkedIdentityRepository, UserRepository};
use std::collections::HashMap;
use std::sync::Arc;
use validator::Validate;

pub struct IdentityProviderService<L: LinkedIdentityRepository, U: UserRepository> {
    linked_identity_repo: Arc<L>,
    user_repo: Arc<U>,
    keycloak: Arc<KeycloakClient>,
}

impl<L: LinkedIdentityRepository, U: UserRepository> IdentityProviderService<L, U> {
    pub fn new(
        linked_identity_repo: Arc<L>,
        user_repo: Arc<U>,
        keycloak: Arc<KeycloakClient>,
    ) -> Self {
        Self {
            linked_identity_repo,
            user_repo,
            keycloak,
        }
    }

    // ========================================================================
    // Identity Provider Management (via Keycloak)
    // ========================================================================

    /// List all identity providers
    pub async fn list_providers(&self) -> Result<Vec<IdentityProvider>> {
        let kc_providers = self.keycloak.list_identity_providers().await?;
        Ok(kc_providers.into_iter().map(Into::into).collect())
    }

    /// Get an identity provider by alias
    pub async fn get_provider(&self, alias: &str) -> Result<IdentityProvider> {
        let kc_provider = self.keycloak.get_identity_provider(alias).await?;
        Ok(kc_provider.into())
    }

    /// Create an identity provider
    pub async fn create_provider(
        &self,
        input: CreateIdentityProviderInput,
    ) -> Result<IdentityProvider> {
        input.validate()?;

        // Validate required config fields based on provider template
        if let Some(template) = IdentityProviderTemplate::find(&input.provider_id) {
            if let Err(missing) = template.validate_config(&input.config) {
                return Err(AppError::Validation(format!(
                    "Missing required config fields for '{}' provider: {}",
                    input.provider_id,
                    missing.join(", ")
                )));
            }
        }

        let kc_provider = KeycloakIdentityProvider {
            alias: input.alias.clone(),
            display_name: input.display_name,
            provider_id: input.provider_id,
            enabled: input.enabled,
            trust_email: input.trust_email,
            store_token: input.store_token,
            link_only: input.link_only,
            first_broker_login_flow_alias: None,
            config: input.config,
            extra: HashMap::new(),
        };

        self.keycloak.create_identity_provider(&kc_provider).await?;

        // Fetch the created provider
        self.get_provider(&input.alias).await
    }

    /// Update an identity provider
    pub async fn update_provider(
        &self,
        alias: &str,
        input: UpdateIdentityProviderInput,
    ) -> Result<IdentityProvider> {
        input.validate()?;

        // Get existing provider
        let existing = self.keycloak.get_identity_provider(alias).await?;

        // Merge updates (preserve extra Keycloak fields like internalId for round-trip)
        let updated = KeycloakIdentityProvider {
            alias: existing.alias,
            display_name: input.display_name.or(existing.display_name),
            provider_id: existing.provider_id,
            enabled: input.enabled.unwrap_or(existing.enabled),
            trust_email: input.trust_email.unwrap_or(existing.trust_email),
            store_token: input.store_token.unwrap_or(existing.store_token),
            link_only: input.link_only.unwrap_or(existing.link_only),
            first_broker_login_flow_alias: existing.first_broker_login_flow_alias,
            config: input.config.unwrap_or(existing.config),
            extra: existing.extra,
        };

        self.keycloak
            .update_identity_provider(alias, &updated)
            .await?;

        // Fetch the updated provider
        self.get_provider(alias).await
    }

    /// Delete an identity provider
    pub async fn delete_provider(&self, alias: &str) -> Result<()> {
        self.keycloak.delete_identity_provider(alias).await
    }

    /// Get available provider templates
    pub fn get_templates(&self) -> Vec<IdentityProviderTemplate> {
        IdentityProviderTemplate::all()
    }

    // ========================================================================
    // User Linked Identities
    // ========================================================================

    /// Get linked identities for a user
    pub async fn get_user_identities(
        &self,
        user_id: StringUuid,
    ) -> Result<Vec<LinkedIdentityInfo>> {
        let identities = self.linked_identity_repo.list_by_user(user_id).await?;
        Ok(identities.into_iter().map(Into::into).collect())
    }

    /// Unlink an identity from a user
    pub async fn unlink_identity(
        &self,
        user_id: StringUuid,
        identity_id: StringUuid,
    ) -> Result<()> {
        // Get the identity to verify ownership
        let identity = self
            .linked_identity_repo
            .find_by_id(identity_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Linked identity not found".to_string()))?;

        // Verify the identity belongs to the user
        if identity.user_id != user_id {
            return Err(AppError::Forbidden(
                "Cannot unlink another user's identity".to_string(),
            ));
        }

        // Get the user to get their Keycloak ID
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Remove from Keycloak
        self.keycloak
            .remove_user_federated_identity(&user.keycloak_id, &identity.provider_alias)
            .await?;

        // Remove from our database
        self.linked_identity_repo.delete(identity_id).await
    }

    /// Sync federated identities from Keycloak to our database
    ///
    /// Called after user login to keep our linked_identities table in sync
    pub async fn sync_user_identities(
        &self,
        user_id: StringUuid,
        keycloak_user_id: &str,
    ) -> Result<()> {
        // Get current identities from Keycloak
        let kc_identities = self
            .keycloak
            .get_user_federated_identities(keycloak_user_id)
            .await?;

        // Get current identities from our database
        let db_identities = self.linked_identity_repo.list_by_user(user_id).await?;

        // Build a set of existing identities for quick lookup
        let existing: HashMap<String, LinkedIdentity> = db_identities
            .into_iter()
            .map(|i| (format!("{}:{}", i.provider_alias, i.external_user_id), i))
            .collect();

        // Add any new identities from Keycloak
        for kc_identity in kc_identities {
            let key = format!("{}:{}", kc_identity.identity_provider, kc_identity.user_id);
            if !existing.contains_key(&key) {
                let input = CreateLinkedIdentityInput {
                    user_id,
                    provider_type: kc_identity.identity_provider.clone(),
                    provider_alias: kc_identity.identity_provider,
                    external_user_id: kc_identity.user_id,
                    external_email: kc_identity.user_name,
                };
                let _ = self.linked_identity_repo.create(&input).await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::linked_identity::MockLinkedIdentityRepository;
    use crate::repository::user::MockUserRepository;
    use mockall::predicate::*;

    #[test]
    fn test_provider_templates() {
        let templates = IdentityProviderTemplate::all();
        assert!(templates.len() >= 5);

        let google = templates.iter().find(|t| t.provider_id == "google");
        assert!(google.is_some());
        assert!(google
            .unwrap()
            .required_config
            .contains(&"clientId".to_string()));
    }

    #[test]
    fn test_provider_templates_github() {
        let templates = IdentityProviderTemplate::all();
        let github = templates.iter().find(|t| t.provider_id == "github");
        assert!(github.is_some());
        let github = github.unwrap();
        assert!(github.required_config.contains(&"clientId".to_string()));
        assert!(github.required_config.contains(&"clientSecret".to_string()));
    }

    #[test]
    fn test_provider_templates_oidc() {
        let templates = IdentityProviderTemplate::all();
        let oidc = templates.iter().find(|t| t.provider_id == "oidc");
        assert!(oidc.is_some());
        let oidc = oidc.unwrap();
        assert!(oidc
            .required_config
            .contains(&"authorizationUrl".to_string()));
        assert!(oidc.required_config.contains(&"tokenUrl".to_string()));
    }

    #[test]
    fn test_provider_templates_saml() {
        let templates = IdentityProviderTemplate::all();
        let saml = templates.iter().find(|t| t.provider_id == "saml");
        assert!(saml.is_some());
    }

    #[tokio::test]
    async fn test_get_user_identities_empty() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();

        linked_mock
            .expect_list_by_user()
            .with(eq(user_id))
            .returning(|_| Ok(vec![]));

        let keycloak = create_test_keycloak_client();
        let service = IdentityProviderService::new(
            Arc::new(linked_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let identities = service.get_user_identities(user_id).await.unwrap();
        assert!(identities.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_identities_multiple() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();

        linked_mock
            .expect_list_by_user()
            .with(eq(user_id))
            .returning(|_| {
                Ok(vec![
                    LinkedIdentity {
                        provider_type: "google".to_string(),
                        provider_alias: "google".to_string(),
                        external_email: Some("user@gmail.com".to_string()),
                        ..Default::default()
                    },
                    LinkedIdentity {
                        provider_type: "github".to_string(),
                        provider_alias: "github".to_string(),
                        external_email: Some("user@github.com".to_string()),
                        ..Default::default()
                    },
                ])
            });

        let keycloak = create_test_keycloak_client();
        let service = IdentityProviderService::new(
            Arc::new(linked_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let identities = service.get_user_identities(user_id).await.unwrap();
        assert_eq!(identities.len(), 2);
        assert_eq!(identities[0].provider_type, "google");
        assert_eq!(identities[1].provider_type, "github");
    }

    #[tokio::test]
    async fn test_unlink_identity_not_found() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let identity_id = StringUuid::new_v4();

        linked_mock
            .expect_find_by_id()
            .with(eq(identity_id))
            .returning(|_| Ok(None));

        let keycloak = create_test_keycloak_client();
        let service = IdentityProviderService::new(
            Arc::new(linked_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.unlink_identity(user_id, identity_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_unlink_identity_wrong_user() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let other_user_id = StringUuid::new_v4();
        let identity_id = StringUuid::new_v4();

        linked_mock
            .expect_find_by_id()
            .with(eq(identity_id))
            .returning(move |_| {
                Ok(Some(LinkedIdentity {
                    id: identity_id,
                    user_id: other_user_id, // Different user
                    provider_type: "google".to_string(),
                    provider_alias: "google".to_string(),
                    ..Default::default()
                }))
            });

        let keycloak = create_test_keycloak_client();
        let service = IdentityProviderService::new(
            Arc::new(linked_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.unlink_identity(user_id, identity_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn test_unlink_identity_user_not_found() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let mut user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let identity_id = StringUuid::new_v4();

        linked_mock
            .expect_find_by_id()
            .with(eq(identity_id))
            .returning(move |_| {
                Ok(Some(LinkedIdentity {
                    id: identity_id,
                    user_id,
                    provider_type: "google".to_string(),
                    provider_alias: "google".to_string(),
                    ..Default::default()
                }))
            });

        user_mock
            .expect_find_by_id()
            .with(eq(user_id))
            .returning(|_| Ok(None));

        let keycloak = create_test_keycloak_client();
        let service = IdentityProviderService::new(
            Arc::new(linked_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.unlink_identity(user_id, identity_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_templates() {
        let linked_mock = MockLinkedIdentityRepository::new();
        let user_mock = MockUserRepository::new();

        let keycloak = create_test_keycloak_client();
        let service = IdentityProviderService::new(
            Arc::new(linked_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let templates = service.get_templates();
        assert!(!templates.is_empty());

        // Check that Google template exists
        assert!(templates.iter().any(|t| t.provider_id == "google"));
        // Check that GitHub template exists
        assert!(templates.iter().any(|t| t.provider_id == "github"));
        // Check that SAML template exists
        assert!(templates.iter().any(|t| t.provider_id == "saml"));
    }

    #[test]
    fn test_linked_identity_into_info() {
        let identity = LinkedIdentity {
            id: StringUuid::new_v4(),
            user_id: StringUuid::new_v4(),
            provider_type: "google".to_string(),
            provider_alias: "google".to_string(),
            external_user_id: "google-123".to_string(),
            external_email: Some("user@gmail.com".to_string()),
            linked_at: chrono::Utc::now(),
        };

        let info: LinkedIdentityInfo = identity.into();
        assert_eq!(info.provider_type, "google");
        assert_eq!(info.external_email, Some("user@gmail.com".to_string()));
    }

    // Helper to create a test KeycloakClient
    fn create_test_keycloak_client() -> KeycloakClient {
        use crate::config::KeycloakConfig;
        KeycloakClient::new(KeycloakConfig {
            url: "http://localhost:8081".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
            event_source: "redis_stream".to_string(),
            event_stream_key: "auth9:keycloak:events".to_string(),
            event_stream_group: "auth9-core".to_string(),
            event_stream_consumer: "auth9-core-1".to_string(),
        })
    }
}
