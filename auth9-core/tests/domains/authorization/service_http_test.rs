//! Service/Client API HTTP Handler Tests
//!
//! Tests for the service and client HTTP endpoints using mock repositories.

use crate::support::create_test_service;
use crate::support::create_test_tenant_access_token;
use crate::support::http::{
    build_test_router, delete_json_with_auth, get_json_with_auth, post_json_with_auth,
    put_json_with_auth, TestAppState,
};
use crate::support::mock_keycloak::MockKeycloakServer;
use auth9_core::api::{MessageResponse, PaginatedResponse, SuccessResponse};
use auth9_core::domain::{Client, Service, ServiceStatus};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Local struct for deserializing ServiceWithClient in tests
/// The response flattens the service and has a nested client object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestServiceWithClient {
    // Service fields (flattened)
    pub id: auth9_core::domain::StringUuid,
    pub tenant_id: Option<auth9_core::domain::StringUuid>,
    pub name: String,
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub logout_uris: Vec<String>,
    pub status: ServiceStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // Nested client with secret
    pub client: TestClientWithSecret,
}

/// Local struct for deserializing ClientWithSecret in tests
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestClientWithSecret {
    // Client fields (flattened)
    pub id: auth9_core::domain::StringUuid,
    pub service_id: auth9_core::domain::StringUuid,
    pub client_id: String,
    pub name: Option<String>,
    #[serde(skip_serializing)]
    pub client_secret_hash: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    // Additional field
    pub client_secret: String,
}

// ============================================================================
// List Services Tests
// ============================================================================

#[tokio::test]
async fn test_list_services() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();

    // Add some services
    let svc1 = create_test_service(None, Some(tenant_id));
    let mut svc2 = create_test_service(None, Some(tenant_id));
    svc2.name = "Service 2".to_string();

    state.service_repo.add_service(svc1).await;
    state.service_repo.add_service(svc2).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<Service>>) =
        get_json_with_auth(&app, "/api/v1/services", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 2);
    assert_eq!(response.pagination.total, 2);
}

#[tokio::test]
async fn test_list_services_with_tenant_filter() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Add services for different tenants
    let svc1 = create_test_service(None, Some(tenant1));
    let svc2 = create_test_service(None, Some(tenant1));
    let svc3 = create_test_service(None, Some(tenant2));

    state.service_repo.add_service(svc1).await;
    state.service_repo.add_service(svc2).await;
    state.service_repo.add_service(svc3).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<Service>>) = get_json_with_auth(
        &app,
        &format!("/api/v1/services?tenant_id={}", tenant1),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 2);
}

#[tokio::test]
async fn test_list_services_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<Service>>) =
        get_json_with_auth(&app, "/api/v1/services", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.is_empty());
}

#[tokio::test]
async fn test_list_services_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add 25 services
    for i in 1..=25 {
        let mut svc = create_test_service(None, None);
        svc.name = format!("Service {}", i);
        state.service_repo.add_service(svc).await;
    }

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<Service>>) =
        get_json_with_auth(&app, "/api/v1/services?page=2&per_page=10", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.page, 2);
    assert_eq!(response.pagination.total, 25);
}

// ============================================================================
// Get Service Tests
// ============================================================================

#[tokio::test]
async fn test_get_service() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.name = "My Service".to_string();
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<SuccessResponse<Service>>) =
        get_json_with_auth(&app, &format!("/api/v1/services/{}", service_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "My Service");
}

#[tokio::test]
async fn test_get_service_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/services/{}", nonexistent_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create Service Tests
// ============================================================================

#[tokio::test]
async fn test_create_service() {
    let mock_kc = MockKeycloakServer::new().await;
    let client_uuid = "kc-client-12345";
    let client_secret = "super-secret-value";
    mock_kc
        .setup_for_service_creation(client_uuid, client_secret)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let tenant_id = Uuid::new_v4();
    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "name": "New Service",
        "client_id": "new-service-client",
        "base_url": "https://newservice.example.com",
        "redirect_uris": ["https://newservice.example.com/callback"]
    });

    let (status, body): (StatusCode, Option<SuccessResponse<TestServiceWithClient>>) =
        post_json_with_auth(&app, "/api/v1/services", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "New Service");
    // The client_secret should be returned in the response
    assert_eq!(response.data.client.client_secret, client_secret);
}

#[tokio::test]
async fn test_create_service_with_logout_uris() {
    let mock_kc = MockKeycloakServer::new().await;
    let client_uuid = "kc-client-logout";
    let client_secret = "secret-with-logout";
    mock_kc
        .setup_for_service_creation(client_uuid, client_secret)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let tenant_id = Uuid::new_v4();
    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "name": "Service with Logout",
        "client_id": "logout-service",
        "base_url": "https://app.example.com",
        "redirect_uris": ["https://app.example.com/cb"],
        "logout_uris": [
            "https://app.example.com/logout",
            "https://app.example.com/signout"
        ]
    });

    let (status, body): (StatusCode, Option<SuccessResponse<TestServiceWithClient>>) =
        post_json_with_auth(&app, "/api/v1/services", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.logout_uris.len(), 2);
}

