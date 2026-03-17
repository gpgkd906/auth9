use auth9_core::config::KeycloakConfig;
use auth9_core::identity_engine::adapters::keycloak::{
    KeycloakFederationBrokerAdapter, KeycloakIdentityEngineAdapter, KeycloakSessionStoreAdapter,
};
use auth9_core::identity_engine::{
    FederationBroker, IdentityEngine, IdentityProviderRepresentation, IdentitySessionStore,
};
use auth9_core::keycloak::KeycloakClient;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn create_test_client(base_url: &str) -> Arc<KeycloakClient> {
    Arc::new(KeycloakClient::new(KeycloakConfig {
        url: base_url.to_string(),
        public_url: base_url.to_string(),
        realm: "test".to_string(),
        admin_client_id: "admin-cli".to_string(),
        admin_client_secret: "test-secret".to_string(), // pragma: allowlist secret
        ssl_required: "none".to_string(),
        core_public_url: None,
        portal_url: None,
        webhook_secret: None,
    }))
}

async fn mock_admin_token(server: &MockServer, expected_calls: u64) {
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300,
            "token_type": "Bearer"
        })))
        .expect(expected_calls)
        .mount(server)
        .await;
}

#[tokio::test]
async fn keycloak_session_store_adapter_revokes_and_logs_out_sessions() {
    let server = MockServer::start().await;
    mock_admin_token(&server, 1).await;

    Mock::given(method("DELETE"))
        .and(path("/admin/realms/test/sessions/session-123"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users/user-123/logout"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = KeycloakSessionStoreAdapter::new(create_test_client(&server.uri()));
    adapter.delete_user_session("session-123").await.unwrap();
    adapter.logout_user("user-123").await.unwrap();
}

#[tokio::test]
async fn keycloak_federation_broker_adapter_supports_identity_provider_crud() {
    let server = MockServer::start().await;
    mock_admin_token(&server, 1).await;

    Mock::given(method("GET"))
        .and(path("/admin/realms/test/identity-provider/instances"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "alias": "corp-oidc",
                "displayName": "Corp OIDC",
                "providerId": "oidc",
                "enabled": true,
                "trustEmail": true,
                "storeToken": false,
                "linkOnly": false,
                "config": {
                    "clientId": "corp-client"
                },
                "internalId": "kc-internal-1"
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/admin/realms/test/identity-provider/instances/corp-oidc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "alias": "corp-oidc",
            "displayName": "Corp OIDC",
            "providerId": "oidc",
            "enabled": true,
            "trustEmail": true,
            "storeToken": false,
            "linkOnly": false,
            "config": {
                "clientId": "corp-client",
                "clientSecret": "corp-secret" // pragma: allowlist secret
            },
            "internalId": "kc-internal-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/admin/realms/test/identity-provider/instances"))
        .and(body_string_contains("\"alias\":\"corp-oidc\""))
        .and(body_string_contains("\"providerId\":\"oidc\""))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/admin/realms/test/identity-provider/instances/corp-oidc"))
        .and(body_string_contains("\"displayName\":\"Corp OIDC Updated\""))
        .and(body_string_contains("\"internalId\":\"kc-internal-1\""))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/admin/realms/test/identity-provider/instances/corp-oidc"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = KeycloakFederationBrokerAdapter::new(create_test_client(&server.uri()));

    let listed = adapter.list_identity_providers().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].alias, "corp-oidc");
    assert_eq!(
        listed[0].extra.get("internalId"),
        Some(&json!("kc-internal-1"))
    );

    let fetched = adapter.get_identity_provider("corp-oidc").await.unwrap();
    assert_eq!(fetched.config.get("clientSecret"), Some(&"corp-secret".to_string()));

    let create_input = IdentityProviderRepresentation {
        alias: "corp-oidc".to_string(),
        display_name: Some("Corp OIDC".to_string()),
        provider_id: "oidc".to_string(),
        enabled: true,
        trust_email: true,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: HashMap::from([("clientId".to_string(), "corp-client".to_string())]),
        extra: HashMap::new(),
    };
    adapter.create_identity_provider(&create_input).await.unwrap();

    let mut update_input = fetched.clone();
    update_input.display_name = Some("Corp OIDC Updated".to_string());
    adapter
        .update_identity_provider("corp-oidc", &update_input)
        .await
        .unwrap();

    adapter.delete_identity_provider("corp-oidc").await.unwrap();
}

#[tokio::test]
async fn keycloak_federation_broker_adapter_reads_and_removes_federated_identities() {
    let server = MockServer::start().await;
    mock_admin_token(&server, 1).await;

    Mock::given(method("GET"))
        .and(path("/admin/realms/test/users/user-123/federated-identity"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "identityProvider": "google",
                "userId": "google-123",
                "userName": "user@gmail.com"
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/admin/realms/test/users/user-123/federated-identity/google"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = KeycloakFederationBrokerAdapter::new(create_test_client(&server.uri()));

    let identities = adapter.get_user_federated_identities("user-123").await.unwrap();
    assert_eq!(identities.len(), 1);
    assert_eq!(identities[0].identity_provider, "google");
    assert_eq!(identities[0].user_id, "google-123");

    adapter
        .remove_user_federated_identity("user-123", "google")
        .await
        .unwrap();
}

#[tokio::test]
async fn keycloak_identity_engine_adapter_updates_realm_through_wrapped_client() {
    let server = MockServer::start().await;
    mock_admin_token(&server, 1).await;

    Mock::given(method("PUT"))
        .and(path("/admin/realms/test"))
        .and(body_string_contains("\"registrationAllowed\":true"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = KeycloakIdentityEngineAdapter::new(create_test_client(&server.uri()));
    adapter
        .update_realm(&auth9_core::keycloak::RealmUpdate {
            registration_allowed: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
}
