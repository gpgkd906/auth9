//! Password management HTTP API handler tests
//!
//! Tests for password reset and password change endpoints.

use super::{post_json, put_json, MockKeycloakServer, TestAppState};
use crate::api::create_test_user;
use auth9_core::api::{MessageResponse, SuccessResponse};
use auth9_core::domain::{PasswordPolicy, StringUuid};
use axum::http::StatusCode;

// ============================================================================
// Forgot Password Tests
// ============================================================================

#[tokio::test]
async fn test_forgot_password_existing_user_no_email_configured() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let mut user = create_test_user(None);
    user.email = "test@example.com".to_string();
    state.user_repo.add_user(user).await;

    // Build router
    let app = build_password_test_router(state);

    // Make request
    let input = serde_json::json!({
        "email": "test@example.com"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/password/forgot", &input).await;

    // Returns BAD_REQUEST because email provider is not configured in tests
    // In production with email configured, this would return OK
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_forgot_password_nonexistent_user() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    // Request reset for non-existent user
    let input = serde_json::json!({
        "email": "nonexistent@example.com"
    });
    let (status, body): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/password/forgot", &input).await;

    // Returns OK because user not found - returns early without trying to send email
    // This prevents email enumeration attacks
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_forgot_password_invalid_email_format() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "email": "not-an-email"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/password/forgot", &input).await;

    // Should return validation error (422 UNPROCESSABLE_ENTITY)
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Reset Password Tests
// ============================================================================

#[tokio::test]
async fn test_reset_password_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "token": "invalid-token",
        "new_password": "NewPassword123!"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/password/reset", &input).await;

    // Should return bad request for invalid token
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_reset_password_short_password() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "token": "some-token",
        "new_password": "short"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/password/reset", &input).await;

    // Should return validation error for short password (422 UNPROCESSABLE_ENTITY)
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Password Policy Tests
// ============================================================================

#[tokio::test]
async fn test_get_password_policy_default() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let tenant_id = StringUuid::new_v4();
    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = super::get_json(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let policy = body.unwrap().data;
    // Should return default policy
    assert_eq!(policy.min_length, 8);
}

#[tokio::test]
async fn test_update_password_policy() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let tenant_id = StringUuid::new_v4();
    let input = serde_json::json!({
        "min_length": 12,
        "require_uppercase": true,
        "require_lowercase": true,
        "require_numbers": true,
        "require_symbols": true,
        "max_age_days": 90,
        "history_count": 5,
        "lockout_threshold": 5,
        "lockout_duration_mins": 30
    });

    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let policy = body.unwrap().data;
    assert_eq!(policy.min_length, 12);
    assert!(policy.require_uppercase);
    assert!(policy.require_lowercase);
}

#[tokio::test]
async fn test_update_password_policy_invalid_min_length() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let tenant_id = StringUuid::new_v4();
    let input = serde_json::json!({
        "min_length": 3  // Too short, should be at least 6
    });

    let (status, _): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
    )
    .await;

    // 422 UNPROCESSABLE_ENTITY for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_password_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::password;
    use axum::routing::{get, post};

    axum::Router::new()
        .route(
            "/api/v1/password/forgot",
            post(password::forgot_password::<TestAppState>),
        )
        .route(
            "/api/v1/password/reset",
            post(password::reset_password::<TestAppState>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/password-policy",
            get(password::get_password_policy::<TestAppState>)
                .put(password::update_password_policy::<TestAppState>),
        )
        .with_state(state)
}
