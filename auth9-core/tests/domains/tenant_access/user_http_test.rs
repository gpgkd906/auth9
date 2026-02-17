//! User API HTTP Handler Tests
//!
//! Tests for the user HTTP endpoints using mock repositories and wiremock Keycloak.

use crate::support::http::{
    build_test_router, delete_json_with_auth, get_json_with_auth, post_json, post_json_with_auth,
    put_json_with_auth, TestAppState,
};
use crate::support::mock_keycloak::MockKeycloakServer;
use crate::support::{
    create_test_admin_token_for_user, create_test_identity_token,
    create_test_identity_token_for_user, create_test_user,
};
use auth9_core::api::{MessageResponse, PaginatedResponse, SuccessResponse};
use auth9_core::domain::{TenantUser, TenantUserWithTenant, User};
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// List Users Tests
// ============================================================================

#[tokio::test]
async fn test_list_users_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can list all users

    // Add some test users
    for i in 1..=3 {
        let mut user = create_test_user(None);
        user.email = format!("user{}@example.com", i);
        user.display_name = Some(format!("User {}", i));
        state.user_repo.add_user(user).await;
    }

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 3);
    assert_eq!(response.pagination.total, 3);
}

#[tokio::test]
async fn test_list_users_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token();

    // Add 25 users
    for i in 1..=25 {
        let mut user = create_test_user(None);
        user.email = format!("user{}@example.com", i);
        state.user_repo.add_user(user).await;
    }

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users?page=2&per_page=10", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 10);
    assert_eq!(response.pagination.total, 25);
    assert_eq!(response.pagination.page, 2);
}

#[tokio::test]
async fn test_list_users_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.is_empty());
    assert_eq!(response.pagination.total, 0);
}

// ============================================================================
// Get User Tests
// ============================================================================

#[tokio::test]
async fn test_get_user_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.email = "john@example.com".to_string();
    user.display_name = Some("John Doe".to_string());
    state.user_repo.add_user(user).await;

    // Users can view their own profile
    let token = create_test_admin_token_for_user(user_id);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        get_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.email, "john@example.com");
    assert_eq!(response.data.display_name, Some("John Doe".to_string()));
}

#[tokio::test]
async fn test_get_user_returns_404() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    // Use token with same user_id as the requested user to pass auth check
    let nonexistent_id = Uuid::new_v4();
    let token = create_test_admin_token_for_user(nonexistent_id);
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, &format!("/api/v1/users/{}", nonexistent_id), &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create User Tests
// ============================================================================

