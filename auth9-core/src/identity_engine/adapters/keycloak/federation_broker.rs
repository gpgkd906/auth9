use crate::error::Result;
use crate::identity_engine::{
    FederatedIdentityRepresentation, FederationBroker, IdentityProviderRepresentation,
};
use crate::keycloak::{KeycloakClient, KeycloakFederatedIdentity, KeycloakIdentityProvider};
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

impl From<KeycloakFederatedIdentity> for FederatedIdentityRepresentation {
    fn from(value: KeycloakFederatedIdentity) -> Self {
        Self {
            identity_provider: value.identity_provider,
            user_id: value.user_id,
            user_name: value.user_name,
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

    async fn get_user_federated_identities(
        &self,
        user_id: &str,
    ) -> Result<Vec<FederatedIdentityRepresentation>> {
        Ok(self
            .client
            .get_user_federated_identities(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn remove_user_federated_identity(
        &self,
        user_id: &str,
        provider_alias: &str,
    ) -> Result<()> {
        self.client
            .remove_user_federated_identity(user_id, provider_alias)
            .await
    }
}
