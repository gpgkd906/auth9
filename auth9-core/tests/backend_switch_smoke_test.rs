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

fn dummy_pool() -> sqlx::MySqlPool {
    sqlx::MySqlPool::connect_lazy("mysql://localhost/dummy").unwrap()
}

#[test]
fn backend_switch_defaults_to_keycloak() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();

    let mut config = common::test_config();
    config.identity_backend = IdentityBackend::Keycloak;

    let (_, federation_broker, _) =
        select_identity_backend(&config, build_keycloak_client(&config), dummy_pool());

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
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.enter();

    let mut config = common::test_config();
    config.identity_backend = IdentityBackend::Auth9Oidc;

    let (session_store, _federation_broker, identity_engine) =
        select_identity_backend(&config, build_keycloak_client(&config), dummy_pool());

    runtime.block_on(async {
        // Session store (no DB) operations should succeed
        session_store.delete_user_session("session-1").await?;
        session_store.logout_user("user-1").await?;

        // Realm update is a no-op for Auth9Oidc
        identity_engine.update_realm(&Default::default()).await?;

        // User store delete_user cleans up oidc tables (succeeds even with dummy pool
        // since the tables may not exist — the important thing is it doesn't panic).
        // With a dummy pool it will fail at DB level, which is expected.
        let _ = identity_engine.user_store().delete_user("user-1").await;

        // Client store get_client_secret now generates a random secret
        let secret = identity_engine
            .client_store()
            .get_client_secret("client-1")
            .await;
        assert!(secret.is_ok());
        assert!(!secret.unwrap().is_empty());

        // Note: federation_broker and credential_store operations now require a
        // real DB connection (backed by SocialProviderRepository and credentials
        // table respectively), so they are not tested with dummy_pool().

        Ok::<(), AppError>(())
    })?;

    Ok(())
}
