//! Keycloak Event Webhook HTTP handler tests
//!
//! Tests for the POST /api/v1/keycloak/events endpoint.

use super::TestAppState;
use auth9_core::api::keycloak_event;
use auth9_core::domain::LoginEventType;
use auth9_core::repository::LoginEventRepository;
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower::ServiceExt;

type HmacSha256 = Hmac<Sha256>;

/// Build a minimal router with only the keycloak events endpoint.
/// This is needed because the keycloak events route is only in build_full_router,
/// which requires HasDbPool (unavailable in test state).
fn build_keycloak_event_test_router(state: TestAppState) -> Router {
    Router::new()
        .route(
            "/api/v1/keycloak/events",
            post(keycloak_event::receive::<TestAppState>),
        )
        .with_state(state)
}

/// Helper: POST raw bytes to the keycloak events endpoint
async fn post_keycloak_event(app: &Router, body: &[u8], headers: Vec<(&str, &str)>) -> StatusCode {
    let mut req_builder = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/keycloak/events")
        .header("Content-Type", "application/json");

    for (key, value) in headers {
        req_builder = req_builder.header(key, value);
    }

    let request = req_builder.body(Body::from(body.to_vec())).unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    response.status()
}

/// Helper: Compute HMAC-SHA256 signature
fn compute_signature(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    let hex = hex::encode(mac.finalize().into_bytes());
    format!("sha256={}", hex)
}

// ============================================================================
// Successful Event Processing
// ============================================================================

#[tokio::test]
async fn test_receive_login_event_success() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "ipAddress": "192.168.1.100",
        "time": 1704067200000i64,
        "details": {
            "username": "testuser",
            "email": "testuser@example.com"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Verify event was recorded
    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, LoginEventType::Success);
}

#[tokio::test]
async fn test_receive_login_error_event() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN_ERROR",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "error": "invalid_user_credentials",
        "time": 1704067200000i64,
        "details": {
            "username": "baduser",
            "email": "baduser@example.com"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, LoginEventType::FailedPassword);
    assert!(events[0].failure_reason.is_some());
}

#[tokio::test]
async fn test_receive_mfa_failure_event() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN_ERROR",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "error": "invalid_totp",
        "time": 1704067200000i64,
        "details": {
            "email": "mfa-user@example.com"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, LoginEventType::FailedMfa);
}

#[tokio::test]
async fn test_receive_social_login_event() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "IDENTITY_PROVIDER_LOGIN",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {
            "identityProvider": "google",
            "email": "google-user@example.com"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, LoginEventType::Social);
}

#[tokio::test]
async fn test_receive_lockout_event() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "USER_DISABLED_BY_TEMPORARY_LOCKOUT",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {}
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, LoginEventType::Locked);
}

// ============================================================================
// Admin Events (should be skipped)
// ============================================================================

#[tokio::test]
async fn test_receive_admin_event_skipped() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "operationType": "CREATE",
        "resourceType": "USER",
        "realmId": "auth9",
        "time": 1704067200000i64
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // No events should be recorded for admin events
    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert!(events.is_empty());
}

// ============================================================================
// Non-Login Events (should be acknowledged but not recorded)
// ============================================================================

#[tokio::test]
async fn test_receive_non_login_event_skipped() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGOUT",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {}
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert!(events.is_empty());
}

// ============================================================================
// Error Cases
// ============================================================================

#[tokio::test]
async fn test_receive_malformed_json() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state);

    let body = b"not valid json at all{{{";

    let status = post_keycloak_event(&app, body, vec![]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// Webhook Signature Verification
// ============================================================================

#[tokio::test]
async fn test_receive_with_valid_signature() {
    let mut state = TestAppState::new("http://mock-keycloak");
    let secret = "test-webhook-secret";
    // Set webhook secret in config
    let mut config = (*state.config).clone();
    config.keycloak.webhook_secret = Some(secret.to_string());
    state.config = std::sync::Arc::new(config);

    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {"email": "sig-user@example.com"}
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();
    let signature = compute_signature(secret, &body_bytes);

    let status = post_keycloak_event(
        &app,
        &body_bytes,
        vec![("x-keycloak-signature", &signature)],
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_receive_with_invalid_signature() {
    let mut state = TestAppState::new("http://mock-keycloak");
    let mut config = (*state.config).clone();
    config.keycloak.webhook_secret = Some("real-secret".to_string());
    state.config = std::sync::Arc::new(config);

    let app = build_keycloak_event_test_router(state);

    let body = serde_json::json!({
        "type": "LOGIN",
        "time": 1704067200000i64
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(
        &app,
        &body_bytes,
        vec![(
            "x-keycloak-signature",
            "sha256=0000000000000000000000000000000000000000000000000000000000000000",
        )],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_receive_missing_signature_when_required() {
    let mut state = TestAppState::new("http://mock-keycloak");
    let mut config = (*state.config).clone();
    config.keycloak.webhook_secret = Some("secret".to_string());
    state.config = std::sync::Arc::new(config);

    let app = build_keycloak_event_test_router(state);

    let body = serde_json::json!({
        "type": "LOGIN",
        "time": 1704067200000i64
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    // No signature header
    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Email Fallback from Username
// ============================================================================

#[tokio::test]
async fn test_receive_email_from_username_fallback() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {
            "username": "fallback-username"
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(&app, &body_bytes, vec![]).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    // Should use username as fallback since email is not provided
    assert_eq!(events[0].email.as_deref(), Some("fallback-username"));
}

// ============================================================================
// User-Agent Header Extraction
// ============================================================================

#[tokio::test]
async fn test_receive_extracts_user_agent() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {"email": "ua-user@example.com"}
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(
        &app,
        &body_bytes,
        vec![("user-agent", "Mozilla/5.0 Test Browser")],
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].user_agent.as_deref(),
        Some("Mozilla/5.0 Test Browser")
    );
}

#[tokio::test]
async fn test_receive_prefers_forwarded_user_agent() {
    let state = TestAppState::new("http://mock-keycloak");
    let app = build_keycloak_event_test_router(state.clone());

    let body = serde_json::json!({
        "type": "LOGIN",
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "time": 1704067200000i64,
        "details": {"email": "fwd-ua@example.com"}
    });
    let body_bytes = serde_json::to_vec(&body).unwrap();

    let status = post_keycloak_event(
        &app,
        &body_bytes,
        vec![
            ("user-agent", "Server UA"),
            ("x-forwarded-user-agent", "Client UA"),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let events = state.login_event_repo.list(0, 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].user_agent.as_deref(), Some("Client UA"));
}
