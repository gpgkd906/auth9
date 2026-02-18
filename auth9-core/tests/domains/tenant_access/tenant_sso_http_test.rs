//! Enterprise SSO Connector API HTTP Handler Tests
//!
//! Tests for the tenant SSO endpoints (/api/v1/tenants/{tenant_id}/sso/connectors).
//! These tests cover:
//! - Authentication (401 without token)
//! - Access control (403 for wrong tenant / non-admin identity tokens)
//! - Input validation (invalid provider_type, missing domains, missing config fields)

use crate::support::http::{
    build_test_router, delete_json_with_auth, get_json_with_auth, post_json_with_auth,
    put_json_with_auth, TestAppState,
};
use crate::support::{
    create_test_identity_token_for_user, create_test_jwt_manager, create_test_tenant,
    create_test_tenant_access_token, MockKeycloakServer,
};
use axum::http::StatusCode;
use serde_json::{json, Value};
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_tenant_access_token(tenant_id: Uuid) -> String {
    let jwt_manager = create_test_jwt_manager();
    jwt_manager
        .create_tenant_access_token(
            Uuid::new_v4(),
            "user@example.com",
            tenant_id,
            "test-client",
            vec!["admin".to_string()],
            vec!["manage:sso".to_string()],
        )
        .unwrap()
}

fn sso_connectors_path(tenant_id: Uuid) -> String {
    format!("/api/v1/tenants/{}/sso/connectors", tenant_id)
}

fn sso_connector_path(tenant_id: Uuid, connector_id: Uuid) -> String {
    format!(
        "/api/v1/tenants/{}/sso/connectors/{}",
        tenant_id, connector_id
    )
}

fn valid_saml_create_input() -> Value {
    json!({
        "alias": "okta-saml",
        "display_name": "Okta SAML",
        "provider_type": "saml",
        "enabled": true,
        "priority": 100,
        "config": {
            "entityId": "https://sp.example.com",
            "singleSignOnServiceUrl": "https://idp.example.com/sso",
            "signingCertificate": "MIID..."
        },
        "domains": ["example.com"]
    })
}

// ============================================================================
// Authentication Tests (401)
// ============================================================================

#[tokio::test]
async fn test_list_connectors_unauthenticated_returns_401() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let tenant_id = Uuid::new_v4();

    let (status, _): (StatusCode, Option<Value>) =
        get_json_with_auth(&app, &sso_connectors_path(tenant_id), "invalid-token").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_connector_unauthenticated_returns_401() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);
    let tenant_id = Uuid::new_v4();

    let (status, _): (StatusCode, Option<Value>) = post_json_with_auth(
        &app,
        &sso_connectors_path(tenant_id),
        &valid_saml_create_input(),
        "invalid-token",
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Access Control Tests (403)
// ============================================================================

#[tokio::test]
async fn test_list_connectors_wrong_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let token_tenant_id = Uuid::new_v4();
    let request_tenant_id = Uuid::new_v4(); // different tenant
    let token = create_tenant_access_token(token_tenant_id);

    let (status, _): (StatusCode, Option<Value>) =
        get_json_with_auth(&app, &sso_connectors_path(request_tenant_id), &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_create_connector_wrong_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let token_tenant_id = Uuid::new_v4();
    let request_tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(token_tenant_id);

    let (status, _): (StatusCode, Option<Value>) = post_json_with_auth(
        &app,
        &sso_connectors_path(request_tenant_id),
        &valid_saml_create_input(),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_connector_wrong_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let token_tenant_id = Uuid::new_v4();
    let request_tenant_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let token = create_tenant_access_token(token_tenant_id);

    let (status, _): (StatusCode, Option<Value>) = put_json_with_auth(
        &app,
        &sso_connector_path(request_tenant_id, connector_id),
        &json!({"enabled": false}),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_delete_connector_wrong_tenant_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let token_tenant_id = Uuid::new_v4();
    let request_tenant_id = Uuid::new_v4();
    let connector_id = Uuid::new_v4();
    let token = create_tenant_access_token(token_tenant_id);

    let (status, _): (StatusCode, Option<Value>) = delete_json_with_auth(
        &app,
        &sso_connector_path(request_tenant_id, connector_id),
        &token,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_identity_token_non_admin_returns_403() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    // Identity token for a non-admin user
    let token = create_test_identity_token_for_user(Uuid::new_v4());
    let tenant_id = Uuid::new_v4();

    let (status, _): (StatusCode, Option<Value>) =
        get_json_with_auth(&app, &sso_connectors_path(tenant_id), &token).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ============================================================================
// Validation Tests (422/400) - Create Connector
// ============================================================================

#[tokio::test]
async fn test_create_connector_invalid_provider_type() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    let app = build_test_router(state);

    let input = json!({
        "alias": "my-ldap",
        "provider_type": "ldap",
        "config": {},
        "domains": ["example.com"]
    });

    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;
    // normalize_provider_type returns Validation error
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422, got {}",
        status
    );
}

#[tokio::test]
async fn test_create_connector_empty_alias() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    let app = build_test_router(state);

    let input = json!({
        "alias": "",
        "provider_type": "saml",
        "config": {
            "entityId": "https://sp.example.com",
            "singleSignOnServiceUrl": "https://idp.example.com/sso",
            "signingCertificate": "MIID..."
        },
        "domains": ["example.com"]
    });

    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422, got {}",
        status
    );
}

#[tokio::test]
async fn test_create_connector_empty_domains() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    let app = build_test_router(state);

    let input = json!({
        "alias": "okta-saml",
        "provider_type": "saml",
        "config": {
            "entityId": "https://sp.example.com",
            "singleSignOnServiceUrl": "https://idp.example.com/sso",
            "signingCertificate": "MIID..."
        },
        "domains": []
    });

    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422, got {}",
        status
    );
}

