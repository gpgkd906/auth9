//! Webhook HTTP API handler tests
//!
//! Tests for webhook management endpoints.

use super::{
    delete_json_with_auth, get_json_with_auth, post_json_with_auth, put_json_with_auth,
    MockKeycloakServer, TestAppState,
};
use crate::api::{create_test_identity_token, create_test_tenant};
use auth9_core::api::{MessageResponse, SuccessResponse};
use auth9_core::domain::{StringUuid, Webhook};
use auth9_core::repository::WebhookRepository;
use auth9_core::service::WebhookTestResult;
use axum::http::StatusCode;
use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};

async fn get_json<T>(app: &axum::Router, path: &str) -> (StatusCode, Option<T>)
where
    T: DeserializeOwned,
{
    let token = create_test_identity_token();
    get_json_with_auth(app, path, &token).await
}

async fn post_json<B, T>(app: &axum::Router, path: &str, body: &B) -> (StatusCode, Option<T>)
where
    B: Serialize,
    T: DeserializeOwned,
{
    let token = create_test_identity_token();
    post_json_with_auth(app, path, body, &token).await
}

async fn put_json<B, T>(app: &axum::Router, path: &str, body: &B) -> (StatusCode, Option<T>)
where
    B: Serialize,
    T: DeserializeOwned,
{
    let token = create_test_identity_token();
    put_json_with_auth(app, path, body, &token).await
}

async fn delete_json<T>(app: &axum::Router, path: &str) -> (StatusCode, Option<T>)
where
    T: DeserializeOwned,
{
    let token = create_test_identity_token();
    delete_json_with_auth(app, path, &token).await
}

// ============================================================================
// List Webhooks Tests
// ============================================================================

#[tokio::test]
async fn test_list_webhooks_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Webhook>>>) =
        get_json(&app, &format!("/api/v1/tenants/{}/webhooks", tenant_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhooks = body.unwrap().data;
    assert!(webhooks.is_empty());
}

#[tokio::test]
async fn test_list_webhooks_with_data() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Add some webhooks
    for i in 0..3 {
        let webhook = Webhook {
            id: StringUuid::new_v4(),
            tenant_id,
            name: format!("Webhook {}", i),
            url: format!("https://example.com/hook/{}", i),
            secret: Some("secret".to_string()),
            events: vec!["login.success".to_string()],
            enabled: true,
            last_triggered_at: None,
            failure_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        state.webhook_repo.add_webhook(webhook).await;
    }

    let app = build_webhook_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<Webhook>>>) =
        get_json(&app, &format!("/api/v1/tenants/{}/webhooks", tenant_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhooks = body.unwrap().data;
    assert_eq!(webhooks.len(), 3);
}

// ============================================================================
// Get Webhook Tests
// ============================================================================

#[tokio::test]
async fn test_get_webhook_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id,
        name: "Test Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: Some("secret123".to_string()),
        events: vec!["login.success".to_string(), "login.failed".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Webhook>>) = get_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant_id, webhook_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhook = body.unwrap().data;
    assert_eq!(webhook.name, "Test Webhook");
    assert_eq!(webhook.events.len(), 2);
}

#[tokio::test]
async fn test_get_webhook_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = get_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant_id, nonexistent_id),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_webhook_wrong_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant1 = create_test_tenant(None);
    let tenant1_id = tenant1.id;
    state.tenant_repo.add_tenant(tenant1).await;

    let tenant2 = create_test_tenant(None);
    let tenant2_id = tenant2.id;
    state.tenant_repo.add_tenant(tenant2).await;

    // Webhook belongs to tenant1
    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id: tenant1_id,
        name: "Test Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: None,
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    // Try to access from tenant2
    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = get_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant2_id, webhook_id),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create Webhook Tests
// ============================================================================

