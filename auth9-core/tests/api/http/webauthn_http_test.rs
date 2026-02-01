//! WebAuthn/Passkey HTTP API handler tests
//!
//! Tests for WebAuthn credential management endpoints.

use super::{get_json, MockKeycloakServer, TestAppState};
use auth9_core::api::SuccessResponse;
use axum::http::StatusCode;
use serde::Deserialize;

// Response type for testing
#[derive(Debug, Deserialize)]
struct RegisterUrlTestResponse {
    url: String,
}

// ============================================================================
// Get Register URL Tests
// ============================================================================

#[tokio::test]
async fn test_get_register_url_default_redirect() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_webauthn_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<RegisterUrlTestResponse>>) =
        get_json(&app, "/api/v1/webauthn/register-url").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert!(response.url.contains("WEBAUTHN_REGISTER"));
    // Should contain default redirect
    assert!(response.url.contains("passkeys"));
}

#[tokio::test]
async fn test_get_register_url_custom_redirect() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_webauthn_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<RegisterUrlTestResponse>>) = get_json(
        &app,
        "/api/v1/webauthn/register-url?redirect_uri=/custom/path",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert!(response.url.contains("WEBAUTHN_REGISTER"));
    // Should contain custom redirect (URL encoded)
    assert!(response.url.contains("custom"));
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_webauthn_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::webauthn;
    use axum::routing::get;

    axum::Router::new()
        .route(
            "/api/v1/webauthn/register-url",
            get(webauthn::get_register_url::<TestAppState>),
        )
        .with_state(state)
}
