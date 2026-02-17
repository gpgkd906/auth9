//! Password management HTTP API handler tests
//!
//! Tests for password reset and password change endpoints.

use crate::support::http::{
    get_json_with_auth, post_json, post_json_with_auth, put_json_with_auth, MockKeycloakServer,
    TestAppState,
};
use crate::support::{create_test_identity_token, create_test_user};
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

    // Returns OK even when email provider is not configured to prevent email enumeration
    // The error is logged server-side but not exposed to the client
    assert_eq!(status, StatusCode::OK);
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

    // Create a tenant first so it can be found
    let tenant = crate::support::create_test_tenant(None);
    state.tenant_repo.add_tenant(tenant.clone()).await;

    let app = build_password_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant.id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let policy = body.unwrap().data;
    // Should return default policy (matches Keycloak realm settings)
    assert_eq!(policy.min_length, 12);
}

#[tokio::test]
async fn test_update_password_policy() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a tenant first so it can be found
    let tenant = crate::support::create_test_tenant(None);
    state.tenant_repo.add_tenant(tenant.clone()).await;

    let app = build_password_test_router(state);
    let token = create_test_identity_token();

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

    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant.id),
        &input,
        &token,
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
    let token = create_test_identity_token();

    let tenant_id = StringUuid::new_v4();
    let input = serde_json::json!({
        "min_length": 3  // Too short, should be at least 6
    });

    let (status, _): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
        &token,
    )
    .await;

    // 422 UNPROCESSABLE_ENTITY for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Password Policy Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_update_password_policy_member_forbidden() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = crate::support::create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Create a tenant access token with "member" role (not owner/admin)
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "member@test.com",
            tenant_id,
            "test-service",
            vec!["member".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "min_length": 6,
        "require_uppercase": false,
        "require_lowercase": false,
        "require_numbers": false,
        "require_symbols": false,
        "max_age_days": 0
    });

    let (status, _): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_password_policy_owner_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = crate::support::create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Create a tenant access token with "owner" role
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "owner@test.com",
            tenant_id,
            "test-service",
            vec!["owner".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "min_length": 12,
        "require_uppercase": true,
        "require_lowercase": true,
        "require_numbers": true,
        "require_symbols": true,
        "max_age_days": 90
    });

    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let policy = body.unwrap().data;
    assert_eq!(policy.min_length, 12);
}

#[tokio::test]
async fn test_update_password_policy_admin_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = crate::support::create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Create a tenant access token with "admin" role
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "admin@test.com",
            tenant_id,
            "test-service",
            vec!["admin".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "min_length": 10,
        "require_uppercase": true,
        "require_lowercase": true,
        "require_numbers": false,
        "require_symbols": false,
        "max_age_days": 60
    });

    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let policy = body.unwrap().data;
    assert_eq!(policy.min_length, 10);
}

#[tokio::test]
async fn test_update_password_policy_service_client_forbidden() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = crate::support::create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Create a service client token
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(uuid::Uuid::new_v4(), "service@test.com", Some(tenant_id))
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "min_length": 6,
        "require_uppercase": false,
        "require_lowercase": false,
        "require_numbers": false,
        "require_symbols": false,
        "max_age_days": 0
    });

    let (status, _): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = put_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_password_policy_member_allowed() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = crate::support::create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Members should be able to READ the policy (just not modify)
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "member@test.com",
            tenant_id,
            "test-service",
            vec!["member".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_password_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<PasswordPolicy>>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/password-policy", tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

// ============================================================================
// Change Password Tests
// ============================================================================

#[tokio::test]
async fn test_change_password_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    // No auth header
    let input = serde_json::json!({
        "current_password": "OldPassword123!",
        "new_password": "NewPassword123!"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json(&app, "/api/v1/password/change", &input).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "current_password": "OldPassword123!",
        "new_password": "NewPassword123!"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json_with_auth(&app, "/api/v1/password/change", &input, "invalid-token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_user_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a token for a user that doesn't exist in the repository
    let user_id = StringUuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "current_password": "OldPassword123!",
        "new_password": "NewPassword123!"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json_with_auth(&app, "/api/v1/password/change", &input, &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_change_password_validation_error_short_password() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "current_password": "OldPassword123!",
        "new_password": "short"  // Too short (less than 8 chars)
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json_with_auth(&app, "/api/v1/password/change", &input, &token).await;

    // 422 UNPROCESSABLE_ENTITY for validation errors
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_change_password_validation_error_empty_current() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_password_test_router(state);

    let input = serde_json::json!({
        "current_password": "",  // Empty current password
        "new_password": "NewPassword123!"
    });
    let (status, _): (StatusCode, Option<MessageResponse>) =
        post_json_with_auth(&app, "/api/v1/password/change", &input, &token).await;

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
            "/api/v1/password/change",
            post(password::change_password::<TestAppState>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/password-policy",
            get(password::get_password_policy::<TestAppState>)
                .put(password::update_password_policy::<TestAppState>),
        )
        .with_state(state)
}
