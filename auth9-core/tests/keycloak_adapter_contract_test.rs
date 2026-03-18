use auth9_core::config::KeycloakConfig;
use auth9_core::identity_engine::adapters::keycloak::{
    KeycloakFederationBrokerAdapter, KeycloakIdentityEngineAdapter, KeycloakSessionStoreAdapter,
};
use auth9_core::identity_engine::{
    FederationBroker, IdentityCredentialInput, IdentityEngine, IdentityProviderRepresentation,
    IdentitySessionStore, IdentityUserCreateInput, IdentityUserUpdateInput,
};
use auth9_core::keycloak::{KeycloakClient, KeycloakOidcClient};
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
        .and(path(
            "/admin/realms/test/identity-provider/instances/corp-oidc",
        ))
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
        .and(path(
            "/admin/realms/test/identity-provider/instances/corp-oidc",
        ))
        .and(body_string_contains(
            "\"displayName\":\"Corp OIDC Updated\"",
        ))
        .and(body_string_contains("\"internalId\":\"kc-internal-1\""))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/corp-oidc",
        ))
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
    assert_eq!(
        fetched.config.get("clientSecret"),
        Some(&"corp-secret".to_string())
    );

    let create_input = IdentityProviderRepresentation {
        alias: "corp-oidc".to_string(),
        display_name: Some("Corp OIDC".to_string()),
        provider_id: "oidc".to_string(),
        enabled: true,
        trust_email: true,
        store_token: false,
        link_only: false,
        first_login_policy: "auto_merge".to_string(),
        first_broker_login_flow_alias: None,
        config: HashMap::from([("clientId".to_string(), "corp-client".to_string())]),
        extra: HashMap::new(),
    };
    adapter
        .create_identity_provider(&create_input)
        .await
        .unwrap();

    let mut update_input = fetched.clone();
    update_input.display_name = Some("Corp OIDC Updated".to_string());
    adapter
        .update_identity_provider("corp-oidc", &update_input)
        .await
        .unwrap();

    adapter.delete_identity_provider("corp-oidc").await.unwrap();
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

#[tokio::test]
async fn keycloak_identity_engine_adapter_supports_user_client_and_credential_stores() {
    let server = MockServer::start().await;
    mock_admin_token(&server, 1).await;

    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .and(body_string_contains("\"username\":\"alice@example.com\""))
        .respond_with(ResponseTemplate::new(201).insert_header("location", "/users/user-123"))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/admin/realms/test/users/user-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-123",
            "username": "alice@example.com",
            "email": "alice@example.com",
            "enabled": true,
            "emailVerified": false,
            "attributes": {}
        })))
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/admin/realms/test/users/user-123"))
        .and(body_string_contains("\"firstName\":\"Alice Updated\""))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/admin/realms/test/users/user-123"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/realms/test/protocol/openid-connect/token"))
        .and(body_string_contains("grant_type=password"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "user-token"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/admin/realms/test/clients"))
        .and(body_string_contains("\"clientId\":\"svc-123\""))
        .respond_with(
            ResponseTemplate::new(201).insert_header("location", "/clients/client-uuid-1"),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path(
            "/admin/realms/test/clients/client-uuid-1/client-secret",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "value": "secret-1" })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/admin/realms/test/clients/client-uuid-1/client-secret",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "value": "secret-2" })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/admin/realms/test/clients"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "client-uuid-1",
                "clientId": "svc-123",
                "enabled": true,
                "protocol": "openid-connect",
                "redirectUris": [],
                "webOrigins": [],
                "publicClient": false
            }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/admin/realms/test/clients/client-uuid-1"))
        .and(body_string_contains("\"name\":\"Service 123 Updated\""))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/admin/realms/test/clients/client-uuid-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/admin/realms/test/users/user-123/credentials"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "id": "cred-1", "type": "password" },
            { "id": "cred-2", "type": "otp" }
        ])))
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/admin/realms/test/users/user-123/credentials/cred-2"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let adapter = KeycloakIdentityEngineAdapter::new(create_test_client(&server.uri()));

    let user_id = adapter
        .user_store()
        .create_user(&IdentityUserCreateInput {
            username: "alice@example.com".to_string(),
            email: "alice@example.com".to_string(),
            first_name: Some("Alice".to_string()),
            last_name: None,
            enabled: true,
            email_verified: false,
            credentials: Some(vec![IdentityCredentialInput {
                credential_type: "password".to_string(),
                value: "Password123!".to_string(),
                temporary: false,
            }]),
        })
        .await
        .unwrap();
    assert_eq!(user_id, "user-123");

    let user = adapter.user_store().get_user(&user_id).await.unwrap();
    assert_eq!(user.email.as_deref(), Some("alice@example.com"));
    assert!(adapter
        .user_store()
        .validate_user_password(&user_id, "Password123!")
        .await
        .unwrap());
    adapter
        .user_store()
        .update_user(
            &user_id,
            &IdentityUserUpdateInput {
                username: None,
                email: None,
                first_name: Some("Alice Updated".to_string()),
                last_name: None,
                enabled: None,
                email_verified: None,
                required_actions: None,
            },
        )
        .await
        .unwrap();

    let client_uuid = adapter
        .client_store()
        .create_oidc_client(&KeycloakOidcClient {
            id: None,
            client_id: "svc-123".to_string(),
            name: Some("Service 123".to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: None,
            root_url: None,
            admin_url: None,
            redirect_uris: Vec::new(),
            web_origins: Vec::new(),
            attributes: None,
            public_client: false,
            secret: None,
        })
        .await
        .unwrap();
    assert_eq!(client_uuid, "client-uuid-1");
    assert_eq!(
        adapter
            .client_store()
            .get_client_secret(&client_uuid)
            .await
            .unwrap(),
        "secret-1"
    );
    assert_eq!(
        adapter
            .client_store()
            .get_client_uuid_by_client_id("svc-123")
            .await
            .unwrap(),
        "client-uuid-1"
    );
    assert_eq!(
        adapter
            .client_store()
            .get_client_by_client_id("svc-123")
            .await
            .unwrap()
            .client_id,
        "svc-123"
    );
    assert_eq!(
        adapter
            .client_store()
            .regenerate_client_secret(&client_uuid)
            .await
            .unwrap(),
        "secret-2"
    );
    adapter
        .client_store()
        .update_oidc_client(
            &client_uuid,
            &KeycloakOidcClient {
                id: Some(client_uuid.clone()),
                client_id: "svc-123".to_string(),
                name: Some("Service 123 Updated".to_string()),
                enabled: true,
                protocol: "openid-connect".to_string(),
                base_url: None,
                root_url: None,
                admin_url: None,
                redirect_uris: Vec::new(),
                web_origins: Vec::new(),
                attributes: None,
                public_client: false,
                secret: None,
            },
        )
        .await
        .unwrap();

    let credentials = adapter
        .credential_store()
        .list_user_credentials(&user_id)
        .await
        .unwrap();
    assert_eq!(credentials.len(), 2);
    adapter
        .credential_store()
        .remove_totp_credentials(&user_id)
        .await
        .unwrap();

    adapter.user_store().delete_user(&user_id).await.unwrap();
    adapter
        .client_store()
        .delete_oidc_client(&client_uuid)
        .await
        .unwrap();
}
