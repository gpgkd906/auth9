//! Session management HTTP API handler tests
//!
//! Tests for session listing and revocation endpoints.

use crate::support::create_test_user;
use crate::support::http::{
    delete_json_with_auth, get_json, get_json_with_auth, post_json, post_json_with_auth,
    MockKeycloakServer, TestAppState,
};
use auth9_core::api::{MessageResponse, SuccessResponse};
use auth9_core::domain::{Session, SessionInfo, StringUuid};
use auth9_core::domains::identity::api::session::RevokeSessionsResponse;
use auth9_core::repository::SessionRepository;
use axum::http::StatusCode;
use chrono::Utc;

// ============================================================================
// List Sessions Tests
// ============================================================================

#[tokio::test]
async fn test_list_user_sessions_admin() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(
            uuid::Uuid::new_v4(),
            "admin@auth9.local",
            Some("Platform Admin"),
        )
        .unwrap();

    // Add a test user and sessions
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add some sessions
    for i in 0..3 {
        let session = Session {
            id: StringUuid::new_v4(),
            user_id,
            keycloak_session_id: Some(format!("kc-session-{}", i)),
            device_type: Some("desktop".to_string()),
            device_name: Some(format!("Chrome on macOS {}", i)),
            ip_address: Some("192.168.1.1".to_string()),
            location: Some("San Francisco, US".to_string()),
            user_agent: None,
            last_active_at: Utc::now(),
            created_at: Utc::now(),
            revoked_at: None,
        };
        state.session_repo.add_session(session).await;
    }

    let app = build_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/admin/users/{}/sessions", user_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let sessions = body.unwrap().data;
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_list_user_sessions_admin_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(
            uuid::Uuid::new_v4(),
            "admin@auth9.local",
            Some("Platform Admin"),
        )
        .unwrap();

    // Add a test user with no sessions
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let app = build_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/admin/users/{}/sessions", user_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let sessions = body.unwrap().data;
    assert_eq!(sessions.len(), 0);
}

// ============================================================================
// Force Logout Tests
// ============================================================================

#[tokio::test]
async fn test_force_logout_user() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock the logout endpoint in Keycloak
    mock_kc.mock_logout_user_success().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(
            uuid::Uuid::new_v4(),
            "admin@auth9.local",
            Some("Platform Admin"),
        )
        .unwrap();

    // Add a test user and sessions
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add some active sessions
    for i in 0..3 {
        let session = Session {
            id: StringUuid::new_v4(),
            user_id,
            keycloak_session_id: Some(format!("kc-session-{}", i)),
            device_type: Some("desktop".to_string()),
            device_name: None,
            ip_address: None,
            location: None,
            user_agent: None,
            last_active_at: Utc::now(),
            created_at: Utc::now(),
            revoked_at: None,
        };
        state.session_repo.add_session(session).await;
    }

    let app = build_session_test_router(state.clone());

    let (status, body): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) =
        post_json_with_auth(
            &app,
            &format!("/api/v1/admin/users/{}/logout", user_id),
            &(),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.revoked_count, 3);

    // Verify sessions are revoked
    let remaining = state
        .session_repo
        .list_active_by_user(user_id)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 0);
}

#[tokio::test]
async fn test_force_logout_user_no_sessions() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock the logout endpoint in Keycloak
    mock_kc.mock_logout_user_success().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(
            uuid::Uuid::new_v4(),
            "admin@auth9.local",
            Some("Platform Admin"),
        )
        .unwrap();

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let app = build_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) =
        post_json_with_auth(
            &app,
            &format!("/api/v1/admin/users/{}/logout", user_id),
            &(),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.revoked_count, 0);
}

#[tokio::test]
async fn test_force_logout_user_rejects_non_admin() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a tenant access token with viewer role (non-admin)
    let tenant_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "jane@example.com",
            tenant_id,
            "test-client",
            vec!["viewer".to_string()],
            vec![],
        )
        .unwrap();

    let target_user = create_test_user(None);
    let target_user_id = target_user.id;
    state.user_repo.add_user(target_user).await;

    let app = build_session_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/admin/users/{}/logout", target_user_id),
        &(),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ============================================================================
// Session Info Tests
// ============================================================================