#[tokio::test]
async fn test_create_webhook_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let input = serde_json::json!({
        "name": "New Webhook",
        "url": "https://example.com/new-hook",
        "secret": "my-secret",
        "events": ["user.created", "user.deleted"],
        "enabled": true
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks", tenant_id),
        &input,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhook = body.unwrap().data;
    assert_eq!(webhook.name, "New Webhook");
    assert_eq!(webhook.url, "https://example.com/new-hook");
    assert_eq!(webhook.events.len(), 2);
    assert!(webhook.enabled);
}

#[tokio::test]
async fn test_create_webhook_validation_error_empty_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let input = serde_json::json!({
        "name": "",
        "url": "https://example.com/hook",
        "events": ["login.success"],
        "enabled": true
    });

    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks", tenant_id),
        &input,
    )
    .await;

    // 422 UNPROCESSABLE_ENTITY for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_webhook_validation_error_invalid_url() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let input = serde_json::json!({
        "name": "Test",
        "url": "not-a-valid-url",
        "events": ["login.success"],
        "enabled": true
    });

    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks", tenant_id),
        &input,
    )
    .await;

    // 422 UNPROCESSABLE_ENTITY for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Update Webhook Tests
// ============================================================================

#[tokio::test]
async fn test_update_webhook_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id,
        name: "Original Name".to_string(),
        url: "https://example.com/original".to_string(),
        secret: None,
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    let input = serde_json::json!({
        "name": "Updated Name",
        "events": ["user.created"],
        "enabled": false
    });

    let (status, body): (StatusCode, Option<SuccessResponse<Webhook>>) = put_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant_id, webhook_id),
        &input,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhook = body.unwrap().data;
    assert_eq!(webhook.name, "Updated Name");
    assert!(!webhook.enabled);
    assert_eq!(webhook.events, vec!["user.created".to_string()]);
}

// ============================================================================
// Delete Webhook Tests
// ============================================================================

#[tokio::test]
async fn test_delete_webhook_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id,
        name: "To Delete".to_string(),
        url: "https://example.com/delete".to_string(),
        secret: None,
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state.clone());

    let (status, body): (StatusCode, Option<MessageResponse>) = delete_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant_id, webhook_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("deleted"));

    // Verify webhook is gone
    let webhooks = state.webhook_repo.list_by_tenant(tenant_id).await.unwrap();
    assert!(webhooks.is_empty());
}

#[tokio::test]
async fn test_delete_webhook_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<MessageResponse>) = delete_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant_id, nonexistent_id),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Update Webhook - Not Found / Wrong Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_update_webhook_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let input = serde_json::json!({
        "name": "Updated Name",
        "events": ["user.created"],
        "enabled": false
    });

    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = put_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant_id, nonexistent_id),
        &input,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_webhook_wrong_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant1 = create_test_tenant(None);
    let tenant1_id = tenant1.id;
    state.tenant_repo.add_tenant(tenant1).await;

    let tenant2 = create_test_tenant(None);
    let tenant2_id = tenant2.id;
    state.tenant_repo.add_tenant(tenant2).await;

    // Webhook belongs to tenant1
    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id: tenant1_id,
        name: "Tenant1 Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: Some("secret123".to_string()),
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    let input = serde_json::json!({
        "name": "Hacked Name",
        "enabled": false
    });

    // Try to update from tenant2
    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = put_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}", tenant2_id, webhook_id),
        &input,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create Webhook - Tenant Not Found Test
// ============================================================================

#[tokio::test]
async fn test_create_webhook_tenant_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Don't create a tenant - use a random ID
    let nonexistent_tenant_id = StringUuid::new_v4();

    let app = build_webhook_test_router(state);

    let input = serde_json::json!({
        "name": "Orphan Webhook",
        "url": "https://example.com/orphan",
        "events": ["user.created"],
        "enabled": true
    });

    // The create_webhook handler does not validate tenant existence,
    // so the webhook is created successfully with the given tenant_id.
    let (status, body): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks", nonexistent_tenant_id),
        &input,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhook = body.unwrap().data;
    assert_eq!(webhook.name, "Orphan Webhook");
    assert_eq!(webhook.tenant_id, nonexistent_tenant_id);
}

// ============================================================================
// Regenerate Webhook Secret Tests
// ============================================================================

