//! Invitation HTTP API handler tests
//!
//! Tests for invitation management endpoints.

use super::{
    delete_json, get_json, post_json, post_json_with_auth, MockKeycloakServer, TestAppState,
};
use crate::api::{create_test_identity_token, create_test_tenant};
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

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json(&app, &format!("/api/v1/tenants/{}/invitations", tenant_id)).await;

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

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) =
        get_json(&app, &format!("/api/v1/tenants/{}/invitations", tenant_id)).await;

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

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) = get_json(
        &app,
        &format!("/api/v1/tenants/{}/invitations?status=pending", tenant_id),
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

    let token = create_test_identity_token();
    let app = build_invitation_test_router(state);

    let role_id = StringUuid::new_v4();
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

    let (status, body): (StatusCode, Option<PaginatedResponse<InvitationResponse>>) = get_json(
        &app,
        &format!(
            "/api/v1/tenants/{}/invitations?page=1&per_page=5",
            tenant_id
        ),
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
