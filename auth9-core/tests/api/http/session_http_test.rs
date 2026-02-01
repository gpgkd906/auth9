//! Session management HTTP API handler tests
//!
//! Tests for session listing and revocation endpoints.

use super::{get_json, post_json, MockKeycloakServer, TestAppState};
use crate::api::create_test_user;
use auth9_core::api::session::RevokeSessionsResponse;
use auth9_core::api::SuccessResponse;
use auth9_core::domain::{Session, SessionInfo, StringUuid};
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
        get_json(&app, &format!("/api/v1/admin/users/{}/sessions", user_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let sessions = body.unwrap().data;
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_list_user_sessions_admin_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Add a test user with no sessions
    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let app = build_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<SessionInfo>>>) =
        get_json(&app, &format!("/api/v1/admin/users/{}/sessions", user_id)).await;

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

    let (status, body): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) = post_json(
        &app,
        &format!("/api/v1/admin/users/{}/logout", user_id),
        &(),
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

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    let app = build_session_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<RevokeSessionsResponse>>) = post_json(
        &app,
        &format!("/api/v1/admin/users/{}/logout", user_id),
        &(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap().data;
    assert_eq!(response.revoked_count, 0);
}

// ============================================================================
// Session Info Tests
// ============================================================================

#[tokio::test]
async fn test_session_info_device_details() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

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
        get_json(&app, &format!("/api/v1/admin/users/{}/sessions", user_id)).await;

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
// Test Router Builder
// ============================================================================

fn build_session_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::session;
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