#[tokio::test]
async fn test_regenerate_webhook_secret_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let original_secret = "original-secret-value".to_string();
    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id,
        name: "Regenerate Test".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: Some(original_secret.clone()),
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!(
            "/api/v1/tenants/{}/webhooks/{}/regenerate-secret",
            tenant_id, webhook_id
        ),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let webhook = body.unwrap().data;
    // Secret should have changed
    assert!(webhook.secret.is_some());
    let new_secret = webhook.secret.unwrap();
    assert_ne!(new_secret, original_secret);
    // New secret should follow the whsec_ format
    assert!(new_secret.starts_with("whsec_"));
}

#[tokio::test]
async fn test_regenerate_webhook_secret_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!(
            "/api/v1/tenants/{}/webhooks/{}/regenerate-secret",
            tenant_id, nonexistent_id
        ),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_regenerate_webhook_secret_wrong_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant1 = create_test_tenant(None);
    let tenant1_id = tenant1.id;
    state.tenant_repo.add_tenant(tenant1).await;

    let tenant2 = create_test_tenant(None);
    let tenant2_id = tenant2.id;
    state.tenant_repo.add_tenant(tenant2).await;

    // Webhook belongs to tenant1
    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id: tenant1_id,
        name: "Tenant1 Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: Some("secret123".to_string()),
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    // Try to regenerate from tenant2
    let (status, _): (StatusCode, Option<SuccessResponse<Webhook>>) = post_json(
        &app,
        &format!(
            "/api/v1/tenants/{}/webhooks/{}/regenerate-secret",
            tenant2_id, webhook_id
        ),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Test Webhook Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_webhook_test_endpoint() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id,
        name: "Test Endpoint Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: Some("secret123".to_string()),
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<WebhookTestResult>>) = post_json(
        &app,
        &format!("/api/v1/tenants/{}/webhooks/{}/test", tenant_id, webhook_id),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let result = body.unwrap().data;
    // The test event will fail because the URL is not reachable,
    // but the handler should still return 200 with a WebhookTestResult.
    assert!(!result.success);
    assert!(result.error.is_some());
    assert!(result.response_time_ms.is_some());
}

#[tokio::test]
async fn test_webhook_test_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_webhook_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<SuccessResponse<WebhookTestResult>>) = post_json(
        &app,
        &format!(
            "/api/v1/tenants/{}/webhooks/{}/test",
            tenant_id, nonexistent_id
        ),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_webhook_test_wrong_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant1 = create_test_tenant(None);
    let tenant1_id = tenant1.id;
    state.tenant_repo.add_tenant(tenant1).await;

    let tenant2 = create_test_tenant(None);
    let tenant2_id = tenant2.id;
    state.tenant_repo.add_tenant(tenant2).await;

    // Webhook belongs to tenant1
    let webhook = Webhook {
        id: StringUuid::new_v4(),
        tenant_id: tenant1_id,
        name: "Tenant1 Webhook".to_string(),
        url: "https://example.com/webhook".to_string(),
        secret: Some("secret123".to_string()),
        events: vec!["login.success".to_string()],
        enabled: true,
        last_triggered_at: None,
        failure_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let webhook_id = webhook.id;
    state.webhook_repo.add_webhook(webhook).await;

    let app = build_webhook_test_router(state);

    // Try to test from tenant2
    let (status, _): (StatusCode, Option<SuccessResponse<WebhookTestResult>>) = post_json(
        &app,
        &format!(
            "/api/v1/tenants/{}/webhooks/{}/test",
            tenant2_id, webhook_id
        ),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_webhook_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::webhook;
    use axum::routing::{get, post};

    axum::Router::new()
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks",
            get(webhook::list_webhooks::<TestAppState>)
                .post(webhook::create_webhook::<TestAppState>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}",
            get(webhook::get_webhook::<TestAppState>)
                .put(webhook::update_webhook::<TestAppState>)
                .delete(webhook::delete_webhook::<TestAppState>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}/test",
            post(webhook::test_webhook::<TestAppState>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{webhook_id}/regenerate-secret",
            post(webhook::regenerate_webhook_secret::<TestAppState>),
        )
        .with_state(state)
}