#[tokio::test]
async fn test_create_service_minimal() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc
        .setup_for_service_creation("kc-minimal", "minimal-secret")
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let tenant_id = Uuid::new_v4();
    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "name": "Minimal Service",
        "client_id": "minimal",
        "redirect_uris": []
    });

    let (status, body): (StatusCode, Option<SuccessResponse<TestServiceWithClient>>) =
        post_json_with_auth(&app, "/api/v1/services", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "Minimal Service");
}

#[tokio::test]
async fn test_create_service_keycloak_conflict() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_oidc_client_conflict().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let tenant_id = Uuid::new_v4();
    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "name": "Existing Service",
        "client_id": "existing-client",
        "redirect_uris": []
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/services", &input, &token).await;

    // Keycloak 409 is translated to Conflict
    assert_eq!(status, StatusCode::CONFLICT);
}

// ============================================================================
// Update Service Tests
// ============================================================================

#[tokio::test]
async fn test_update_service() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let input = json!({
        "name": "Updated Service Name"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Service>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/services/{}", service_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.name, "Updated Service Name");
}

#[tokio::test]
async fn test_update_service_status() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let input = json!({
        "status": "inactive"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Service>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/services/{}", service_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.status, ServiceStatus::Inactive);
}

#[tokio::test]
async fn test_update_service_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let nonexistent_id = Uuid::new_v4();
    let input = json!({
        "name": "Updated"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        &format!("/api/v1/services/{}", nonexistent_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Delete Service Tests
// ============================================================================

#[tokio::test]
async fn test_delete_service() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.setup_for_service_deletion().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, &format!("/api/v1/services/{}", service_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_service_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) = delete_json_with_auth(
        &app,
        &format!("/api/v1/services/{}", nonexistent_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Client Tests
// ============================================================================

#[tokio::test]
async fn test_list_clients() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    // Add clients
    let client1 = Client {
        id: auth9_core::domain::StringUuid::new_v4(),
        service_id: auth9_core::domain::StringUuid::from(service_id),
        client_id: "client-1".to_string(),
        name: Some("Client 1".to_string()),
        client_secret_hash: "hash1".to_string(),
        created_at: chrono::Utc::now(),
    };
    let client2 = Client {
        id: auth9_core::domain::StringUuid::new_v4(),
        service_id: auth9_core::domain::StringUuid::from(service_id),
        client_id: "client-2".to_string(),
        name: Some("Client 2".to_string()),
        client_secret_hash: "hash2".to_string(),
        created_at: chrono::Utc::now(),
    };

    state.service_repo.add_client(client1).await;
    state.service_repo.add_client(client2).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    // Get raw response to debug
    let (status, raw_body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/services/{}/clients", service_id),
        &token,
    )
    .await;

    println!("Status: {:?}", status);
    println!("Body: {:?}", raw_body);

    assert_eq!(status, StatusCode::OK);
    assert!(raw_body.is_some());
    let response = raw_body.unwrap();
    let data = response.get("data").expect("Should have data field");
    assert!(data.is_array());
    assert_eq!(data.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_create_client() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc
        .mock_create_oidc_client_success("kc-new-client")
        .await;
    mock_kc
        .mock_get_client_secret_any("new-client-secret")
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let input = json!({
        "name": "New Client"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<TestClientWithSecret>>) =
        post_json_with_auth(
            &app,
            &format!("/api/v1/services/{}/clients", service_id),
            &input,
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.client_secret, "new-client-secret");
}

#[tokio::test]
async fn test_create_client_without_name() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc
        .mock_create_oidc_client_success("kc-nameless-client")
        .await;
    mock_kc.mock_get_client_secret_any("nameless-secret").await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let input = json!({});

    let (status, body): (StatusCode, Option<SuccessResponse<TestClientWithSecret>>) =
        post_json_with_auth(
            &app,
            &format!("/api/v1/services/{}/clients", service_id),
            &input,
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_delete_client() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_get_client_uuid_not_found().await;
    mock_kc.mock_delete_oidc_client_success().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let client = Client {
        id: auth9_core::domain::StringUuid::new_v4(),
        service_id: auth9_core::domain::StringUuid::from(service_id),
        client_id: "client-to-delete".to_string(),
        name: Some("Client to Delete".to_string()),
        client_secret_hash: "hash".to_string(),
        created_at: chrono::Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<MessageResponse>) = delete_json_with_auth(
        &app,
        &format!("/api/v1/services/{}/clients/client-to-delete", service_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));
}

#[tokio::test]
async fn test_regenerate_client_secret() {
    let mock_kc = MockKeycloakServer::new().await;
    let kc_uuid = "kc-client-uuid";
    mock_kc
        .mock_get_client_uuid_by_client_id("existing-client", kc_uuid)
        .await;
    mock_kc
        .mock_regenerate_client_secret(kc_uuid, "brand-new-secret")
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let client = Client {
        id: auth9_core::domain::StringUuid::new_v4(),
        service_id: auth9_core::domain::StringUuid::from(service_id),
        client_id: "existing-client".to_string(),
        name: Some("Existing Client".to_string()),
        client_secret_hash: "old-hash".to_string(),
        created_at: chrono::Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<SuccessResponse<serde_json::Value>>) =
        post_json_with_auth(
            &app,
            &format!(
                "/api/v1/services/{}/clients/existing-client/regenerate-secret",
                service_id
            ),
            &json!({}),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data["client_secret"], "brand-new-secret");
    assert_eq!(response.data["client_id"], "existing-client");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_service_with_multiple_redirect_uris() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc
        .setup_for_service_creation("kc-multi-uri", "multi-uri-secret")
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let tenant_id = Uuid::new_v4();
    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "name": "Multi URI Service",
        "client_id": "multi-uri",
        "base_url": "https://app.example.com",
        "redirect_uris": [
            "https://app.example.com/callback",
            "https://app.example.com/oauth/callback",
            "https://staging.example.com/callback"
        ]
    });

    let (status, body): (StatusCode, Option<SuccessResponse<TestServiceWithClient>>) =
        post_json_with_auth(&app, "/api/v1/services", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.redirect_uris.len(), 3);
}

// ============================================================================
// Integration Info Tests
// ============================================================================

#[tokio::test]
async fn test_integration_info_success() {
    let mock_kc = MockKeycloakServer::new().await;
    // Register specific mock first (wiremock matches FIFO)
    mock_kc
        .mock_get_client_secret("kc-uuid-1", "the-secret-value")
        .await;
    // Then register broader client lookup mock
    mock_kc
        .mock_get_client_uuid_by_client_id("test-client", "kc-uuid-1")
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    // Add a client
    let client = Client {
        id: auth9_core::domain::StringUuid::new_v4(),
        service_id: auth9_core::domain::StringUuid::from(service_id),
        client_id: "test-client".to_string(),
        name: Some("Main Client".to_string()),
        client_secret_hash: "hash".to_string(),
        created_at: chrono::Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/services/{}/integration", service_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let data = body.unwrap()["data"].clone();

    // Verify service basic info
    assert!(data["service"]["name"].is_string());

    // Verify clients
    let clients = data["clients"].as_array().unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0]["client_id"], "test-client");
    assert_eq!(clients[0]["client_secret"], "the-secret-value");
    assert_eq!(clients[0]["public_client"], false);

    // Verify endpoints
    assert!(data["endpoints"]["authorize"]
        .as_str()
        .unwrap()
        .contains("/api/v1/auth/authorize"));
    assert!(data["endpoints"]["token"]
        .as_str()
        .unwrap()
        .contains("/api/v1/auth/token"));
    assert!(data["endpoints"]["jwks"]
        .as_str()
        .unwrap()
        .contains("/.well-known/jwks.json"));

    // Verify grpc
    assert!(data["grpc"]["address"].is_string());
    assert!(data["grpc"]["auth_mode"].is_string());

    // Verify environment_variables
    let env_vars = data["environment_variables"].as_array().unwrap();
    assert!(env_vars.iter().any(|v| v["key"] == "AUTH9_DOMAIN"));
    assert!(env_vars
        .iter()
        .any(|v| v["key"] == "AUTH9_CLIENT_ID" && v["value"] == "test-client"));
    assert!(env_vars
        .iter()
        .any(|v| v["key"] == "AUTH9_CLIENT_SECRET" && v["value"] == "the-secret-value"));
}

#[tokio::test]
async fn test_integration_info_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/services/{}/integration", nonexistent_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_integration_info_no_clients() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/services/{}/integration", service_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let data = body.unwrap()["data"].clone();
    let clients = data["clients"].as_array().unwrap();
    assert!(clients.is_empty());

    // env_vars should NOT have AUTH9_CLIENT_ID when no clients
    let env_vars = data["environment_variables"].as_array().unwrap();
    assert!(!env_vars.iter().any(|v| v["key"] == "AUTH9_CLIENT_ID"));
}

#[tokio::test]
async fn test_update_service_redirect_uris() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service).await;

    let app = build_test_router(state);
    let token = create_test_tenant_access_token();

    let input = json!({
        "redirect_uris": [
            "https://new.example.com/callback",
            "https://new.example.com/oauth"
        ]
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Service>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/services/{}", service_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.redirect_uris.len(), 2);
}
