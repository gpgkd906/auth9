//! Hosted Login API HTTP handler tests
//!
//! Tests for the /api/v1/hosted-login/* endpoints:
//! - password login (success, wrong password, user not found, invalid input)
//! - logout (with token, without token)
//! - start/complete password reset

use crate::support::http::{post_json, post_json_with_auth, MockKeycloakServer, TestAppState};
use crate::support::create_test_user;
use auth9_core::domains::identity::api::hosted_login::HostedLoginTokenResponse;
use auth9_core::http_support::MessageResponse;
use axum::http::StatusCode;

// ============================================================================
// Password Login Tests
// ============================================================================

#[tokio::test]
async fn test_password_login_success() {
    let mock_kc = MockKeycloakServer::new().await;

    // Mock Keycloak get-user (needed for validate_user_password → get_user)
    mock_kc
        .mock_get_user_success("kc-user-test")
        .await;
    // Mock Keycloak token endpoint (simulates valid password)
    mock_kc.mock_validate_password_valid().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user with known identity_subject
    let user = create_test_user(None);
    state.user_repo.add_user(user.clone()).await;

    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "test@example.com",
        "password": "CorrectPassword123!" // pragma: allowlist secret
    });
    let (status, body): (StatusCode, Option<HostedLoginTokenResponse>) =
        post_json(&app, "/api/v1/hosted-login/password", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let token_response = body.unwrap();
    assert_eq!(token_response.token_type, "Bearer");
    assert!(!token_response.access_token.is_empty());
    assert!(token_response.expires_in > 0);
}

#[tokio::test]
async fn test_password_login_wrong_password() {
    let mock_kc = MockKeycloakServer::new().await;

    // Mock Keycloak get-user + failed password validation
    mock_kc
        .mock_get_user_success("kc-user-test")
        .await;
    mock_kc.mock_validate_password_invalid().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    state.user_repo.add_user(user).await;

    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "test@example.com",
        "password": "WrongPassword!" // pragma: allowlist secret
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/password", &input).await;

    // Should return 401 with generic message (no enumeration)
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_password_login_user_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // No user added to repository
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "nonexistent@example.com",
        "password": "SomePassword123!" // pragma: allowlist secret
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/password", &input).await;

    // Should return 401 (same as wrong password — prevents email enumeration)
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_password_login_empty_email() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "",
        "password": "SomePassword123!" // pragma: allowlist secret
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/password", &input).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_password_login_invalid_email() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "not-an-email",
        "password": "SomePassword123!" // pragma: allowlist secret
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/password", &input).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_password_login_empty_password() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "test@example.com",
        "password": ""
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/password", &input).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// Hosted Logout Tests
// ============================================================================

#[tokio::test]
async fn test_hosted_logout_with_valid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock session deletion in Keycloak
    mock_kc.mock_logout_user_success().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a valid identity token with session
    let session_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            user_id,
            "test@example.com",
            Some("Test User"),
            Some(session_id),
        )
        .unwrap();

    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({});
    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json_with_auth(&app, "/api/v1/hosted-login/logout", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert_eq!(body.unwrap().message, "Logged out successfully.");
}

#[tokio::test]
async fn test_hosted_logout_without_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({});
    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/logout", &input).await;

    // Should still return OK (graceful degradation)
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert_eq!(body.unwrap().message, "Logged out successfully.");
}

#[tokio::test]
async fn test_hosted_logout_with_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({});
    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json_with_auth(&app, "/api/v1/hosted-login/logout", &input, "invalid-token").await;

    // Should still return OK (expired/invalid tokens are handled gracefully)
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

// ============================================================================
// Start Password Reset Tests
// ============================================================================

#[tokio::test]
async fn test_start_password_reset_existing_user() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let mut user = create_test_user(None);
    user.email = "test@example.com".to_string();
    state.user_repo.add_user(user).await;

    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "test@example.com"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/start-password-reset", &input).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_start_password_reset_unknown_user() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "nonexistent@example.com"
    });
    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/start-password-reset", &input).await;

    // Returns OK to prevent email enumeration
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_start_password_reset_invalid_email() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "email": "not-an-email"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/start-password-reset", &input).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Complete Password Reset Tests
// ============================================================================

#[tokio::test]
async fn test_complete_password_reset_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "token": "invalid-token",
        "new_password": "NewPassword123!" // pragma: allowlist secret
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/complete-password-reset", &input).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_complete_password_reset_short_password() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_hosted_login_test_router(state);

    let input = serde_json::json!({
        "token": "some-token",
        "new_password": "short" // pragma: allowlist secret
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/hosted-login/complete-password-reset", &input).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_hosted_login_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::domains::identity::api::hosted_login;
    use auth9_core::domains::identity::api::password;
    use axum::routing::post;

    axum::Router::new()
        .route(
            "/api/v1/hosted-login/password",
            post(hosted_login::password_login::<TestAppState>),
        )
        .route(
            "/api/v1/hosted-login/logout",
            post(hosted_login::hosted_logout::<TestAppState>),
        )
        .route(
            "/api/v1/hosted-login/start-password-reset",
            post(hosted_login::start_password_reset::<TestAppState>),
        )
        .route(
            "/api/v1/hosted-login/complete-password-reset",
            post(hosted_login::complete_password_reset::<TestAppState>),
        )
        // Also include existing password handlers used by password reset
        .route(
            "/api/v1/auth/forgot-password",
            post(password::forgot_password::<TestAppState>),
        )
        .route(
            "/api/v1/auth/reset-password",
            post(password::reset_password::<TestAppState>),
        )
        .with_state(state)
}
