use crate::error::{AppError, Result};
use anyhow::anyhow;
use crate::identity_engine::{
    FederatedIdentityRepresentation, FederationBroker, IdentityProviderRepresentation,
};
use async_trait::async_trait;

#[derive(Default)]
pub struct Auth9OidcFederationBrokerAdapter;

impl Auth9OidcFederationBrokerAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FederationBroker for Auth9OidcFederationBrokerAdapter {
    async fn list_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>> {
        Ok(vec![])
    }

    async fn get_identity_provider(&self, alias: &str) -> Result<IdentityProviderRepresentation> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc identity provider '{}' not implemented",
            alias
        )))
    }

    async fn create_identity_provider(
        &self,
        _provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc identity provider create not implemented"
        )))
    }

    async fn update_identity_provider(
        &self,
        alias: &str,
        _provider: &IdentityProviderRepresentation,
    ) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc identity provider '{}' update not implemented",
            alias
        )))
    }

    async fn delete_identity_provider(&self, alias: &str) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc identity provider '{}' delete not implemented",
            alias
        )))
    }

    async fn get_user_federated_identities(
        &self,
        _user_id: &str,
    ) -> Result<Vec<FederatedIdentityRepresentation>> {
        Ok(vec![])
    }

    async fn remove_user_federated_identity(
        &self,
        _user_id: &str,
        provider_alias: &str,
    ) -> Result<()> {
        Err(AppError::Internal(anyhow!(
            "auth9_oidc federated identity '{}' remove not implemented",
            provider_alias
        )))
    }
}