#[tokio::test]
async fn test_create_user_returns_201() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-12345";
    mock_kc.mock_create_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state.enable_public_registration().await;
    let app = build_test_router(state);

    let input = json!({
        "email": "newuser@example.com",
        "display_name": "New User",
        "password": "SecurePass123!"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        post_json(&app, "/api/v1/users", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.email, "newuser@example.com");
    assert_eq!(response.data.display_name, Some("New User".to_string()));
    assert_eq!(response.data.keycloak_id, keycloak_user_id);
}

#[tokio::test]
async fn test_create_user_without_password() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-no-pwd";
    mock_kc.mock_create_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state.enable_public_registration().await;
    let app = build_test_router(state);

    let input = json!({
        "email": "nopwd@example.com"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        post_json(&app, "/api/v1/users", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.email, "nopwd@example.com");
}

#[tokio::test]
async fn test_create_user_keycloak_conflict() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_user_conflict().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state.enable_public_registration().await;
    let app = build_test_router(state);

    let input = json!({
        "email": "existing@example.com"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/users", &input).await;

    // Should return 409 Conflict (translated from Keycloak error)
    assert_eq!(status, StatusCode::CONFLICT);
}

// ============================================================================
// Update User Tests
// ============================================================================

#[tokio::test]
async fn test_update_user_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-to-update";
    mock_kc.mock_update_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can update users

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    user.display_name = Some("Old Name".to_string());
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let input = json!({
        "display_name": "New Name"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        put_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.display_name, Some("New Name".to_string()));
}

#[tokio::test]
async fn test_update_user_avatar_only() {
    let mock_kc = MockKeycloakServer::new().await;
    // No Keycloak mock needed for avatar-only update (display_name not changed)

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can update users

    let user_id = Uuid::new_v4();
    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let input = json!({
        "avatar_url": "https://cdn.example.com/avatar.png"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        put_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(
        response.data.avatar_url,
        Some("https://cdn.example.com/avatar.png".to_string())
    );
}

#[tokio::test]
async fn test_update_user_returns_404() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can update users
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let input = json!({
        "display_name": "New Name"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        &format!("/api/v1/users/{}", nonexistent_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Delete User Tests
// ============================================================================

#[tokio::test]
async fn test_delete_user_returns_200() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-to-delete";
    mock_kc.mock_delete_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can delete users

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_user_keycloak_not_found_still_succeeds() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_delete_user_not_found().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can delete users

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = "nonexistent-kc-user".to_string();
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    // Should succeed even if Keycloak user doesn't exist (404 is ignored)
    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_delete_user_returns_404() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin can delete users
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}", nonexistent_id), &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// User-Tenant Association Tests
// ============================================================================

#[tokio::test]
async fn test_add_user_to_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let auth_user_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    // Auth user must be owner of the target tenant
    let owner_tu = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(auth_user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "owner".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(owner_tu).await;

    let token = create_test_admin_token_for_user(auth_user_id);
    let app = build_test_router(state);

    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "role_in_tenant": "admin"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<TenantUser>>) = post_json_with_auth(
        &app,
        &format!("/api/v1/users/{}/tenants", user_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.role_in_tenant, "admin");
}

#[tokio::test]
async fn test_remove_user_from_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let auth_user_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    // Auth user must be owner of the target tenant
    let owner_tu = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(auth_user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "owner".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(owner_tu).await;

    // Add target user to tenant
    let tenant_user = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "member".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(tenant_user).await;

    let token = create_test_admin_token_for_user(auth_user_id);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) = delete_json_with_auth(
        &app,
        &format!("/api/v1/users/{}/tenants/{}", user_id, tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.message.contains("removed"));
}

#[tokio::test]
async fn test_get_user_tenants() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let tenant_id1 = Uuid::new_v4();
    let tenant_id2 = Uuid::new_v4();

    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    // Add user to two tenants
    let tu1 = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id1),
        role_in_tenant: "admin".to_string(),
        joined_at: chrono::Utc::now(),
    };
    let tu2 = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id2),
        role_in_tenant: "member".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(tu1).await;
    state.user_repo.add_tenant_user(tu2).await;

    let app = build_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (
        StatusCode,
        Option<SuccessResponse<Vec<TenantUserWithTenant>>>,
    ) = get_json_with_auth(&app, &format!("/api/v1/users/{}/tenants", user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 2);
    // Verify tenant info is included
    assert!(response.data.iter().all(|tu| !tu.tenant.name.is_empty()));
}

#[tokio::test]
async fn test_list_users_by_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();

    // Create users
    let mut user1 = create_test_user(Some(user1_id));
    user1.email = "user1@example.com".to_string();
    let mut user2 = create_test_user(Some(user2_id));
    user2.email = "user2@example.com".to_string();

    state.user_repo.add_user(user1).await;
    state.user_repo.add_user(user2).await;

    // Add users to tenant
    let tu1 = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(user1_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "member".to_string(),
        joined_at: chrono::Utc::now(),
    };
    let tu2 = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(user2_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "admin".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(tu1).await;
    state.user_repo.add_tenant_user(tu2).await;

    let app = build_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<User>>>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/users", tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 2);
}

// ============================================================================
// MFA Tests
// ============================================================================

#[tokio::test]
async fn test_enable_mfa() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-mfa";
    let admin_id = Uuid::new_v4();
    mock_kc.setup_for_mfa_enable(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_admin_token_for_user(admin_id);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    user.mfa_enabled = false;
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) = post_json_with_auth(
        &app,
        &format!("/api/v1/users/{}/mfa", user_id),
        &json!({}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response.data.mfa_enabled);
}

#[tokio::test]
async fn test_disable_mfa() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-mfa-disable";
    let admin_id = Uuid::new_v4();
    mock_kc.setup_for_mfa_disable(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_admin_token_for_user(admin_id);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    user.mfa_enabled = true;
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}/mfa", user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(!response.data.mfa_enabled);
}

#[tokio::test]
async fn test_enable_mfa_user_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let admin_id = Uuid::new_v4();
    let admin_kc_id = admin_id.to_string();
    mock_kc.mock_get_user_success(&admin_kc_id).await;
    mock_kc.mock_validate_password_valid().await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_admin_token_for_user(admin_id);
    let app = build_test_router(state);

    let nonexistent_id = Uuid::new_v4();
    let (status, _body): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/users/{}/mfa", nonexistent_id),
        &json!({"current_password": "Admin123!"}),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// GET /api/v1/users/me Tests
// ============================================================================

#[tokio::test]
async fn test_get_me_returns_current_user() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.email = "me@example.com".to_string();
    user.display_name = Some("Me User".to_string());
    state.user_repo.add_user(user).await;

    let token = create_test_admin_token_for_user(user_id);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users/me", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.email, "me@example.com");
    assert_eq!(response.data.display_name, Some("Me User".to_string()));
}

// ============================================================================
// PUT /api/v1/users/me Tests
// ============================================================================

#[tokio::test]
async fn test_update_me_changes_display_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-me-update";
    mock_kc.mock_update_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    user.display_name = Some("Old Name".to_string());
    state.user_repo.add_user(user).await;

    let token = create_test_admin_token_for_user(user_id);
    let app = build_test_router(state);

    let input = json!({
        "display_name": "Updated Name"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        put_json_with_auth(&app, "/api/v1/users/me", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.display_name, Some("Updated Name".to_string()));
}

// ============================================================================
// Self-Update via PUT /api/v1/users/:id Tests
// ============================================================================

#[tokio::test]
async fn test_self_update_succeeds_without_admin() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-self-update";
    mock_kc.mock_update_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    user.display_name = Some("Original".to_string());
    state.user_repo.add_user(user).await;

    // Non-admin token for the same user
    let token = create_test_admin_token_for_user(user_id);
    let app = build_test_router(state);

    let input = json!({
        "display_name": "Self Updated"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        put_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.display_name, Some("Self Updated".to_string()));
}

#[tokio::test]
async fn test_update_other_user_requires_admin() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();
    let user = create_test_user(Some(other_user_id));
    state.user_repo.add_user(user).await;

    // Non-admin token for a different user
    let token = create_test_identity_token_for_user(user_id);
    let app = build_test_router(state);

    let input = json!({
        "display_name": "Hacked Name"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        &format!("/api/v1/users/{}", other_user_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_list_users_non_admin_identity_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Non-admin identity token (email not in platform_admin_emails)
    let token = create_test_identity_token_for_user(Uuid::new_v4());
    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/users", &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_users_service_client_with_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();

    // ServiceClient token with tenant context
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(Uuid::new_v4(), "svc@test.com", Some(tenant_id))
        .unwrap();

    let app = build_test_router(state);

    // Service client tokens are forbidden from listing users (least-privilege)
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/users", &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_users_service_client_no_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // ServiceClient token without tenant context
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(Uuid::new_v4(), "svc@test.com", None)
        .unwrap();

    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/users", &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_users_tenant_access_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let auth_user_id = Uuid::new_v4();

    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    let tu = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "member".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(tu).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            auth_user_id,
            "tenant-user@test.com",
            tenant_id,
            "my-service",
            vec!["admin".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_get_user_service_client_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(Uuid::new_v4(), "svc@test.com", Some(Uuid::new_v4()))
        .unwrap();

    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_user_tenant_access_no_admin_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let other_user_id = Uuid::new_v4();
    let user = create_test_user(Some(other_user_id));
    state.user_repo.add_user(user).await;

    // TenantAccess with member role (not admin)
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            user_id,
            "member@test.com",
            Uuid::new_v4(),
            "my-service",
            vec!["member".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, &format!("/api/v1/users/{}", other_user_id), &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_user_tenant_access_with_admin_role() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let other_user_id = Uuid::new_v4();
    let user = create_test_user(Some(other_user_id));
    state.user_repo.add_user(user).await;

    // TenantAccess with admin role
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            Uuid::new_v4(),
            "admin@test.com",
            Uuid::new_v4(),
            "my-service",
            vec!["admin".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        get_json_with_auth(&app, &format!("/api/v1/users/{}", other_user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_create_user_public_registration_disabled_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    // Don't call enable_public_registration - it's disabled by default
    let app = build_test_router(state);

    let input = json!({
        "email": "newuser@example.com"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/users", &input).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_user_with_invalid_auth_header_returns_401() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "email": "user@example.com"
    });

    // Use an invalid/expired token
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/users", &input, "invalid-token-here").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_user_admin_can_create_without_registration() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_user_success("kc-admin-created").await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    // Don't enable public registration - admin can always create
    let token = create_test_identity_token();
    let app = build_test_router(state);

    let input = json!({
        "email": "admin-created@example.com"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        post_json_with_auth(&app, "/api/v1/users", &input, &token).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_delete_user_non_admin_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    // Non-admin identity token
    let token = create_test_identity_token_for_user(Uuid::new_v4());
    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_user_service_client_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(Uuid::new_v4(), "svc@test.com", Some(Uuid::new_v4()))
        .unwrap();

    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_add_user_to_tenant_invalid_role_returns_400() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let auth_user_id = Uuid::new_v4();

    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    // Auth user is owner
    let owner_tu = TenantUser {
        id: auth9_core::domain::StringUuid::new_v4(),
        user_id: auth9_core::domain::StringUuid::from(auth_user_id),
        tenant_id: auth9_core::domain::StringUuid::from(tenant_id),
        role_in_tenant: "owner".to_string(),
        joined_at: chrono::Utc::now(),
    };
    state.user_repo.add_tenant_user(owner_tu).await;

    let token = create_test_admin_token_for_user(auth_user_id);
    let app = build_test_router(state);

    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "role_in_tenant": "superadmin"  // invalid role
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/users/{}/tenants", user_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_add_user_to_tenant_service_client_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let user = create_test_user(Some(user_id));
    state.user_repo.add_user(user).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(Uuid::new_v4(), "svc@test.com", Some(tenant_id))
        .unwrap();

    let app = build_test_router(state);

    let input = json!({
        "tenant_id": tenant_id.to_string(),
        "role_in_tenant": "member"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/users/{}/tenants", user_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_users_with_search() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token();

    let mut user1 = create_test_user(None);
    user1.email = "alice@example.com".to_string();
    user1.display_name = Some("Alice Wonderland".to_string());
    state.user_repo.add_user(user1).await;

    let mut user2 = create_test_user(None);
    user2.email = "bob@example.com".to_string();
    user2.display_name = Some("Bob Builder".to_string());
    state.user_repo.add_user(user2).await;

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users?search=alice", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].email, "alice@example.com");
}

#[tokio::test]
async fn test_list_users_with_empty_search() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token();

    let user = create_test_user(None);
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    // Empty search should return all users
    let (status, body): (StatusCode, Option<PaginatedResponse<User>>) =
        get_json_with_auth(&app, "/api/v1/users?search=", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 1);
}

#[tokio::test]
async fn test_tenant_access_with_user_write_permission_can_delete() {
    let mock_kc = MockKeycloakServer::new().await;
    let keycloak_user_id = "kc-user-to-delete-by-perm";
    mock_kc.mock_delete_user_success(keycloak_user_id).await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.keycloak_id = keycloak_user_id.to_string();
    state.user_repo.add_user(user).await;

    // TenantAccess with user:delete permission (not admin role)
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            Uuid::new_v4(),
            "permuser@test.com",
            Uuid::new_v4(),
            "my-service",
            vec!["member".to_string()],
            vec!["user:delete".to_string()],
        )
        .unwrap();

    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, &format!("/api/v1/users/{}", user_id), &token).await;

    assert_eq!(status, StatusCode::OK);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_create_user_with_special_email() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_user_success("kc-special-email").await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state.enable_public_registration().await;
    let app = build_test_router(state);

    let input = json!({
        "email": "user+tag@example.com",
        "display_name": "User with Tagged Email"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        post_json(&app, "/api/v1/users", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.email, "user+tag@example.com");
}

#[tokio::test]
async fn test_create_user_minimal() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_user_success("kc-minimal").await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state.enable_public_registration().await;
    let app = build_test_router(state);

    // Only email, no display_name or password
    let input = json!({
        "email": "minimal@example.com"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<User>>) =
        post_json(&app, "/api/v1/users", &input).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.email, "minimal@example.com");
    assert!(response.data.display_name.is_none());
}
