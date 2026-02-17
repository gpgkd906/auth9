//! Invitation HTTP API handler tests
//!
//! Tests for invitation management endpoints.

use crate::support::http::{
    delete_json, get_json, get_json_with_auth, post_json, post_json_with_auth, MockKeycloakServer,
    TestAppState,
};
use crate::support::{
    create_test_identity_token, create_test_role, create_test_service, create_test_tenant,
};
use auth9_core::api::{MessageResponse, PaginatedResponse, SuccessResponse};
use auth9_core::domain::{Invitation, InvitationResponse, InvitationStatus, StringUuid};
use axum::http::StatusCode;
use chrono::Utc;

// ============================================================================
// List Invitations Tests
// ============================================================================

#[tokio::test]
async fn test_list_invitations_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_invitation_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/tenants/{}/invitations", tenant_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 0);
    assert_eq!(response.pagination.total, 0);
}

#[tokio::test]
async fn test_list_invitations_with_data() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Add invitations directly
    for i in 0..3 {
        let invitation = Invitation {
            id: StringUuid::new_v4(),
            tenant_id,
            email: format!("user{}@example.com", i),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            token_hash: format!("hash_{}", i),
            status: InvitationStatus::Pending,
            expires_at: Utc::now() + chrono::Duration::hours(72),
            accepted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        state.invitation_repo.add_invitation(invitation).await;
    }

    let app = build_invitation_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/tenants/{}/invitations", tenant_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 3);
}

#[tokio::test]
async fn test_list_invitations_filter_by_status() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Pending invitation
    state
        .invitation_repo
        .add_invitation(Invitation {
            id: StringUuid::new_v4(),
            tenant_id,
            email: "pending@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            token_hash: "hash_1".to_string(),
            status: InvitationStatus::Pending,
            expires_at: Utc::now() + chrono::Duration::hours(72),
            accepted_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await;

    // Accepted invitation
    state
        .invitation_repo
        .add_invitation(Invitation {
            id: StringUuid::new_v4(),
            tenant_id,
            email: "accepted@example.com".to_string(),
            role_ids: vec![],
            invited_by: StringUuid::new_v4(),
            token_hash: "hash_2".to_string(),
            status: InvitationStatus::Accepted,
            expires_at: Utc::now() + chrono::Duration::hours(72),
            accepted_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await;

    let app = build_invitation_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/tenants/{}/invitations?status=pending", tenant_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].email, "pending@example.com");
}

// ============================================================================
// Create Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_create_invitation_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Create a service and role for validation
    let service_id = uuid::Uuid::new_v4();
    let service = create_test_service(Some(service_id), Some(*tenant_id));
    state.service_repo.add_service(service).await;
    let role = create_test_role(None, service_id);
    let role_id = role.id;
    state.rbac_repo.add_role(role).await;

    let token = create_test_identity_token();
    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": [role_id.to_string()],
        "expires_in_hours": 48
    });

    let (status, body): (StatusCode, Option<SuccessResponse<InvitationResponse>>) =
        post_json_with_auth(
            &app,
            &format!("/api/v1/tenants/{}/invitations", tenant_id),
            &input,
            &token,
        )
        .await;

    // Should succeed with 201 Created
    assert_eq!(status, StatusCode::CREATED);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.email, "newuser@example.com");
    assert_eq!(response.status, InvitationStatus::Pending);
    assert_eq!(response.tenant_id, tenant_id);
}

#[tokio::test]
async fn test_create_invitation_no_auth() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": []
    });

    // POST without auth token
    let (status, _): (StatusCode, Option<SuccessResponse<InvitationResponse>>) = post_json(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
    )
    .await;

    // Should fail with 401
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Get Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_get_invitation_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let invitation = Invitation {
        id: StringUuid::new_v4(),
        tenant_id: StringUuid::new_v4(),
        email: "test@example.com".to_string(),
        role_ids: vec![],
        invited_by: StringUuid::new_v4(),
        token_hash: "hash".to_string(),
        status: InvitationStatus::Pending,
        expires_at: Utc::now() + chrono::Duration::hours(72),
        accepted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let invitation_id = invitation.id;
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<InvitationResponse>>) =
        get_json(&app, &format!("/api/v1/invitations/{}", invitation_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.email, "test@example.com");
}

