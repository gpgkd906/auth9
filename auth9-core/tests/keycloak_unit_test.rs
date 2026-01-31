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
        .and(wiremock::matchers::body_string_contains("client_secret=test-secret"))
        .and(wiremock::matchers::body_string_contains("grant_type=password"))
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
        admin_client_secret: String::new(),  // Empty secret
        ssl_required: "none".to_string(),
    };

    // Mock token endpoint - should NOT receive client_secret parameter
    Mock::given(method("POST"))
        .and(path("/realms/master/protocol/openid-connect/token"))
        .and(wiremock::matchers::body_string_contains("grant_type=password"))
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
