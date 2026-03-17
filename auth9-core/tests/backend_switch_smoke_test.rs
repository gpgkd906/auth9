mod common;

use auth9_core::config::{Config, IdentityBackend, KeycloakConfig};
use auth9_core::keycloak::KeycloakClient;
use auth9_core::server::select_identity_backend;
use auth9_core::{AppError, Result};
use std::sync::Arc;

fn build_keycloak_client(config: &Config) -> Arc<KeycloakClient> {
    Arc::new(KeycloakClient::new(KeycloakConfig {
        url: config.keycloak.url.clone(),
        public_url: config.keycloak.public_url.clone(),
        realm: config.keycloak.realm.clone(),
        admin_client_id: config.keycloak.admin_client_id.clone(),
        admin_client_secret: config.keycloak.admin_client_secret.clone(),
        ssl_required: config.keycloak.ssl_required.clone(),
        core_public_url: config.keycloak.core_public_url.clone(),
        portal_url: config.keycloak.portal_url.clone(),
        webhook_secret: config.keycloak.webhook_secret.clone(),
    }))
}

#[test]
fn backend_switch_defaults_to_keycloak() {
    let mut config = common::test_config();
    config.identity_backend = IdentityBackend::Keycloak;

    let (_, federation_broker, _) =
        select_identity_backend(&config, build_keycloak_client(&config));

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let providers = runtime
        .block_on(federation_broker.list_identity_providers())
        .unwrap_err();

    match providers {
        AppError::Keycloak(_) => {}
        other => panic!("expected keycloak error, got {other:?}"),
    }
}

#[test]
fn backend_switch_can_use_auth9_oidc_stub() -> Result<()> {
    let mut config = common::test_config();
    config.identity_backend = IdentityBackend::Auth9Oidc;

    let (session_store, federation_broker, identity_engine) =
        select_identity_backend(&config, build_keycloak_client(&config));

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        session_store.delete_user_session("session-1").await?;
        session_store.logout_user("user-1").await?;
        let providers = federation_broker.list_identity_providers().await?;
        let linked = federation_broker
            .get_user_federated_identities("user-1")
            .await?;
        identity_engine.update_realm(&Default::default()).await?;
        assert!(providers.is_empty());
        assert!(linked.is_empty());
        Ok::<(), AppError>(())
    })?;

    Ok(())
}
