use crate::error::{AppError, Result};
use crate::identity_engine::{FederationBroker, IdentityProviderRepresentation};
use crate::models::social_provider::CreateSocialProviderInput;
use crate::models::social_provider::UpdateSocialProviderInput;
use crate::repository::social_provider::SocialProviderRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct Auth9OidcFederationBrokerAdapter {
    social_provider_repo: Arc<dyn SocialProviderRepository>,
}

impl Auth9OidcFederationBrokerAdapter {
    pub fn new(social_provider_repo: Arc<dyn SocialProviderRepository>) -> Self {
        Self {
            social_provider_repo,
        }
    }
}

#[async_trait]
impl FederationBroker for Auth9OidcFederationBrokerAdapter {
    async fn list_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>> {
        let providers = self.social_provider_repo.list_all().await?;
        Ok(providers.into_iter().map(Into::into).collect())
    }

    async fn get_identity_provider(&self, alias: &str) -> Result<IdentityProviderRepresentation> {
        let provider = self
            .social_provider_repo
            .find_by_alias(alias)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Identity provider '{}' not found", alias))
            })?;
        Ok(provider.into())
    }

    async fn create_identity_provider(
        &self,
        provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        let input = CreateSocialProviderInput::from(provider);
        self.social_provider_repo.create(&input).await?;
        Ok(())
    }

    async fn update_identity_provider(
        &self,
        alias: &str,
        provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        let input = UpdateSocialProviderInput {
            display_name: provider.display_name.clone(),
            enabled: Some(provider.enabled),
            trust_email: Some(provider.trust_email),
            store_token: Some(provider.store_token),
            link_only: Some(provider.link_only),
            first_login_policy: Some(provider.first_login_policy.clone()),
            config: Some(provider.config.clone()),
        };
        self.social_provider_repo.update(alias, &input).await?;
        Ok(())
    }

    async fn delete_identity_provider(&self, alias: &str) -> Result<()> {
        self.social_provider_repo.delete_by_alias(alias).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::social_provider::SocialProvider;
    use crate::repository::social_provider::MockSocialProviderRepository;
    use mockall::predicate::*;
    use std::collections::HashMap;

    fn create_test_adapter(
        social_mock: MockSocialProviderRepository,
    ) -> Auth9OidcFederationBrokerAdapter {
        Auth9OidcFederationBrokerAdapter::new(Arc::new(social_mock))
    }

    #[tokio::test]
    async fn test_list_identity_providers() {
        let mut social_mock = MockSocialProviderRepository::new();

        social_mock.expect_list_all().returning(|| {
            Ok(vec![SocialProvider {
                alias: "google".to_string(),
                provider_type: "google".to_string(),
                ..Default::default()
            }])
        });

        let adapter = create_test_adapter(social_mock);
        let providers = adapter.list_identity_providers().await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].alias, "google");
        assert_eq!(providers[0].provider_id, "google");
    }

    #[tokio::test]
    async fn test_get_identity_provider() {
        let mut social_mock = MockSocialProviderRepository::new();

        social_mock
            .expect_find_by_alias()
            .with(eq("github"))
            .returning(|alias| {
                Ok(Some(SocialProvider {
                    alias: alias.to_string(),
                    provider_type: "github".to_string(),
                    ..Default::default()
                }))
            });

        let adapter = create_test_adapter(social_mock);
        let provider = adapter.get_identity_provider("github").await.unwrap();
        assert_eq!(provider.alias, "github");
    }

    #[tokio::test]
    async fn test_get_identity_provider_not_found() {
        let mut social_mock = MockSocialProviderRepository::new();

        social_mock
            .expect_find_by_alias()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let adapter = create_test_adapter(social_mock);
        let result = adapter.get_identity_provider("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_identity_provider() {
        let mut social_mock = MockSocialProviderRepository::new();

        social_mock.expect_create().returning(|input| {
            Ok(SocialProvider {
                alias: input.alias.clone(),
                provider_type: input.provider_type.clone(),
                ..Default::default()
            })
        });

        let adapter = create_test_adapter(social_mock);
        let repr = IdentityProviderRepresentation {
            alias: "microsoft".to_string(),
            display_name: Some("Microsoft".to_string()),
            provider_id: "microsoft".to_string(),
            enabled: true,
            trust_email: false,
            store_token: false,
            link_only: false,
            first_login_policy: "auto_merge".to_string(),
            first_broker_login_flow_alias: None,
            config: HashMap::new(),
            extra: HashMap::new(),
        };

        let result = adapter.create_identity_provider(&repr).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_identity_provider() {
        let mut social_mock = MockSocialProviderRepository::new();

        social_mock
            .expect_delete_by_alias()
            .with(eq("google"))
            .returning(|_| Ok(()));

        let adapter = create_test_adapter(social_mock);
        let result = adapter.delete_identity_provider("google").await;
        assert!(result.is_ok());
    }
}
