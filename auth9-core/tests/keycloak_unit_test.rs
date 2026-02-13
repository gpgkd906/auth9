//! Keycloak Client Unit Tests (using WireMock)
//! These tests are fast and don't require a real Keycloak instance.

use auth9_core::config::KeycloakConfig;
use auth9_core::keycloak::{CreateKeycloakUserInput, KeycloakClient, KeycloakOidcClient};
use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn create_test_config(base_url: &str) -> KeycloakConfig {
    KeycloakConfig {
        url: base_url.to_string(),
        public_url: base_url.to_string(),
        realm: "test".to_string(),
        admin_client_id: "auth9-admin".to_string(),
        admin_client_secret: "test-secret".to_string(),
        ssl_required: "none".to_string(),
        core_public_url: None,
        portal_url: None,
        webhook_secret: None,
    }
}

fn create_test_client(base_url: &str) -> KeycloakClient {
    let config = create_test_config(base_url);
    KeycloakClient::new(config)
}

#[tokio::test]
async fn test_create_user_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint first
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock user creation endpoint
    let user_id = "user-uuid-12345";
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(201).append_header(
            "Location",
            format!("{}/admin/realms/test/users/{}", mock_server.uri(), user_id),
        ))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .create_user(&CreateKeycloakUserInput {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            enabled: true,
            email_verified: false,
            credentials: None,
        })
        .await;

    assert!(result.is_ok());
    let created_id = result.unwrap();
    assert_eq!(created_id, user_id);
}

