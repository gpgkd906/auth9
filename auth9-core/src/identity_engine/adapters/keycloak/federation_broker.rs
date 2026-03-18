use crate::error::Result;
use crate::identity_engine::{FederationBroker, IdentityProviderRepresentation};
use crate::keycloak::{KeycloakClient, KeycloakIdentityProvider};
use async_trait::async_trait;
use std::sync::Arc;

pub struct KeycloakFederationBrokerAdapter {
    client: Arc<KeycloakClient>,
}

impl KeycloakFederationBrokerAdapter {
    pub fn new(client: Arc<KeycloakClient>) -> Self {
        Self { client }
    }
}

impl From<KeycloakIdentityProvider> for IdentityProviderRepresentation {
    fn from(value: KeycloakIdentityProvider) -> Self {
        Self {
            alias: value.alias,
            display_name: value.display_name,
            provider_id: value.provider_id,
            enabled: value.enabled,
            trust_email: value.trust_email,
            store_token: value.store_token,
            link_only: value.link_only,
            first_login_policy: "auto_merge".to_string(),
            first_broker_login_flow_alias: value.first_broker_login_flow_alias,
            config: value.config,
            extra: value.extra,
        }
    }
}

impl From<IdentityProviderRepresentation> for KeycloakIdentityProvider {
    fn from(value: IdentityProviderRepresentation) -> Self {
        Self {
            alias: value.alias,
            display_name: value.display_name,
            provider_id: value.provider_id,
            enabled: value.enabled,
            trust_email: value.trust_email,
            store_token: value.store_token,
            link_only: value.link_only,
            first_broker_login_flow_alias: value.first_broker_login_flow_alias,
            config: value.config,
            extra: value.extra,
        }
    }
}

#[async_trait]
impl FederationBroker for KeycloakFederationBrokerAdapter {
    async fn list_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>> {
        Ok(self
            .client
            .list_identity_providers()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn get_identity_provider(&self, alias: &str) -> Result<IdentityProviderRepresentation> {
        Ok(self.client.get_identity_provider(alias).await?.into())
    }

    async fn create_identity_provider(
        &self,
        provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        self.client
            .create_identity_provider(&provider.clone().into())
            .await
    }

    async fn update_identity_provider(
        &self,
        alias: &str,
        provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        self.client
            .update_identity_provider(alias, &provider.clone().into())
            .await
    }

    async fn delete_identity_provider(&self, alias: &str) -> Result<()> {
        self.client.delete_identity_provider(alias).await
    }
}
