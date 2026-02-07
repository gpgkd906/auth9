//! RBAC Cross-Service Integration Tests
//!
//! Verifies that permissions cannot be assigned to roles across different services.

use super::mock_keycloak::MockKeycloakServer;
use super::{build_test_router, post_json_with_auth, TestAppState};
use crate::api::{create_test_identity_token, create_test_permission, create_test_role};
use auth9_core::api::MessageResponse;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_assign_permission_different_service_fails() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let token = create_test_identity_token(); // Platform admin

    let service1_id = Uuid::new_v4();
    let service2_id = Uuid::new_v4();

    // Create Role in Service 1
    let role = create_test_role(None, service1_id);
    let role_id = role.id.0;
    state.rbac_repo.add_role(role).await;

    // Create Permission in Service 2
    let permission = create_test_permission(None, service2_id);
    let permission_id = permission.id.0;
    state.rbac_repo.add_permission(permission).await;

    let app = build_test_router(state);

    let input = json!({
        "permission_id": permission_id.to_string()
    });

    // Attempt assignment
    let (status, body): (StatusCode, Option<MessageResponse>) = post_json_with_auth(
        &app,
        &format!("/api/v1/roles/{}/permissions", role_id),
        &input,
        &token,
    )
    .await;

    // Verify failure (Bad Request)
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.is_some());
    let response = body.unwrap();
    assert!(response
        .message
        .contains("Cannot assign permission from service"));
}