#[tokio::test]
async fn test_create_user_conflict() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock conflict response (user already exists)
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/users"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "errorMessage": "User exists with same username"
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .create_user(&CreateKeycloakUserInput {
            username: "existinguser".to_string(),
            email: "existing@example.com".to_string(),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            credentials: None,
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_user_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock get user endpoint
    Mock::given(method("GET"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": user_id,
            "username": "testuser",
            "email": "test@example.com",
            "firstName": "Test",
            "lastName": "User",
            "enabled": true,
            "emailVerified": true
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let user = client.get_user(user_id).await.unwrap();
    assert_eq!(user.username, "testuser".to_string());
    assert_eq!(user.email, Some("test@example.com".to_string()));
}

#[tokio::test]
async fn test_get_user_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock 404 response
    Mock::given(method("GET"))
        .and(path_regex(r"/admin/realms/test/users/.*"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_user("nonexistent-user").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_user_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-to-delete";
    // Mock delete endpoint
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_user(user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_oidc_client_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "oidc-client-uuid-123";
    // Mock client creation endpoint
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/clients"))
        .respond_with(ResponseTemplate::new(201).append_header(
            "Location",
            format!(
                "{}/admin/realms/test/clients/{}",
                mock_server.uri(),
                client_uuid
            ),
        ))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let oidc_client = KeycloakOidcClient {
        id: None,
        client_id: "my-app".to_string(),
        name: Some("My Application".to_string()),
        enabled: true,
        public_client: false,
        redirect_uris: vec!["https://app.example.com/callback".to_string()],
        web_origins: vec!["https://app.example.com".to_string()],
        protocol: "openid-connect".to_string(),
        base_url: Some("https://app.example.com".to_string()),
        root_url: Some("https://app.example.com".to_string()),
        admin_url: None,
        attributes: None,
        secret: None,
    };

    let result = client.create_oidc_client(&oidc_client).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_client_secret_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "client-uuid-123";
    // Mock get secret endpoint
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/clients/{}/client-secret",
            client_uuid
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "secret",
            "value": "super-secret-value-abc123"
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let secret = client.get_client_secret(client_uuid).await.unwrap();
    assert_eq!(secret, "super-secret-value-abc123");
}

#[tokio::test]
async fn test_regenerate_client_secret_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "client-uuid-456";
    // Mock regenerate secret endpoint
    Mock::given(method("POST"))
        .and(path(format!(
            "/admin/realms/test/clients/{}/client-secret",
            client_uuid
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "secret",
            "value": "new-regenerated-secret-xyz789"
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let new_secret = client.regenerate_client_secret(client_uuid).await.unwrap();
    assert_eq!(new_secret, "new-regenerated-secret-xyz789");
}

#[tokio::test]
async fn test_search_users_by_email() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Note: We use path_regex because the actual path includes query params
    Mock::given(method("GET"))
        .and(path_regex(r"/admin/realms/test/users.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "user-1",
                "username": "founduser",
                "email": "found@example.com",
                "enabled": true,
                "emailVerified": true
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let users = client
        .search_users_by_email("found@example.com")
        .await
        .unwrap();
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, Some("found@example.com".to_string()));
}

#[tokio::test]
async fn test_delete_oidc_client_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "client-to-delete";
    // Mock delete client endpoint
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/clients/{}", client_uuid)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_oidc_client(client_uuid).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_user_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock update user endpoint
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let update = auth9_core::keycloak::KeycloakUserUpdate {
        username: None,
        first_name: Some("Updated".to_string()),
        last_name: Some("User".to_string()),
        email: None,
        enabled: None,
        email_verified: None,
        required_actions: None,
    };

    let result = client.update_user(user_id, &update).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_user_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "nonexistent-user";
    // Mock 404 response
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let update = auth9_core::keycloak::KeycloakUserUpdate {
        username: None,
        first_name: Some("Updated".to_string()),
        last_name: None,
        email: None,
        enabled: None,
        email_verified: None,
        required_actions: None,
    };

    let result = client.update_user(user_id, &update).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_oidc_client_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "client-uuid-123";
    // Mock update client endpoint
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/clients/{}", client_uuid)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let oidc_client = KeycloakOidcClient {
        id: Some(client_uuid.to_string()),
        client_id: "updated-app".to_string(),
        name: Some("Updated Application".to_string()),
        enabled: true,
        public_client: false,
        redirect_uris: vec!["https://updated.example.com/callback".to_string()],
        web_origins: vec!["https://updated.example.com".to_string()],
        protocol: "openid-connect".to_string(),
        base_url: None,
        root_url: None,
        admin_url: None,
        attributes: None,
        secret: None,
    };

    let result = client.update_oidc_client(client_uuid, &oidc_client).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_oidc_client_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "nonexistent-client";
    // Mock 404 response
    Mock::given(method("PUT"))
        .and(path(format!("/admin/realms/test/clients/{}", client_uuid)))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let oidc_client = KeycloakOidcClient {
        id: Some(client_uuid.to_string()),
        client_id: "test-app".to_string(),
        name: None,
        enabled: true,
        public_client: false,
        redirect_uris: vec![],
        web_origins: vec![],
        protocol: "openid-connect".to_string(),
        base_url: None,
        root_url: None,
        admin_url: None,
        attributes: None,
        secret: None,
    };

    let result = client.update_oidc_client(client_uuid, &oidc_client).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_client_uuid_by_client_id_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "found-client-uuid-123";
    // Mock query client endpoint
    Mock::given(method("GET"))
        .and(path_regex(r"/admin/realms/test/clients.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": client_uuid,
                "clientId": "my-app",
                "name": "My Application",
                "enabled": true,
                "protocol": "openid-connect",
                "publicClient": false,
                "redirectUris": [],
                "webOrigins": []
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_client_uuid_by_client_id("my-app").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), client_uuid);
}

#[tokio::test]
async fn test_get_client_uuid_by_client_id_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock empty response (client not found)
    Mock::given(method("GET"))
        .and(path_regex(r"/admin/realms/test/clients.*"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_client_uuid_by_client_id("nonexistent-app").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_oidc_client_conflict() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock conflict response (client already exists)
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/clients"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "errorMessage": "Client already exists"
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let oidc_client = KeycloakOidcClient {
        id: None,
        client_id: "existing-app".to_string(),
        name: Some("Existing Application".to_string()),
        enabled: true,
        public_client: false,
        redirect_uris: vec![],
        web_origins: vec![],
        protocol: "openid-connect".to_string(),
        base_url: None,
        root_url: None,
        admin_url: None,
        attributes: None,
        secret: None,
    };

    let result = client.create_oidc_client(&oidc_client).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_user_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "nonexistent-user";
    // Mock 404 response
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_user(user_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_oidc_client_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client_uuid = "nonexistent-client";
    // Mock 404 response
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/clients/{}", client_uuid)))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_oidc_client(client_uuid).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_user_credentials_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock list credentials endpoint
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "cred-1",
                "type": "password"
            },
            {
                "id": "cred-2",
                "type": "otp"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.list_user_credentials(user_id).await;
    assert!(result.is_ok());
    let credentials = result.unwrap();
    assert_eq!(credentials.len(), 2);
    assert_eq!(credentials[0].credential_type, "password");
}

#[tokio::test]
async fn test_list_user_credentials_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "nonexistent-user";
    // Mock 404 response
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials",
            user_id
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.list_user_credentials(user_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_user_credential_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    let credential_id = "cred-uuid-67890";
    // Mock delete credential endpoint
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials/{}",
            user_id, credential_id
        )))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_user_credential(user_id, credential_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_user_credential_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    let credential_id = "nonexistent-cred";
    // Mock 404 response
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials/{}",
            user_id, credential_id
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_user_credential(user_id, credential_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_remove_totp_credentials_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint - will be called multiple times
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";

    // Mock list credentials endpoint
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "password-cred",
                "type": "password"
            },
            {
                "id": "totp-cred",
                "type": "totp"
            },
            {
                "id": "otp-cred",
                "type": "otp"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Mock delete credential endpoint (matches any credential under this user)
    Mock::given(method("DELETE"))
        .and(path_regex(format!(
            r"/admin/realms/test/users/{}/credentials/.*",
            user_id
        )))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.remove_totp_credentials(user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_totp_credentials_empty() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";

    // Mock list credentials endpoint - no OTP credentials
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "password-cred",
                "type": "password"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.remove_totp_credentials(user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_confidential_client_with_secret() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint - should receive client_secret parameter
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .and(wiremock::matchers::body_string_contains(
            "client_secret=test-secret",
        ))
        .and(wiremock::matchers::body_string_contains(
            "grant_type=password",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "confidential-token",
            "expires_in": 300,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock create user endpoint
    Mock::given(method("POST"))
        .and(path_regex(r"^/admin/realms/test/users$"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            &format!("{}/admin/realms/test/users/user-123", mock_server.uri()),
        ))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    // Any operation that requires admin token should send client_secret
    let input = CreateKeycloakUserInput {
        username: "testuser".to_string(),
        email: "test@example.com".to_string(),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        enabled: true,
        email_verified: false,
        credentials: None,
    };

    let result = client.create_user(&input).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_backward_compatibility_without_client_secret() {
    let mock_server = MockServer::start().await;

    // Create config without client_secret (backward compatibility)
    let config = KeycloakConfig {
        url: mock_server.uri(),
        public_url: mock_server.uri(),
        realm: "test".to_string(),
        admin_client_id: "admin-cli".to_string(),
        admin_client_secret: String::new(), // Empty secret
        ssl_required: "none".to_string(),
        core_public_url: None,
        portal_url: None,
        webhook_secret: None,
    };

    // Mock token endpoint - should NOT receive client_secret parameter
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .and(wiremock::matchers::body_string_contains(
            "grant_type=password",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "public-token",
            "expires_in": 300,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Mock create user endpoint
    Mock::given(method("POST"))
        .and(path_regex(r"^/admin/realms/test/users$"))
        .respond_with(ResponseTemplate::new(201).insert_header(
            "Location",
            &format!("{}/admin/realms/test/users/user-456", mock_server.uri()),
        ))
        .mount(&mock_server)
        .await;

    let client = KeycloakClient::new(config);

    // Verify the client works without secret (backward compatibility)
    let input = CreateKeycloakUserInput {
        username: "testuser2".to_string(),
        email: "test2@example.com".to_string(),
        first_name: Some("Test".to_string()),
        last_name: Some("User".to_string()),
        enabled: true,
        email_verified: false,
        credentials: None,
    };

    let result = client.create_user(&input).await;
    assert!(result.is_ok());
}

// ============================================================================
// Master Realm Admin Client Tests
// ============================================================================

use auth9_core::keycloak::KeycloakSeeder;

fn create_seeder_config(base_url: &str) -> KeycloakConfig {
    KeycloakConfig {
        url: base_url.to_string(),
        public_url: base_url.to_string(),
        realm: "auth9".to_string(),
        admin_client_id: "auth9-admin".to_string(),
        admin_client_secret: "test-secret".to_string(),
        ssl_required: "none".to_string(),
        core_public_url: None,
        portal_url: None,
        webhook_secret: None,
    }
}

#[tokio::test]
async fn test_seed_master_admin_client_success() {
    let mock_server = MockServer::start().await;

    // Set environment variable for client secret
    std::env::set_var("KEYCLOAK_ADMIN_CLIENT_SECRET", "preset-secret-123");

    // Mock token endpoint (using admin-cli)
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .and(wiremock::matchers::body_string_contains(
            "client_id=admin-cli",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "admin-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock check if client exists (not found)
    Mock::given(method("GET"))
        .and(path("/admin/realms/master/clients"))
        .and(wiremock::matchers::query_param("clientId", "auth9-admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    // Mock client creation in master realm
    Mock::given(method("POST"))
        .and(path("/admin/realms/master/clients"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let config = create_seeder_config(&mock_server.uri());
    let seeder = KeycloakSeeder::new(&config);

    let result = seeder.seed_master_admin_client().await;
    assert!(result.is_ok());

    // Clean up
    std::env::remove_var("KEYCLOAK_ADMIN_CLIENT_SECRET");
}

#[tokio::test]
async fn test_seed_master_admin_client_already_exists() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "admin-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock check if client exists (found)
    Mock::given(method("GET"))
        .and(path("/admin/realms/master/clients"))
        .and(wiremock::matchers::query_param("clientId", "auth9-admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "existing-client-uuid",
                "clientId": "auth9-admin"
            }
        ])))
        .mount(&mock_server)
        .await;

    let config = create_seeder_config(&mock_server.uri());
    let seeder = KeycloakSeeder::new(&config);

    let result = seeder.seed_master_admin_client().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_seed_master_admin_client_conflict_handling() {
    let mock_server = MockServer::start().await;

    std::env::set_var("KEYCLOAK_ADMIN_CLIENT_SECRET", "preset-secret-123");

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "admin-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock check if client exists (not found initially)
    Mock::given(method("GET"))
        .and(path("/admin/realms/master/clients"))
        .and(wiremock::matchers::query_param("clientId", "auth9-admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    // Mock client creation returning conflict (race condition scenario)
    Mock::given(method("POST"))
        .and(path("/admin/realms/master/clients"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "errorMessage": "Client auth9-admin already exists"
        })))
        .mount(&mock_server)
        .await;

    let config = create_seeder_config(&mock_server.uri());
    let seeder = KeycloakSeeder::new(&config);

    let result = seeder.seed_master_admin_client().await;
    // Should succeed despite conflict (idempotency)
    assert!(result.is_ok());

    std::env::remove_var("KEYCLOAK_ADMIN_CLIENT_SECRET");
}

#[tokio::test]
async fn test_seed_master_admin_client_with_auto_generated_secret() {
    let mock_server = MockServer::start().await;

    // No preset secret (should retrieve auto-generated one)
    std::env::remove_var("KEYCLOAK_ADMIN_CLIENT_SECRET");

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "admin-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock check if client exists (not found)
    Mock::given(method("GET"))
        .and(path("/admin/realms/master/clients"))
        .and(wiremock::matchers::query_param("clientId", "auth9-admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    // Mock client creation
    Mock::given(method("POST"))
        .and(path("/admin/realms/master/clients"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    // Mock get client UUID by clientId
    Mock::given(method("GET"))
        .and(path("/admin/realms/master/clients"))
        .and(wiremock::matchers::query_param("clientId", "auth9-admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "new-client-uuid-123",
                "clientId": "auth9-admin"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Mock get client secret
    Mock::given(method("GET"))
        .and(path(
            "/admin/realms/master/clients/new-client-uuid-123/client-secret",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "secret",
            "value": "auto-generated-secret-xyz"
        })))
        .mount(&mock_server)
        .await;

    let config = create_seeder_config(&mock_server.uri());
    let seeder = KeycloakSeeder::new(&config);

    let result = seeder.seed_master_admin_client().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_seed_admin_client_in_configured_realm() {
    let mock_server = MockServer::start().await;

    std::env::set_var("KEYCLOAK_ADMIN_CLIENT_SECRET", "preset-secret-123");

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "admin-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock check if client exists in auth9 realm (not found)
    Mock::given(method("GET"))
        .and(path("/admin/realms/auth9/clients"))
        .and(wiremock::matchers::query_param("clientId", "auth9-admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    // Mock client creation in auth9 realm
    Mock::given(method("POST"))
        .and(path("/admin/realms/auth9/clients"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let config = create_seeder_config(&mock_server.uri());
    let seeder = KeycloakSeeder::new(&config);

    let result = seeder.seed_admin_client().await;
    assert!(result.is_ok());

    std::env::remove_var("KEYCLOAK_ADMIN_CLIENT_SECRET");
}

// ============================================================================
// Password Management Tests
// ============================================================================

#[tokio::test]
async fn test_reset_user_password_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock reset password endpoint
    Mock::given(method("PUT"))
        .and(path(format!(
            "/admin/realms/test/users/{}/reset-password",
            user_id
        )))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .reset_user_password(user_id, "NewPassword123!", false)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reset_user_password_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "nonexistent-user";
    // Mock 404 response
    Mock::given(method("PUT"))
        .and(path(format!(
            "/admin/realms/test/users/{}/reset-password",
            user_id
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .reset_user_password(user_id, "NewPassword123!", false)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_user_password_valid() {
    let mock_server = MockServer::start().await;

    // Mock admin token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock get user endpoint
    Mock::given(method("GET"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": user_id,
            "username": "testuser",
            "email": "test@example.com",
            "enabled": true
        })))
        .mount(&mock_server)
        .await;

    // Mock user authentication token endpoint (successful)
    Mock::given(method("POST"))
        .and(path("/realms/test/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "user-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .validate_user_password(user_id, "CorrectPassword123!")
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap()); // Password is valid
}

#[tokio::test]
async fn test_validate_user_password_invalid() {
    let mock_server = MockServer::start().await;

    // Mock admin token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock get user endpoint
    Mock::given(method("GET"))
        .and(path(format!("/admin/realms/test/users/{}", user_id)))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": user_id,
            "username": "testuser",
            "email": "test@example.com",
            "enabled": true
        })))
        .mount(&mock_server)
        .await;

    // Mock user authentication token endpoint (failed - wrong password)
    Mock::given(method("POST"))
        .and(path("/realms/test/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "Invalid user credentials"
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .validate_user_password(user_id, "WrongPassword!")
        .await;
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Password is invalid
}

// ============================================================================
// Session Management Tests
// ============================================================================

#[tokio::test]
async fn test_get_user_sessions_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock get user sessions endpoint
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/sessions",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "session-1",
                "username": "testuser",
                "userId": user_id,
                "ipAddress": "192.168.1.1",
                "start": 1704067200000_i64,
                "lastAccess": 1704070800000_i64,
                "clients": {
                    "client-uuid": "my-app"
                }
            },
            {
                "id": "session-2",
                "username": "testuser",
                "userId": user_id,
                "ipAddress": "10.0.0.1",
                "start": 1704067200000_i64,
                "lastAccess": 1704070800000_i64,
                "clients": {}
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_user_sessions(user_id).await;
    assert!(result.is_ok());
    let sessions = result.unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[0].ip_address, Some("192.168.1.1".to_string()));
}

#[tokio::test]
async fn test_get_user_sessions_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "nonexistent-user";
    // Mock 404 response
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/sessions",
            user_id
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_user_sessions(user_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_user_session_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let session_id = "session-uuid-12345";
    // Mock delete session endpoint
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/sessions/{}", session_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_user_session(session_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_user_session_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let session_id = "nonexistent-session";
    // Mock 404 response
    Mock::given(method("DELETE"))
        .and(path(format!("/admin/realms/test/sessions/{}", session_id)))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_user_session(session_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_logout_user_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock logout endpoint
    Mock::given(method("POST"))
        .and(path(format!("/admin/realms/test/users/{}/logout", user_id)))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.logout_user(user_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_logout_user_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "nonexistent-user";
    // Mock 404 response
    Mock::given(method("POST"))
        .and(path(format!("/admin/realms/test/users/{}/logout", user_id)))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.logout_user(user_id).await;
    assert!(result.is_err());
}

// ============================================================================
// Identity Provider Tests
// ============================================================================

#[tokio::test]
async fn test_list_identity_providers_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock list identity providers endpoint
    Mock::given(method("GET"))
        .and(path("/admin/realms/test/identity-provider/instances"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "alias": "google",
                "displayName": "Google",
                "providerId": "google",
                "enabled": true,
                "config": {
                    "clientId": "google-client-id",
                    "clientSecret": "google-secret"
                }
            },
            {
                "alias": "github",
                "displayName": "GitHub",
                "providerId": "github",
                "enabled": true,
                "config": {
                    "clientId": "github-client-id",
                    "clientSecret": "github-secret"
                }
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.list_identity_providers().await;
    assert!(result.is_ok());
    let providers = result.unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0].alias, "google");
    assert_eq!(providers[1].alias, "github");
}

#[tokio::test]
async fn test_list_identity_providers_empty() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock empty list
    Mock::given(method("GET"))
        .and(path("/admin/realms/test/identity-provider/instances"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.list_identity_providers().await;
    assert!(result.is_ok());
    let providers = result.unwrap();
    assert_eq!(providers.len(), 0);
}

#[tokio::test]
async fn test_create_identity_provider_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock create identity provider endpoint
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/identity-provider/instances"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    use auth9_core::keycloak::KeycloakIdentityProvider;
    let provider = KeycloakIdentityProvider {
        alias: "google".to_string(),
        display_name: Some("Google".to_string()),
        provider_id: "google".to_string(),
        enabled: true,
        trust_email: false,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: std::collections::HashMap::new(),
    };

    let result = client.create_identity_provider(&provider).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_identity_provider_conflict() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock conflict response
    Mock::given(method("POST"))
        .and(path("/admin/realms/test/identity-provider/instances"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "errorMessage": "Identity provider already exists"
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    use auth9_core::keycloak::KeycloakIdentityProvider;
    let provider = KeycloakIdentityProvider {
        alias: "google".to_string(),
        display_name: Some("Google".to_string()),
        provider_id: "google".to_string(),
        enabled: true,
        trust_email: false,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: std::collections::HashMap::new(),
    };

    let result = client.create_identity_provider(&provider).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_identity_provider_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock delete identity provider endpoint
    Mock::given(method("DELETE"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/google",
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_identity_provider("google").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_identity_provider_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock 404 response
    Mock::given(method("DELETE"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/nonexistent",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.delete_identity_provider("nonexistent").await;
    assert!(result.is_err());
}

// ============================================================================
// WebAuthn Credentials Tests
// ============================================================================

#[tokio::test]
async fn test_list_webauthn_credentials_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock list credentials endpoint - includes both webauthn and other credentials
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "cred-1",
                "type": "password"
            },
            {
                "id": "cred-2",
                "type": "webauthn"
            },
            {
                "id": "cred-3",
                "type": "webauthn-passwordless"
            },
            {
                "id": "cred-4",
                "type": "totp"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.list_webauthn_credentials(user_id).await;
    assert!(result.is_ok());
    let creds = result.unwrap();
    // Should only include webauthn credentials (cred-2 and cred-3)
    assert_eq!(creds.len(), 2);
    assert!(creds.iter().all(|c| c.credential_type.contains("webauthn")));
}

#[tokio::test]
async fn test_list_webauthn_credentials_empty() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock list credentials endpoint - no webauthn credentials
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/credentials",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "cred-1",
                "type": "password"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.list_webauthn_credentials(user_id).await;
    assert!(result.is_ok());
    let creds = result.unwrap();
    assert_eq!(creds.len(), 0);
}

// ============================================================================
// Realm Management Tests
// ============================================================================

#[tokio::test]
async fn test_get_realm_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock get realm endpoint
    Mock::given(method("GET"))
        .and(path("/admin/realms/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "test-realm-id",
            "realm": "test",
            "displayName": "Test Realm",
            "enabled": true,
            "registrationAllowed": false,
            "resetPasswordAllowed": true,
            "loginWithEmailAllowed": true
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_realm().await;
    assert!(result.is_ok());
    let realm = result.unwrap();
    assert_eq!(realm.realm, "test");
}

#[tokio::test]
async fn test_get_realm_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock 404 response
    Mock::given(method("GET"))
        .and(path("/admin/realms/test"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_realm().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_realm_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock update realm endpoint
    Mock::given(method("PUT"))
        .and(path("/admin/realms/test"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    use auth9_core::keycloak::RealmUpdate;
    let update = RealmUpdate {
        registration_allowed: Some(true),
        reset_password_allowed: Some(true),
        ssl_required: Some("none".to_string()),
        ..Default::default()
    };

    let result = client.update_realm(&update).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_realm_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock 404 response
    Mock::given(method("PUT"))
        .and(path("/admin/realms/test"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    use auth9_core::keycloak::RealmUpdate;
    let update = RealmUpdate {
        registration_allowed: Some(true),
        ..Default::default()
    };

    let result = client.update_realm(&update).await;
    assert!(result.is_err());
}

// ============================================================================
// Federated Identity Tests
// ============================================================================

#[tokio::test]
async fn test_get_user_federated_identities_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock get federated identities endpoint
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/federated-identity",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "identityProvider": "google",
                "userId": "google-user-id-123",
                "userName": "test@gmail.com"
            },
            {
                "identityProvider": "github",
                "userId": "github-user-id-456",
                "userName": "testuser"
            }
        ])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_user_federated_identities(user_id).await;
    assert!(result.is_ok());
    let identities = result.unwrap();
    assert_eq!(identities.len(), 2);
    assert_eq!(identities[0].identity_provider, "google");
    assert_eq!(identities[1].identity_provider, "github");
}

#[tokio::test]
async fn test_get_user_federated_identities_empty() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock empty response
    Mock::given(method("GET"))
        .and(path(format!(
            "/admin/realms/test/users/{}/federated-identity",
            user_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_user_federated_identities(user_id).await;
    assert!(result.is_ok());
    let identities = result.unwrap();
    assert_eq!(identities.len(), 0);
}

#[tokio::test]
async fn test_remove_user_federated_identity_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock remove federated identity endpoint
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/admin/realms/test/users/{}/federated-identity/google",
            user_id
        )))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .remove_user_federated_identity(user_id, "google")
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_remove_user_federated_identity_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    let user_id = "user-uuid-12345";
    // Mock 404 response
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/admin/realms/test/users/{}/federated-identity/nonexistent",
            user_id
        )))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client
        .remove_user_federated_identity(user_id, "nonexistent")
        .await;
    assert!(result.is_err());
}

// ============================================================================
// Get Identity Provider Tests
// ============================================================================

#[tokio::test]
async fn test_get_identity_provider_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock get identity provider endpoint
    Mock::given(method("GET"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/google",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "alias": "google",
            "displayName": "Google",
            "providerId": "google",
            "enabled": true,
            "config": {
                "clientId": "google-client-id",
                "clientSecret": "google-secret"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_identity_provider("google").await;
    assert!(result.is_ok());
    let provider = result.unwrap();
    assert_eq!(provider.alias, "google");
    assert_eq!(provider.provider_id, "google");
}

#[tokio::test]
async fn test_get_identity_provider_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock 404 response
    Mock::given(method("GET"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/nonexistent",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    let result = client.get_identity_provider("nonexistent").await;
    assert!(result.is_err());
}

// ============================================================================
// Update Identity Provider Tests
// ============================================================================

#[tokio::test]
async fn test_update_identity_provider_success() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock update identity provider endpoint
    Mock::given(method("PUT"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/google",
        ))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    use auth9_core::keycloak::KeycloakIdentityProvider;
    let provider = KeycloakIdentityProvider {
        alias: "google".to_string(),
        display_name: Some("Updated Google".to_string()),
        provider_id: "google".to_string(),
        enabled: false,
        trust_email: false,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: std::collections::HashMap::new(),
    };

    let result = client.update_identity_provider("google", &provider).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_identity_provider_not_found() {
    let mock_server = MockServer::start().await;

    // Mock token endpoint
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "mock-token",
            "expires_in": 300
        })))
        .mount(&mock_server)
        .await;

    // Mock 404 response
    Mock::given(method("PUT"))
        .and(path(
            "/admin/realms/test/identity-provider/instances/nonexistent",
        ))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = create_test_client(&mock_server.uri());

    use auth9_core::keycloak::KeycloakIdentityProvider;
    let provider = KeycloakIdentityProvider {
        alias: "nonexistent".to_string(),
        display_name: None,
        provider_id: "oidc".to_string(),
        enabled: true,
        trust_email: false,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: std::collections::HashMap::new(),
    };

    let result = client
        .update_identity_provider("nonexistent", &provider)
        .await;
    assert!(result.is_err());
}
