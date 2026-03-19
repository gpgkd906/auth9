//! WebAuthn/Passkey HTTP API handler tests
//!
//! Tests for WebAuthn credential management endpoints.

use crate::support::create_test_user;
use crate::support::http::{
    delete_json, delete_json_with_auth, get_json, get_json_with_auth, TestAppState,
};
use auth9_core::http_support::{MessageResponse, SuccessResponse};
use auth9_core::models::webauthn::WebAuthnCredential;
use axum::http::StatusCode;

// ============================================================================
// List Passkeys Tests
// ============================================================================

#[tokio::test]
async fn test_list_passkeys_success() {
    let state = TestAppState::new("http://localhost:8081");

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
    // NoOp credential store returns empty vec
    assert_eq!(credentials.len(), 0);
}

#[tokio::test]
async fn test_list_passkeys_empty() {
    let state = TestAppState::new("http://localhost:8081");

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

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
    let state = TestAppState::new("http://localhost:8081");

    let app = build_passkey_test_router(state);

    let (status, _): (StatusCode, Option<SuccessResponse<Vec<WebAuthnCredential>>>) =
        get_json(&app, "/api/v1/me/passkeys").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_passkeys_invalid_token() {
    let state = TestAppState::new("http://localhost:8081");

    let app = build_passkey_test_router(state);

    let (status, _): (StatusCode, Option<SuccessResponse<Vec<WebAuthnCredential>>>) =
        get_json_with_auth(&app, "/api/v1/me/passkeys", "invalid-token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Delete Passkey Tests
// ============================================================================

#[tokio::test]
async fn test_delete_passkey_keycloak_success() {
    let state = TestAppState::new("http://localhost:8081");

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_passkey_test_router(state);

    // Delete a Keycloak credential (prefixed with kc_)
    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, "/api/v1/me/passkeys/kc_cred-123", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_passkey_unauthorized() {
    let state = TestAppState::new("http://localhost:8081");

    let app = build_passkey_test_router(state);

    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, "/api/v1/me/passkeys/cred-123").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_passkey_invalid_token() {
    let state = TestAppState::new("http://localhost:8081");

    let app = build_passkey_test_router(state);

    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, "/api/v1/me/passkeys/cred-123", "invalid-token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_passkey_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::domains::identity::api::webauthn;
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
