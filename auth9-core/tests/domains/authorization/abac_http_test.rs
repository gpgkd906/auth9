use crate::support::create_test_jwt_manager;
use crate::support::http::{
    build_test_router, get_json_with_auth, post_json_with_auth, TestAppState,
};
use crate::support::mock_keycloak::MockKeycloakServer;
use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

fn create_member_token_for_tenant(tenant_id: Uuid) -> String {
    let jwt_manager = create_test_jwt_manager();
    jwt_manager
        .create_tenant_access_token(
            Uuid::new_v4(),
            "member@auth9.local",
            tenant_id,
            "auth9-test-service",
            vec!["member".to_string()],
            vec!["user:read".to_string()],
        )
        .expect("failed to create member token")
}

#[tokio::test]
async fn test_abac_list_policies_forbidden_without_permission() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let tenant_id = Uuid::new_v4();
    let token = create_member_token_for_tenant(tenant_id);

    let (status, _body): (StatusCode, Option<serde_json::Value>) = get_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/abac/policies", tenant_id),
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_abac_create_policy_forbidden_without_permission() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let tenant_id = Uuid::new_v4();
    let token = create_member_token_for_tenant(tenant_id);
    let input = json!({
        "change_note": "member should not create",
        "policy": { "rules": [] }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/abac/policies", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_abac_simulate_forbidden_without_permission() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let tenant_id = Uuid::new_v4();
    let token = create_member_token_for_tenant(tenant_id);
    let input = json!({
        "simulation": {
            "action": "user_manage",
            "resource_type": "tenant",
            "subject": { "roles": ["member"] },
            "resource": { "tenant_id": tenant_id.to_string() },
            "request": {},
            "env": { "hour": 10 }
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        &format!("/api/v1/tenants/{}/abac/simulate", tenant_id),
        &input,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}
