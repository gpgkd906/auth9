//! Identity Provider service
//!
//! Manages social login and enterprise SSO identity providers.
//! Auth9 owns linked_identities as primary data — no Keycloak federated identity API calls.

use crate::error::{AppError, Result};
use crate::identity_engine::{FederationBroker, IdentityProviderRepresentation};
use crate::models::common::StringUuid;
use crate::models::identity_provider::{
    CreateIdentityProviderInput, IdentityProvider, IdentityProviderTemplate,
    UpdateIdentityProviderInput,
};
use crate::models::linked_identity::{
    CreateLinkedIdentityInput, LinkedIdentity, LinkedIdentityInfo,
};
use crate::repository::LinkedIdentityRepository;
use std::collections::HashMap;
use std::sync::Arc;
use validator::Validate;

pub struct IdentityProviderService<L: LinkedIdentityRepository> {
    linked_identity_repo: Arc<L>,
    federation_broker: Arc<dyn FederationBroker>,
}

impl<L: LinkedIdentityRepository> IdentityProviderService<L> {
    pub fn new(
        linked_identity_repo: Arc<L>,
        federation_broker: Arc<dyn FederationBroker>,
    ) -> Self {
        Self {
            linked_identity_repo,
            federation_broker,
        }
    }

    // ========================================================================
    // Identity Provider Management (via identity engine adapter)
    // ========================================================================

    /// List all identity providers
    pub async fn list_providers(&self) -> Result<Vec<IdentityProvider>> {
        let kc_providers = self.federation_broker.list_identity_providers().await?;
        Ok(kc_providers.into_iter().map(Into::into).collect())
    }

    /// Get an identity provider by alias
    pub async fn get_provider(&self, alias: &str) -> Result<IdentityProvider> {
        let kc_provider = self.federation_broker.get_identity_provider(alias).await?;
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

        let provider = IdentityProviderRepresentation {
            alias: input.alias.clone(),
            display_name: input.display_name,
            provider_id: input.provider_id,
            enabled: input.enabled,
            trust_email: input.trust_email,
            store_token: input.store_token,
            link_only: input.link_only,
            first_login_policy: input.first_login_policy,
            first_broker_login_flow_alias: None,
            config: input.config,
            extra: HashMap::new(),
        };

        self.federation_broker
            .create_identity_provider(&provider)
            .await?;

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
        let existing = self.federation_broker.get_identity_provider(alias).await?;

        // Merge updates (preserve extra Keycloak fields like internalId for round-trip)
        let updated = IdentityProviderRepresentation {
            alias: existing.alias,
            display_name: input.display_name.or(existing.display_name),
            provider_id: existing.provider_id,
            enabled: input.enabled.unwrap_or(existing.enabled),
            trust_email: input.trust_email.unwrap_or(existing.trust_email),
            store_token: input.store_token.unwrap_or(existing.store_token),
            link_only: input.link_only.unwrap_or(existing.link_only),
            first_login_policy: input.first_login_policy.unwrap_or(existing.first_login_policy),
            first_broker_login_flow_alias: existing.first_broker_login_flow_alias,
            config: input.config.unwrap_or(existing.config),
            extra: existing.extra,
        };

        self.federation_broker
            .update_identity_provider(alias, &updated)
            .await?;

        // Fetch the updated provider
        self.get_provider(alias).await
    }

    /// Delete an identity provider
    pub async fn delete_provider(&self, alias: &str) -> Result<()> {
        self.federation_broker.delete_identity_provider(alias).await
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

        // Remove from our database (Auth9 is sole owner of linked_identities)
        self.linked_identity_repo.delete(identity_id).await
    }

    /// Create a linked identity directly
    pub async fn create_linked_identity(
        &self,
        input: &CreateLinkedIdentityInput,
    ) -> Result<LinkedIdentity> {
        self.linked_identity_repo.create(input).await
    }