#[tokio::test]
async fn test_session_info_device_details() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = state
        .jwt_manager
        .create_identity_token(
            uuid::Uuid::new_v4(),
            "admin@auth9.local",
            Some("Platform Admin"),
        )
        .unwrap();

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let session = Session {
        id: StringUuid::new_v4(),
        user_id,
        keycloak_session_id: Some("kc-123".to_string()),
        device_type: Some("mobile".to_string()),
        device_name: Some("Safari on iPhone".to_string()),
        ip_address: Some("10.0.0.1".to_string()),
        location: Some("New York, US".to_string()),
        user_agent: Some("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)".to_string()),
        last_active_at: Utc::now(),
        created_at: Utc::now(),
        revoked_at: None,
    };
    state.session_repo.add_session(session).await;

    let app = build_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json_with_auth(
            &app,
            &format!("/api/v1/admin/users/{}/sessions", user_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let sessions = body.unwrap().data;
    assert_eq!(sessions.len(), 1);

    let session_info = &sessions[0];
    assert_eq!(session_info.device_type, Some("mobile".to_string()));
    assert_eq!(
        session_info.device_name,
        Some("Safari on iPhone".to_string())
    );
    assert_eq!(session_info.ip_address, Some("10.0.0.1".to_string()));
    assert_eq!(session_info.location, Some("New York, US".to_string()));
}

// ============================================================================
// My Sessions Tests (Authenticated User)
// ============================================================================

#[tokio::test]
async fn test_list_my_sessions_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add sessions for this user
    let current_session_id = StringUuid::new_v4();
    for i in 0..3 {
        let session = Session {
            id: if i == 0 {
                current_session_id
            } else {
                StringUuid::new_v4()
            },
            user_id,
            keycloak_session_id: Some(format!("kc-session-{}", i)),
            device_type: Some("desktop".to_string()),
            device_name: Some(format!("Chrome on macOS {}", i)),
            ip_address: Some("192.168.1.1".to_string()),
            location: Some("San Francisco, US".to_string()),
            user_agent: None,
            last_active_at: Utc::now(),
            created_at: Utc::now(),
            revoked_at: None,
        };
        state.session_repo.add_session(session).await;
    }

    // Create a valid JWT token with session ID
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(*current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json_with_auth(&app, "/api/v1/me/sessions", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let sessions = body.unwrap().data;
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_list_my_sessions_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user with no sessions
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token with session ID
    let current_session_id = uuid::Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json_with_auth(&app, "/api/v1/me/sessions", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let sessions = body.unwrap().data;
    assert_eq!(sessions.len(), 0);
}

#[tokio::test]
async fn test_list_my_sessions_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_my_session_test_router(state);

    // No auth header
    let (status, _): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json(&app, "/api/v1/me/sessions").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_my_sessions_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_my_session_test_router(state);

    // Invalid token
    let (status, _): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json_with_auth(&app, "/api/v1/me/sessions", "invalid-token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_revoke_session_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add a current session (the one tied to the token)
    let current_session_id = StringUuid::new_v4();
    let current_session = Session {
        id: current_session_id,
        user_id,
        keycloak_session_id: Some("kc-current-session".to_string()),
        device_type: Some("desktop".to_string()),
        device_name: None,
        ip_address: None,
        location: None,
        user_agent: None,
        last_active_at: Utc::now(),
        created_at: Utc::now(),
        revoked_at: None,
    };
    state.session_repo.add_session(current_session).await;

    // Add a session to revoke
    let session_id = StringUuid::new_v4();
    let session = Session {
        id: session_id,
        user_id,
        keycloak_session_id: Some("kc-session-to-revoke".to_string()),
        device_type: Some("desktop".to_string()),
        device_name: None,
        ip_address: None,
        location: None,
        user_agent: None,
        last_active_at: Utc::now(),
        created_at: Utc::now(),
        revoked_at: None,
    };
    state.session_repo.add_session(session).await;

    // Create a valid JWT token with session ID
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(*current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state.clone());

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json_with_auth(&app, &format!("/api/v1/me/sessions/{}", session_id), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("revoked"));

    // Verify only the target session is revoked (current session should remain)
    let remaining = state
        .session_repo
        .list_active_by_user(user_id)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, current_session_id);
}

#[tokio::test]
async fn test_revoke_current_session_rejected() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let current_session_id = StringUuid::new_v4();
    let current_session = Session {
        id: current_session_id,
        user_id,
        keycloak_session_id: Some("kc-current".to_string()),
        device_type: Some("desktop".to_string()),
        device_name: Some("Chrome".to_string()),
        ip_address: None,
        location: None,
        user_agent: None,
        last_active_at: Utc::now(),
        created_at: Utc::now(),
        revoked_at: None,
    };
    state.session_repo.add_session(current_session).await;

    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(*current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state.clone());

    #[derive(serde::Deserialize)]
    struct ErrorResponse {
        error: String,
        message: String,
    }

    let (status, body): (StatusCode, Option<ErrorResponse>) = delete_json_with_auth(
        &app,
        &format!("/api/v1/me/sessions/{}", current_session_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.is_some());
    let error = body.unwrap();
    assert_eq!(error.error, "bad_request");
    assert!(error.message.contains("current session"));

    let sessions = state
        .session_repo
        .list_active_by_user(user_id)
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
}

#[tokio::test]
async fn test_revoke_session_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user (but no sessions)
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token with session ID
    let current_session_id = uuid::Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state);

    let nonexistent_id = StringUuid::new_v4();
    let (status, _): (StatusCode, Option<MessageResponse>) = delete_json_with_auth(
        &app,
        &format!("/api/v1/me/sessions/{}", nonexistent_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_revoke_session_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_my_session_test_router(state);

    let session_id = StringUuid::new_v4();
    // No auth header - use regular delete
    let (status, _): (StatusCode, Option<MessageResponse>) =
        crate::support::http::delete_json(&app, &format!("/api/v1/me/sessions/{}", session_id))
            .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_revoke_other_sessions_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add current session (the one tied to the token)
    let current_session_id = StringUuid::new_v4();
    let current_session = Session {
        id: current_session_id,
        user_id,
        keycloak_session_id: Some("kc-current-session".to_string()),
        device_type: Some("desktop".to_string()),
        device_name: None,
        ip_address: None,
        location: None,
        user_agent: None,
        last_active_at: Utc::now(),
        created_at: Utc::now(),
        revoked_at: None,
    };
    state.session_repo.add_session(current_session).await;

    // Add multiple other sessions
    for i in 0..3 {
        let session = Session {
            id: StringUuid::new_v4(),
            user_id,
            keycloak_session_id: Some(format!("kc-session-{}", i)),
            device_type: Some("desktop".to_string()),
            device_name: None,
            ip_address: None,
            location: None,
            user_agent: None,
            last_active_at: Utc::now(),
            created_at: Utc::now(),
            revoked_at: None,
        };
        state.session_repo.add_session(session).await;
    }

    // Create a valid JWT token with session ID
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(*current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state.clone());

    let (status, body): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) =
        post_json_with_auth(&app, "/api/v1/me/sessions/revoke-others", &(), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    // The current session is excluded (identified by session_id in token)
    // so all 3 other sessions should be revoked
    let response = body.unwrap().data;
    assert_eq!(response.revoked_count, 3);

    // Verify only current session remains
    let remaining = state
        .session_repo
        .list_active_by_user(user_id)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, current_session_id);
}

#[tokio::test]
async fn test_revoke_other_sessions_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_my_session_test_router(state);

    // No auth header
    let (status, _): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) =
        post_json(&app, "/api/v1/me/sessions/revoke-others", &()).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_revoke_other_sessions_no_other_sessions() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user with no sessions
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token with session ID
    let current_session_id = uuid::Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            *user_id,
            "test@example.com",
            Some("Test User"),
            Some(current_session_id),
        )
        .unwrap();

    let app = build_my_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) =
        post_json_with_auth(&app, "/api/v1/me/sessions/revoke-others", &(), &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.revoked_count, 0);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_session_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::domains::identity::api::session;
    use axum::routing::{get, post};

    axum::Router::new()
        .route(
            "/api/v1/admin/users/{user_id}/sessions",
            get(session::list_user_sessions::<TestAppState>),
        )
        .route(
            "/api/v1/admin/users/{user_id}/logout",
            post(session::force_logout_user::<TestAppState>),
        )
        .with_state(state)
}

fn build_my_session_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::domains::identity::api::session;
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route(
            "/api/v1/me/sessions",
            get(session::list_my_sessions::<TestAppState>),
        )
        .route(
            "/api/v1/me/sessions/{session_id}",
            delete(session::revoke_session::<TestAppState>),
        )
        .route(
            "/api/v1/me/sessions/revoke-others",
            post(session::revoke_other_sessions::<TestAppState>),
        )
        .with_state(state)
}
