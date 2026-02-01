//! WebAuthn/Passkey HTTP API handler tests
//!
//! Tests for WebAuthn credential management endpoints.

use super::{
    delete_json, delete_json_with_auth, get_json, get_json_with_auth, MockKeycloakServer,
    TestAppState,
};
use crate::api::create_test_user;
use auth9_core::api::{MessageResponse, SuccessResponse};
use auth9_core::domain::WebAuthnCredential;
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
// List Passkeys Tests
// ============================================================================

#[tokio::test]
async fn test_list_passkeys_success() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock the credentials endpoint - the user_id in the token becomes keycloak_user_id
    mock_kc
        .mock_list_user_credentials_any(vec![
            ("cred-1", "webauthn"),
            ("cred-2", "webauthn-passwordless"),
        ])
        .await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token - the sub claim is used as keycloak_user_id
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_passkey_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<WebAuthnCredential>>>) =
        get_json_with_auth(&app, "/api/v1/me/passkeys", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let credentials = body.unwrap().data;
    assert_eq!(credentials.len(), 2);
}

#[tokio::test]
async fn test_list_passkeys_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock empty credentials list
    mock_kc.mock_list_user_credentials_any(vec![]).await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_passkey_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<WebAuthnCredential>>>) =
        get_json_with_auth(&app, "/api/v1/me/passkeys", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let credentials = body.unwrap().data;
    assert_eq!(credentials.len(), 0);
}

#[tokio::test]
async fn test_list_passkeys_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_passkey_test_router(state);

    // No auth header
    let (status, _): (StatusCode, Option<SuccessResponse<Vec<WebAuthnCredential>>>) =
        get_json(&app, "/api/v1/me/passkeys").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_passkeys_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_passkey_test_router(state);

    // Invalid token
    let (status, _): (StatusCode, Option<SuccessResponse<Vec<WebAuthnCredential>>>) =
        get_json_with_auth(&app, "/api/v1/me/passkeys", "invalid-token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Delete Passkey Tests
// ============================================================================

#[tokio::test]
async fn test_delete_passkey_success() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock the delete credential endpoint
    mock_kc.mock_delete_user_credential_success().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_passkey_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, "/api/v1/me/passkeys/cred-123", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_passkey_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock delete credential returning 404
    mock_kc.mock_delete_user_credential_not_found().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_passkey_test_router(state);

    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, "/api/v1/me/passkeys/nonexistent-cred", &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_passkey_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_passkey_test_router(state);

    // No auth header
    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, "/api/v1/me/passkeys/cred-123").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_passkey_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_passkey_test_router(state);

    // Invalid token
    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, "/api/v1/me/passkeys/cred-123", "invalid-token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
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

fn build_passkey_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::webauthn;
    use axum::routing::{delete, get};

    axum::Router::new()
        .route(
            "/api/v1/me/passkeys",
            get(webauthn::list_passkeys::<TestAppState>),
        )
        .route(
            "/api/v1/me/passkeys/{credential_id}",
            delete(webauthn::delete_passkey::<TestAppState>),
        )
        .with_state(state)
}