    /// Find a linked identity by provider alias and external user ID
    pub async fn find_linked_identity(
        &self,
        provider_alias: &str,
        external_user_id: &str,
    ) -> Result<Option<LinkedIdentity>> {
        self.linked_identity_repo
            .find_by_provider(provider_alias, external_user_id)
            .await
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_engine::adapters::keycloak::KeycloakFederationBrokerAdapter;
    use crate::keycloak::KeycloakClient;
    use crate::repository::linked_identity::MockLinkedIdentityRepository;
    use mockall::predicate::*;
    use std::sync::Arc;

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
        let user_id = StringUuid::new_v4();

        linked_mock
            .expect_list_by_user()
            .with(eq(user_id))
            .returning(|_| Ok(vec![]));

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let identities = service.get_user_identities(user_id).await.unwrap();
        assert!(identities.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_identities_multiple() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
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

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let identities = service.get_user_identities(user_id).await.unwrap();
        assert_eq!(identities.len(), 2);
        assert_eq!(identities[0].provider_type, "google");
        assert_eq!(identities[1].provider_type, "github");
    }

    #[tokio::test]
    async fn test_unlink_identity_not_found() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_id = StringUuid::new_v4();
        let identity_id = StringUuid::new_v4();

        linked_mock
            .expect_find_by_id()
            .with(eq(identity_id))
            .returning(|_| Ok(None));

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let result = service.unlink_identity(user_id, identity_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_unlink_identity_wrong_user() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
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

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let result = service.unlink_identity(user_id, identity_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn test_unlink_identity_success() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
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

        linked_mock
            .expect_delete()
            .with(eq(identity_id))
            .returning(|_| Ok(()));

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let result = service.unlink_identity(user_id, identity_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_templates() {
        let linked_mock = MockLinkedIdentityRepository::new();

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

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

    #[tokio::test]
    async fn test_create_linked_identity_success() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_id = StringUuid::new_v4();

        linked_mock.expect_create().returning(|input| {
            Ok(LinkedIdentity {
                id: StringUuid::new_v4(),
                user_id: input.user_id,
                provider_type: input.provider_type.clone(),
                provider_alias: input.provider_alias.clone(),
                external_user_id: input.external_user_id.clone(),
                external_email: input.external_email.clone(),
                linked_at: chrono::Utc::now(),
            })
        });

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let input = CreateLinkedIdentityInput {
            user_id,
            provider_type: "google".to_string(),
            provider_alias: "google".to_string(),
            external_user_id: "google-ext-123".to_string(),
            external_email: Some("user@gmail.com".to_string()),
        };
        let result = service.create_linked_identity(&input).await;
        assert!(result.is_ok());
        let linked = result.unwrap();
        assert_eq!(linked.user_id, user_id);
        assert_eq!(linked.provider_type, "google");
        assert_eq!(linked.external_user_id, "google-ext-123");
    }

    #[tokio::test]
    async fn test_find_linked_identity_existing() {
        let mut linked_mock = MockLinkedIdentityRepository::new();
        let user_id = StringUuid::new_v4();

        linked_mock
            .expect_find_by_provider()
            .with(eq("github"), eq("gh-456"))
            .returning(move |alias, ext_id| {
                Ok(Some(LinkedIdentity {
                    user_id,
                    provider_alias: alias.to_string(),
                    external_user_id: ext_id.to_string(),
                    provider_type: "github".to_string(),
                    ..Default::default()
                }))
            });

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let result = service.find_linked_identity("github", "gh-456").await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().external_user_id, "gh-456");
    }

    #[tokio::test]
    async fn test_find_linked_identity_not_found() {
        let mut linked_mock = MockLinkedIdentityRepository::new();

        linked_mock
            .expect_find_by_provider()
            .with(eq("github"), eq("nonexistent"))
            .returning(|_, _| Ok(None));

        let keycloak = create_test_federation_broker();
        let service = IdentityProviderService::new(Arc::new(linked_mock), keycloak);

        let result = service.find_linked_identity("github", "nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    // Helper to create a test KeycloakClient
    fn create_test_keycloak_client() -> Arc<KeycloakClient> {
        use crate::config::KeycloakConfig;
        Arc::new(KeycloakClient::new(KeycloakConfig {
            url: "http://localhost:8081".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        }))
    }

    fn create_test_federation_broker() -> Arc<dyn FederationBroker> {
        Arc::new(KeycloakFederationBrokerAdapter::new(
            create_test_keycloak_client(),
        ))
    }
}