#[tokio::test]
async fn test_get_invitation_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_invitation_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<SuccessResponse<InvitationResponse>>) =
        get_json(&app, &format!("/api/v1/invitations/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Revoke Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_revoke_invitation_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let invitation = Invitation {
        id: StringUuid::new_v4(),
        tenant_id: StringUuid::new_v4(),
        email: "revoke@example.com".to_string(),
        role_ids: vec![],
        invited_by: StringUuid::new_v4(),
        token_hash: "hash".to_string(),
        status: InvitationStatus::Pending,
        expires_at: Utc::now() + chrono::Duration::hours(72),
        accepted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let invitation_id = invitation.id;
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<InvitationResponse>>) = post_json(
        &app,
        &format!("/api/v1/invitations/{}/revoke", invitation_id),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.status, InvitationStatus::Revoked);
}

#[tokio::test]
async fn test_revoke_invitation_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_invitation_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<SuccessResponse<InvitationResponse>>) = post_json(
        &app,
        &format!("/api/v1/invitations/{}/revoke", nonexistent_id),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Delete Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_delete_invitation_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let invitation = Invitation {
        id: StringUuid::new_v4(),
        tenant_id: StringUuid::new_v4(),
        email: "delete@example.com".to_string(),
        role_ids: vec![],
        invited_by: StringUuid::new_v4(),
        token_hash: "hash".to_string(),
        status: InvitationStatus::Pending,
        expires_at: Utc::now() + chrono::Duration::hours(72),
        accepted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let invitation_id = invitation.id;
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, &format!("/api/v1/invitations/{}", invitation_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_invitation_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_invitation_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, &format!("/api/v1/invitations/{}", nonexistent_id)).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Resend Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_resend_invitation_no_email_provider() {
    // When email provider is not configured, resend should return 400
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let invitation = Invitation {
        id: StringUuid::new_v4(),
        tenant_id,
        email: "resend@example.com".to_string(),
        role_ids: vec![],
        invited_by: StringUuid::new_v4(),
        token_hash: "hash".to_string(),
        status: InvitationStatus::Pending,
        expires_at: Utc::now() + chrono::Duration::hours(72),
        accepted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let invitation_id = invitation.id;
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json(
        &app,
        &format!("/api/v1/invitations/{}/resend", invitation_id),
        &serde_json::json!({}),
    )
    .await;

    // Email provider not configured → 400 Bad Request
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_resend_invitation_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_invitation_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json(
        &app,
        &format!("/api/v1/invitations/{}/resend", nonexistent_id),
        &serde_json::json!({}),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_resend_revoked_invitation_fails() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let invitation = Invitation {
        id: StringUuid::new_v4(),
        tenant_id: StringUuid::new_v4(),
        email: "revoked@example.com".to_string(),
        role_ids: vec![],
        invited_by: StringUuid::new_v4(),
        token_hash: "hash".to_string(),
        status: InvitationStatus::Revoked,
        expires_at: Utc::now() + chrono::Duration::hours(72),
        accepted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let invitation_id = invitation.id;
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json(
        &app,
        &format!("/api/v1/invitations/{}/resend", invitation_id),
        &serde_json::json!({}),
    )
    .await;

    // Cannot resend non-pending invitation → 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// Accept Invitation Tests
// ============================================================================

#[tokio::test]
async fn test_accept_invitation_empty_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": ""
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    // Should fail with validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_accept_invitation_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": "nonexistent-token-abc123"
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    // Should fail because token doesn't match any invitation
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_list_invitations_service_client_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(uuid::Uuid::new_v4(), "svc@test.com", Some(*tenant_id))
        .unwrap();

    let app = build_invitation_test_router(state);

    let (status, _): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_invitations_non_admin_identity_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Non-admin identity token
    let token = crate::support::create_test_identity_token_for_user(uuid::Uuid::new_v4());
    let app = build_invitation_test_router(state);

    let (status, _): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_invitations_tenant_access_wrong_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let other_tenant_id = uuid::Uuid::new_v4();
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "user@test.com",
            other_tenant_id,
            "my-service",
            vec!["admin".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_invitation_test_router(state);

    let (status, _): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_invitations_tenant_access_same_tenant_succeeds() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "user@test.com",
            tenant_id,
            "my-service",
            vec!["admin".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_invitation_test_router(state);

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/tenants/{}/invitations", tenant_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_create_invitation_service_client_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_service_client_token(uuid::Uuid::new_v4(), "svc@test.com", Some(*tenant_id))
        .unwrap();

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": []
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_invitation_tenant_access_member_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = *tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // TenantAccess with member role (not admin/owner)
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "member@test.com",
            tenant_id,
            "my-service",
            vec!["member".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": []
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_invitation_cross_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let other_tenant_id = uuid::Uuid::new_v4();
    let jwt_manager = crate::support::create_test_jwt_manager();
    let token = jwt_manager
        .create_tenant_access_token(
            uuid::Uuid::new_v4(),
            "admin@test.com",
            other_tenant_id,
            "my-service",
            vec!["admin".to_string()],
            vec![],
        )
        .unwrap();

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": []
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_invitation_non_admin_identity_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = crate::support::create_test_identity_token_for_user(uuid::Uuid::new_v4());
    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": []
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_invitation_user_already_member_returns_409() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Add an existing user who is already a member
    use crate::support::create_test_user;
    let user_id = uuid::Uuid::new_v4();
    let mut user = create_test_user(Some(user_id));
    user.email = "existing@example.com".to_string();
    state.user_repo.add_user(user).await;

    let tu = auth9_core::domain::TenantUser {
        id: StringUuid::new_v4(),
        user_id: StringUuid::from(user_id),
        tenant_id,
        role_in_tenant: "member".to_string(),
        joined_at: Utc::now(),
    };
    state.user_repo.add_tenant_user(tu).await;

    let token = create_test_identity_token();
    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "email": "existing@example.com",
        "role_ids": []
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_invitation_invalid_role_returns_400() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = create_test_identity_token();
    let app = build_invitation_test_router(state);

    let nonexistent_role_id = StringUuid::new_v4();
    let input = serde_json::json!({
        "email": "newuser@example.com",
        "role_ids": [nonexistent_role_id.to_string()]
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// Pagination Tests
// ============================================================================

#[tokio::test]
async fn test_list_invitations_with_pagination() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Add 15 invitations
    for i in 0..15 {
        state
            .invitation_repo
            .add_invitation(Invitation {
                id: StringUuid::new_v4(),
                tenant_id,
                email: format!("user{}@example.com", i),
                role_ids: vec![],
                invited_by: StringUuid::new_v4(),
                token_hash: format!("hash_{}", i),
                status: InvitationStatus::Pending,
                expires_at: Utc::now() + chrono::Duration::hours(72),
                accepted_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .await;
    }

    let app = build_invitation_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json_with_auth(
            &app,
            &format!(
                "/api/v1/tenants/{}/invitations?page=1&per_page=5",
                tenant_id
            ),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.len(), 5);
    assert_eq!(response.pagination.total, 15);
    assert_eq!(response.pagination.page, 1);
    assert_eq!(response.pagination.per_page, 5);
}

// ============================================================================
// Accept Invitation Tests (comprehensive)
// ============================================================================

/// Helper to hash a token with argon2, same as InvitationService::generate_token
fn hash_token(token: &str) -> String {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(token.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn create_pending_invitation(
    tenant_id: StringUuid,
    email: &str,
    token: &str,
    role_ids: Vec<StringUuid>,
) -> Invitation {
    Invitation {
        id: StringUuid::new_v4(),
        tenant_id,
        email: email.to_string(),
        role_ids,
        invited_by: StringUuid::new_v4(),
        token_hash: hash_token(token),
        status: InvitationStatus::Pending,
        expires_at: Utc::now() + chrono::Duration::hours(72),
        accepted_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn test_accept_invitation_existing_user_success() {
    use auth9_core::domain::User;

    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = "test-accept-token-abc123";
    let invitation = create_pending_invitation(tenant_id, "existing@example.com", token, vec![]);
    state.invitation_repo.add_invitation(invitation).await;

    // Add existing user matching the invitation email
    let user = User {
        id: StringUuid::new_v4(),
        keycloak_id: "kc-user-1".to_string(),
        email: "existing@example.com".to_string(),
        display_name: Some("Existing User".to_string()),
        ..Default::default()
    };
    state.user_repo.add_user(user).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token
    });

    let (status, body): (StatusCode, Option<SuccessResponse<InvitationResponse>>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.status, InvitationStatus::Accepted);
}

#[tokio::test]
async fn test_accept_invitation_new_user_with_keycloak_creation() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_user_success("new-kc-user-id").await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = "test-new-user-token-xyz";
    let invitation = create_pending_invitation(tenant_id, "newuser@example.com", token, vec![]);
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token,
        "email": "newuser@example.com",
        "password": "SecurePass123!",
        "display_name": "New User"
    });

    let (status, body): (StatusCode, Option<SuccessResponse<InvitationResponse>>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.status, InvitationStatus::Accepted);
}

#[tokio::test]
async fn test_accept_invitation_new_user_without_password_fails() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = "test-no-password-token";
    let invitation = create_pending_invitation(tenant_id, "nopass@example.com", token, vec![]);
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    // User doesn't exist, no password provided → 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_accept_invitation_expired() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = "test-expired-token";
    let mut invitation = create_pending_invitation(tenant_id, "expired@example.com", token, vec![]);
    invitation.expires_at = Utc::now() - chrono::Duration::hours(1); // Already expired
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_accept_invitation_already_accepted() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = "test-accepted-token";
    let mut invitation =
        create_pending_invitation(tenant_id, "accepted@example.com", token, vec![]);
    invitation.status = InvitationStatus::Accepted;
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    // Accepted status is not Pending → won't be found by list_pending → 404
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_accept_invitation_email_mismatch() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let token = "test-mismatch-token";
    let invitation = create_pending_invitation(tenant_id, "correct@example.com", token, vec![]);
    state.invitation_repo.add_invitation(invitation).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token,
        "email": "wrong@example.com"
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_accept_invitation_with_role_assignment() {
    use auth9_core::domain::User;

    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let service = create_test_service(None, Some(*tenant_id));
    let service_id = service.id;
    state.service_repo.add_service(service).await;

    let role = create_test_role(None, *service_id);
    let role_id = role.id;
    state.rbac_repo.add_role(role).await;

    let token = "test-role-assign-token";
    let invitation =
        create_pending_invitation(tenant_id, "roleuser@example.com", token, vec![role_id]);
    state.invitation_repo.add_invitation(invitation).await;

    // Add existing user
    let user = User {
        id: StringUuid::new_v4(),
        keycloak_id: "kc-role-user".to_string(),
        email: "roleuser@example.com".to_string(),
        display_name: Some("Role User".to_string()),
        ..Default::default()
    };
    state.user_repo.add_user(user).await;

    let app = build_invitation_test_router(state);

    let input = serde_json::json!({
        "token": token
    });

    let (status, body): (StatusCode, Option<SuccessResponse<InvitationResponse>>) =
        post_json(&app, "/api/v1/invitations/accept", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    assert_eq!(response.data.status, InvitationStatus::Accepted);
}

// ============================================================================
// Create Invitation - Role Validation Tests
// ============================================================================

#[tokio::test]
async fn test_create_invitation_role_service_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    // Create a role but with a service_id that doesn't exist in client_service
    let service_id = uuid::Uuid::new_v4();
    let role = create_test_role(None, service_id);
    let role_id = role.id;
    state.rbac_repo.add_role(role).await;
    // Note: service NOT added to service_repo

    let app = build_invitation_test_router(state);
    let token = create_test_identity_token();

    let input = serde_json::json!({
        "email": "newmember@example.com",
        "role_ids": [role_id]
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    // Service not found for the role → 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_invitation_role_from_different_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant = create_test_tenant(None);
    let tenant_id = tenant.id;
    state.tenant_repo.add_tenant(tenant).await;

    let other_tenant = create_test_tenant(None);
    let other_tenant_id = other_tenant.id;
    state.tenant_repo.add_tenant(other_tenant).await;

    // Create a service belonging to a different tenant
    let service = create_test_service(None, Some(*other_tenant_id));
    let service_id = service.id;
    state.service_repo.add_service(service).await;

    let role = create_test_role(None, *service_id);
    let role_id = role.id;
    state.rbac_repo.add_role(role).await;

    let app = build_invitation_test_router(state);
    let token = create_test_identity_token();

    let input = serde_json::json!({
        "email": "cross-tenant@example.com",
        "role_ids": [role_id]
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/invitations", tenant_id),
        &input,
        &token,
    )
    .await;

    // Role belongs to a different tenant → 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_invitation_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::invitation;
    use axum::routing::{get, post};

    axum::Router::new()
        .route(
            "/api/v1/tenants/{tenant_id}/invitations",
            get(invitation::list::<TestAppState>).post(invitation::create::<TestAppState>),
        )
        .route(
            "/api/v1/invitations/{id}",
            get(invitation::get::<TestAppState>).delete(invitation::delete::<TestAppState>),
        )
        .route(
            "/api/v1/invitations/{id}/revoke",
            post(invitation::revoke::<TestAppState>),
        )
        .route(
            "/api/v1/invitations/{id}/resend",
            post(invitation::resend::<TestAppState>),
        )
        .route(
            "/api/v1/invitations/accept",
            post(invitation::accept::<TestAppState>),
        )
        .with_state(state)
}