#[tokio::test]
async fn test_create_connector_invalid_domain_format() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    let app = build_test_router(state);

    let input = json!({
        "alias": "okta-saml",
        "provider_type": "saml",
        "config": {
            "entityId": "https://sp.example.com",
            "singleSignOnServiceUrl": "https://idp.example.com/sso",
            "signingCertificate": "MIID..."
        },
        "domains": ["user@example.com"]
    });

    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422, got {}",
        status
    );
}

#[tokio::test]
async fn test_create_connector_missing_saml_config_fields() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    let app = build_test_router(state);

    // Missing entityId and signingCertificate
    let input = json!({
        "alias": "okta-saml",
        "provider_type": "saml",
        "config": {
            "singleSignOnServiceUrl": "https://idp.example.com/sso"
        },
        "domains": ["example.com"]
    });

    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422, got {}",
        status
    );
}

#[tokio::test]
async fn test_create_connector_missing_oidc_config_fields() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    let app = build_test_router(state);

    // Missing required OIDC fields
    let input = json!({
        "alias": "google-oidc",
        "provider_type": "oidc",
        "config": {
            "clientId": "my-client"
        },
        "domains": ["example.com"]
    });

    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 400 or 422, got {}",
        status
    );
}

// ============================================================================
// Platform Admin Access Tests
// ============================================================================

#[tokio::test]
async fn test_platform_admin_can_access_any_tenant_sso() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();

    // Platform admin tenant access token (uses admin@auth9.local)
    let token = create_test_tenant_access_token();

    let app = build_test_router(state);

    // Platform admin should pass access control. The actual query will
    // fail due to no real DB, but status should NOT be 401 or 403.
    let (status, _): (StatusCode, Option<Value>) =
        get_json_with_auth(&app, &sso_connectors_path(tenant_id), &token).await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    assert_ne!(status, StatusCode::FORBIDDEN);
}

// ============================================================================
// SAML Config Alias Normalization Tests (via create endpoint)
// ============================================================================

#[tokio::test]
async fn test_create_connector_normalizes_saml_sso_url_alias() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let tenant_id = Uuid::new_v4();
    let token = create_tenant_access_token(tenant_id);

    // Seed a tenant so tenant_service().get() succeeds
    let mut tenant = create_test_tenant(Some(tenant_id));
    tenant.slug = "acme".to_string();
    state.tenant_repo.add_tenant(tenant).await;

    let app = build_test_router(state);

    // Use the aliased config keys: ssoUrl, certificate
    let input = json!({
        "alias": "okta-saml",
        "provider_type": "SAML",
        "config": {
            "entityId": "https://sp.example.com",
            "ssoUrl": "https://idp.example.com/sso",
            "certificate": "MIID..."
        },
        "domains": ["example.com"]
    });

    // This will pass validation (aliases are normalized) but may fail at DB/Keycloak layer.
    // We just verify it doesn't fail with a validation error.
    let (status, _): (StatusCode, Option<Value>) =
        post_json_with_auth(&app, &sso_connectors_path(tenant_id), &input, &token).await;

    // Should NOT be a validation error (400/422)
    assert_ne!(status, StatusCode::BAD_REQUEST);
    assert_ne!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
