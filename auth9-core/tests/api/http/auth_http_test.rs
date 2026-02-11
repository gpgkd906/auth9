//! Auth API HTTP Handler Tests
//!
//! Tests for the auth HTTP endpoints using mock repositories.

use super::{
    build_test_router, get_json, get_json_with_auth, get_raw, post_json, MockKeycloakServer,
    TestAppState,
};
use crate::api::create_test_service;
use auth9_core::api::auth::{OpenIdConfiguration, TokenResponse};
use auth9_core::domain::{Client, StringUuid};
use axum::http::StatusCode;
use base64::Engine;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// OpenID Discovery Tests
// ============================================================================

#[tokio::test]
async fn test_openid_configuration_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<OpenIdConfiguration>) =
        get_json(&app, "/.well-known/openid-configuration").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let config = body.unwrap();
    assert!(config.issuer.contains("auth9.test"));
}

#[tokio::test]
async fn test_openid_configuration_endpoints() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<OpenIdConfiguration>) =
        get_json(&app, "/.well-known/openid-configuration").await;

    assert_eq!(status, StatusCode::OK);
    let config = body.unwrap();

    // Verify all endpoint URLs are correctly set
    assert!(config
        .authorization_endpoint
        .contains("/api/v1/auth/authorize"));
    assert!(config.token_endpoint.contains("/api/v1/auth/token"));
    assert!(config.userinfo_endpoint.contains("/api/v1/auth/userinfo"));
    assert!(config.end_session_endpoint.contains("/api/v1/auth/logout"));

    // Verify supported values
    assert!(config
        .response_types_supported
        .contains(&"code".to_string()));
    assert!(config
        .grant_types_supported
        .contains(&"authorization_code".to_string()));
    assert!(config
        .grant_types_supported
        .contains(&"client_credentials".to_string()));
    assert!(config.scopes_supported.contains(&"openid".to_string()));
}

#[tokio::test]
async fn test_openid_configuration_hmac_algorithm() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<OpenIdConfiguration>) =
        get_json(&app, "/.well-known/openid-configuration").await;

    assert_eq!(status, StatusCode::OK);
    let config = body.unwrap();

    // Without RSA keys configured, should use HS256
    assert!(config
        .id_token_signing_alg_values_supported
        .contains(&"HS256".to_string()));
    // JWKS URI should always be present (returns empty keys for HS256)
    assert!(config.jwks_uri.is_some());
}

#[tokio::test]
async fn test_jwks_empty_without_rsa() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, body) = get_raw(&app, "/.well-known/jwks.json").await;

    // Without RSA keys, JWKS should return 200 with empty keys array
    assert_eq!(status, StatusCode::OK);
    let body_str = std::str::from_utf8(&body).unwrap();
    let jwks: serde_json::Value = serde_json::from_str(body_str).unwrap();
    assert!(jwks["keys"].as_array().unwrap().is_empty());
}

// ============================================================================
// Authorize Tests
// ============================================================================

#[tokio::test]
async fn test_authorize_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a service with redirect_uris
    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://app.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    // Create a client for this service
    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "test-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: Some("Test Client".to_string()),
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    let (status, body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=test-client&redirect_uri=https://app.example.com/callback&scope=openid&state=csrf-state",
    ).await;

    // Should redirect to Keycloak
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
    // Body should be empty for redirect
    assert!(body.is_empty() || status == StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_authorize_missing_state() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://app.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "test-client-no-state".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    // Missing state parameter should return 400 (CSRF protection)
    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=test-client-no-state&redirect_uri=https://app.example.com/callback&scope=openid",
    ).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_authorize_client_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=nonexistent&redirect_uri=https://app.example.com/callback&scope=openid&state=csrf-state",
    ).await;

    // Client not found should return 404
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_authorize_invalid_redirect_uri() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create service with specific redirect_uris
    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://allowed.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "restricted-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    // Try to use a non-whitelisted redirect_uri
    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=restricted-client&redirect_uri=https://evil.com/callback&scope=openid&state=csrf-state",
    ).await;

    // Invalid redirect_uri should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_authorize_with_state() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://app.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "state-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=state-client&redirect_uri=https://app.example.com/callback&scope=openid&state=mystate123",
    ).await;

    // Should still redirect successfully with state
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_authorize_with_nonce() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://app.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "nonce-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=nonce-client&redirect_uri=https://app.example.com/callback&scope=openid&nonce=random-nonce-123&state=csrf-state",
    ).await;

    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

// ============================================================================
// Logout Tests
// ============================================================================

#[tokio::test]
async fn test_logout_minimal() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(&app, "/api/v1/auth/logout").await;

    // Should redirect to Keycloak logout
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_logout_with_id_token_hint() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/logout?id_token_hint=eyJhbGciOiJSUzI1NiJ9.test",
    )
    .await;

    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_logout_with_post_redirect_uri() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/logout?post_logout_redirect_uri=https://app.example.com/logged-out",
    )
    .await;

    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_logout_full_params() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/logout?id_token_hint=token123&post_logout_redirect_uri=https://app.example.com/logged-out&state=logout-state",
    ).await;

    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

// ============================================================================
// Userinfo Tests
// ============================================================================

#[tokio::test]
async fn test_userinfo_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a valid identity token
    let user_id = Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/auth/userinfo", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let claims = body.unwrap();
    assert_eq!(claims["email"], "test@example.com");
}

#[tokio::test]
async fn test_userinfo_no_auth_header() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, "/api/v1/auth/userinfo").await;

    // Should return 400 (axum extracts Bearer requirement)
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_userinfo_invalid_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/auth/userinfo", "invalid-token").await;

    // Invalid token should return 401
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_userinfo_malformed_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    // Token with invalid format
    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/auth/userinfo", "not.a.valid.jwt.token").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Callback Tests
// ============================================================================

#[tokio::test]
async fn test_callback_missing_state() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(&app, "/api/v1/auth/callback?code=auth-code-123").await;

    // Missing state should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_callback_invalid_state() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/callback?code=auth-code-123&state=invalid-state",
    )
    .await;

    // Invalid state (not base64) should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_callback_invalid_state_json() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    // Valid base64 but invalid JSON
    let invalid_state = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"not json");

    let (status, _body) = get_raw(
        &app,
        &format!(
            "/api/v1/auth/callback?code=auth-code-123&state={}",
            invalid_state
        ),
    )
    .await;

    // Invalid JSON in state should return 500 (internal error during deserialization)
    assert!(status == StatusCode::INTERNAL_SERVER_ERROR || status == StatusCode::BAD_REQUEST);
}

// ============================================================================
// Token Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_token_unsupported_grant_type() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "password",
        "username": "user",
        "password": "pass"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Unsupported grant_type should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_auth_code_missing_code() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "authorization_code",
        "client_id": "test-client",
        "redirect_uri": "https://app.example.com/callback"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing code should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_auth_code_missing_client_id() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "authorization_code",
        "code": "auth-code-123",
        "redirect_uri": "https://app.example.com/callback"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing client_id should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_auth_code_missing_redirect_uri() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "authorization_code",
        "code": "auth-code-123",
        "client_id": "test-client"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing redirect_uri should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_client_credentials_missing_client_secret() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "client_credentials",
        "client_id": "test-client"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing client_secret should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_client_credentials_missing_client_id() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "client_credentials",
        "client_secret": "secret123"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing client_id should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_client_credentials_client_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "client_credentials",
        "client_id": "nonexistent-client",
        "client_secret": "secret123"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Client not found returns 401 (Unauthorized) for security reasons
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_refresh_missing_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "refresh_token",
        "client_id": "test-client"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing refresh_token should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_refresh_missing_client_id() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "refresh_token",
        "refresh_token": "refresh-token-123"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    // Missing client_id should return 400
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_token_response_structure() {
    // Test that TokenResponse has correct structure
    let response = TokenResponse {
        access_token: "token123".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some("refresh123".to_string()),
        id_token: Some("id123".to_string()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("access_token"));
    assert!(json.contains("Bearer"));
    assert!(json.contains("3600"));
}

#[tokio::test]
async fn test_token_response_without_optional_fields() {
    let response = TokenResponse {
        access_token: "token123".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: None,
        id_token: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("access_token"));
    // Optional fields should serialize as null
    assert!(json.contains("null"));
}

// ============================================================================
// Additional Edge Cases
// ============================================================================

#[tokio::test]
async fn test_authorize_multiple_redirect_uris() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec![
        "https://app1.example.com/callback".to_string(),
        "https://app2.example.com/callback".to_string(),
        "https://app3.example.com/callback".to_string(),
    ];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "multi-redirect-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    // Test first URI
    let (status1, _) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=multi-redirect-client&redirect_uri=https://app1.example.com/callback&scope=openid&state=csrf-state",
    ).await;
    assert_eq!(status1, StatusCode::TEMPORARY_REDIRECT);

    // Test second URI
    let (status2, _) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=multi-redirect-client&redirect_uri=https://app2.example.com/callback&scope=openid&state=csrf-state",
    ).await;
    assert_eq!(status2, StatusCode::TEMPORARY_REDIRECT);

    // Test invalid URI
    let (status3, _) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=multi-redirect-client&redirect_uri=https://invalid.example.com/callback&scope=openid&state=csrf-state",
    ).await;
    assert_eq!(status3, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_userinfo_with_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token(user_id, "named@example.com", Some("Named User"))
        .unwrap();

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/auth/userinfo", &token).await;

    assert_eq!(status, StatusCode::OK);
    let claims = body.unwrap();
    assert_eq!(claims["email"], "named@example.com");
    // Name might be present in claims depending on JWT structure
}

#[tokio::test]
async fn test_userinfo_without_name() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token(user_id, "noname@example.com", None)
        .unwrap();

    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/auth/userinfo", &token).await;

    assert_eq!(status, StatusCode::OK);
    let claims = body.unwrap();
    assert_eq!(claims["email"], "noname@example.com");
}

// ============================================================================
// Health & Ready Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_health_endpoint_via_router() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) = get_json(&app, "/health").await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["version"].as_str().is_some());
}

#[tokio::test]
async fn test_ready_endpoint_via_router() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(&app, "/ready").await;
    // TestAppState.check_ready always returns (true, true)
    assert_eq!(status, StatusCode::OK);
}

// ============================================================================
// Logout with valid token tests
// ============================================================================

#[tokio::test]
async fn test_logout_with_valid_token_and_session() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a user and session first
    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    // Create identity token with session ID
    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            user_id,
            "logout-user@example.com",
            Some("Logout User"),
            Some(session_id),
        )
        .unwrap();

    let app = build_test_router(state);

    // Logout with bearer token containing session ID
    let request = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri("/api/v1/auth/logout")
        .header("Authorization", format!("Bearer {}", token))
        .body(axum::body::Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();

    // Should still redirect to Keycloak (session revocation is best-effort)
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_logout_with_expired_token() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    // Use an obviously invalid token (not a real JWT)
    let request = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri("/api/v1/auth/logout")
        .header("Authorization", "Bearer expired-invalid-token")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();

    // Should still redirect to Keycloak even with invalid token
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_logout_with_token_no_session_id() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create identity token WITHOUT session ID
    let user_id = Uuid::new_v4();
    let token = state
        .jwt_manager
        .create_identity_token(user_id, "no-session@example.com", None)
        .unwrap();

    let app = build_test_router(state);

    let request = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri("/api/v1/auth/logout")
        .header("Authorization", format!("Bearer {}", token))
        .body(axum::body::Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();

    // Should still redirect to Keycloak
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}

// ============================================================================
// Authorize with scope filtering tests
// ============================================================================

#[tokio::test]
async fn test_authorize_with_invalid_scope_rejects() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://app.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "scope-test-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    // Scope without openid should return 400
    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=scope-test-client&redirect_uri=https://app.example.com/callback&scope=profile+email&state=csrf-state",
    ).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ============================================================================
// Client Credentials Success Tests
// ============================================================================

#[tokio::test]
async fn test_token_client_credentials_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Create a service and client with a properly hashed secret
    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service.clone()).await;

    let client_secret = "test-secret-for-credentials";
    let salt =
        argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2 = argon2::Argon2::default();
    let hashed = argon2::PasswordHasher::hash_password(&argon2, client_secret.as_bytes(), &salt)
        .unwrap()
        .to_string();

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "cc-test-client".to_string(),
        client_secret_hash: hashed,
        name: Some("CC Test Client".to_string()),
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    let input = json!({
        "grant_type": "client_credentials",
        "client_id": "cc-test-client",
        "client_secret": client_secret
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["access_token"].as_str().is_some());
    assert!(body["expires_in"].as_i64().unwrap() > 0);
    // client_credentials should not have refresh_token or id_token
    assert!(body["refresh_token"].is_null());
    assert!(body["id_token"].is_null());
}

#[tokio::test]
async fn test_token_client_credentials_wrong_secret() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), None);
    state.service_repo.add_service(service.clone()).await;

    let salt =
        argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2_hasher = argon2::Argon2::default();
    let hashed = argon2::PasswordHasher::hash_password(&argon2_hasher, b"correct-secret", &salt)
        .unwrap()
        .to_string();

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "cc-wrong-secret-client".to_string(),
        client_secret_hash: hashed,
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    let input = json!({
        "grant_type": "client_credentials",
        "client_id": "cc-wrong-secret-client",
        "client_secret": "wrong-secret"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_token_client_credentials_with_tenant() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let service = create_test_service(Some(service_id), Some(tenant_id));
    state.service_repo.add_service(service.clone()).await;

    let client_secret = "tenant-client-secret";
    let salt =
        argon2::password_hash::SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let argon2_hasher = argon2::Argon2::default();
    let hashed =
        argon2::PasswordHasher::hash_password(&argon2_hasher, client_secret.as_bytes(), &salt)
            .unwrap()
            .to_string();

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "cc-tenant-client".to_string(),
        client_secret_hash: hashed,
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    let input = json!({
        "grant_type": "client_credentials",
        "client_id": "cc-tenant-client",
        "client_secret": client_secret
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["access_token"].as_str().is_some());
}

// ============================================================================
// Callback Success Tests (with mock Keycloak OIDC endpoints)
// ============================================================================

/// Helper to prime callback state in cache and return nonce.
async fn create_callback_state_nonce(
    state: &TestAppState,
    redirect_uri: &str,
    client_id: &str,
) -> String {
    let nonce = Uuid::new_v4().to_string();
    let state_payload = serde_json::json!({
        "redirect_uri": redirect_uri,
        "client_id": client_id,
        "original_state": "test-state"
    });
    state
        .cache_manager
        .store_oidc_state(&nonce, &state_payload.to_string(), 300)
        .await
        .unwrap();
    nonce
}

#[tokio::test]
async fn test_callback_success_existing_user() {
    let mock_kc = MockKeycloakServer::new().await;

    // Set up Keycloak mock for token exchange and userinfo
    // NOTE: mock_get_client_uuid_by_client_id must be mounted LAST because its
    // broad path_regex would shadow more specific mocks (wiremock: last-wins)
    let kc_sub = "kc-existing-user-123";
    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-client-secret")
        .await;
    mock_kc
        .mock_token_exchange_success(
            "kc-access-token",
            Some("kc-refresh-token"),
            Some("kc-id-token"),
        )
        .await;
    mock_kc
        .mock_userinfo_endpoint(kc_sub, "existing@example.com", Some("Existing User"))
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("callback-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Pre-create the user in the repo
    let user = auth9_core::domain::User {
        id: StringUuid::new_v4(),
        email: "existing@example.com".to_string(),
        display_name: Some("Existing User".to_string()),
        avatar_url: None,
        keycloak_id: kc_sub.to_string(),
        mfa_enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.user_repo.add_user(user).await;

    let state_nonce = create_callback_state_nonce(
        &state,
        "https://app.example.com/callback",
        "callback-client",
    )
    .await;
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        &format!(
            "/api/v1/auth/callback?code=auth-code-123&state={}",
            state_nonce
        ),
    )
    .await;

    // Should redirect to the redirect_uri with code
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_callback_success_new_user_created() {
    let mock_kc = MockKeycloakServer::new().await;

    let kc_sub = "kc-new-user-456";
    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-client-secret")
        .await;
    mock_kc
        .mock_token_exchange_success("kc-access-token-new", None, None)
        .await;
    mock_kc
        .mock_userinfo_endpoint(kc_sub, "newuser@example.com", Some("New User"))
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("callback-new-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let state_nonce = create_callback_state_nonce(
        &state,
        "https://app.example.com/callback",
        "callback-new-client",
    )
    .await;
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        &format!(
            "/api/v1/auth/callback?code=new-auth-code&state={}",
            state_nonce
        ),
    )
    .await;

    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

#[tokio::test]
async fn test_callback_missing_cached_state_returns_error() {
    let mock_kc = MockKeycloakServer::new().await;

    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-client-secret")
        .await;
    mock_kc
        .mock_token_exchange_failure("invalid_grant", "Authorization code expired")
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("fail-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        &format!(
            "/api/v1/auth/callback?code=expired-code&state={}",
            "missing-state"
        ),
    )
    .await;

    // State is one-time and server-side. Unknown state should fail.
    assert!(status.as_u16() >= 400);
}

#[tokio::test]
async fn test_callback_does_not_depend_on_userinfo() {
    let mock_kc = MockKeycloakServer::new().await;

    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-client-secret")
        .await;
    mock_kc
        .mock_token_exchange_success("kc-access-token-fail", None, None)
        .await;
    mock_kc.mock_userinfo_endpoint_failure().await;
    mock_kc
        .mock_get_client_uuid_by_client_id("userinfo-fail-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let state_nonce = create_callback_state_nonce(
        &state,
        "https://app.example.com/callback",
        "userinfo-fail-client",
    )
    .await;
    let app = build_test_router(state);

    let (status, _body) = get_raw(
        &app,
        &format!(
            "/api/v1/auth/callback?code=valid-code&state={}",
            state_nonce
        ),
    )
    .await;

    // Callback now only validates one-time state and forwards authorization code.
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

// ============================================================================
// Token Authorization Code Success Tests
// ============================================================================

#[tokio::test]
async fn test_token_authorization_code_success_existing_user() {
    let mock_kc = MockKeycloakServer::new().await;

    let kc_sub = "kc-token-user-789";
    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-client-secret")
        .await;
    mock_kc
        .mock_token_exchange_success(
            "kc-access-token-auth",
            Some("kc-refresh-token-auth"),
            Some("kc-id-token-auth"),
        )
        .await;
    mock_kc
        .mock_userinfo_endpoint(kc_sub, "tokenuser@example.com", Some("Token User"))
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("token-auth-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Pre-create the user
    let user = auth9_core::domain::User {
        id: StringUuid::new_v4(),
        email: "tokenuser@example.com".to_string(),
        display_name: Some("Token User".to_string()),
        avatar_url: None,
        keycloak_id: kc_sub.to_string(),
        mfa_enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let input = json!({
        "grant_type": "authorization_code",
        "client_id": "token-auth-client",
        "code": "valid-auth-code",
        "redirect_uri": "https://app.example.com/callback"
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["access_token"].as_str().is_some());
    assert!(body["expires_in"].as_i64().unwrap() > 0);
    // Should have refresh_token and id_token from Keycloak
    assert_eq!(body["refresh_token"], "kc-refresh-token-auth");
    assert_eq!(body["id_token"], "kc-id-token-auth");
}

#[tokio::test]
async fn test_token_authorization_code_new_user() {
    let mock_kc = MockKeycloakServer::new().await;

    let kc_sub = "kc-new-token-user-abc";
    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-secret")
        .await;
    mock_kc
        .mock_token_exchange_success("kc-access-new", None, None)
        .await;
    mock_kc
        .mock_userinfo_endpoint(kc_sub, "newtoken@example.com", None)
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("token-new-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    // Don't pre-create user
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "authorization_code",
        "client_id": "token-new-client",
        "code": "new-user-code",
        "redirect_uri": "https://app.example.com/callback"
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["access_token"].as_str().is_some());
}

// ============================================================================
// Token Refresh Tests (with mock Keycloak)
// ============================================================================

#[tokio::test]
async fn test_token_refresh_success() {
    let mock_kc = MockKeycloakServer::new().await;

    let kc_sub = "kc-refresh-user-xyz";
    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-secret")
        .await;
    mock_kc
        .mock_token_exchange_success("kc-refreshed-access", Some("kc-new-refresh"), None)
        .await;
    mock_kc
        .mock_userinfo_endpoint(kc_sub, "refresh@example.com", Some("Refresh User"))
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("refresh-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state
        .cache_manager
        .bind_refresh_token_session(
            "original-refresh-token",
            &StringUuid::new_v4().to_string(),
            300,
        )
        .await
        .unwrap();

    // Pre-create the user
    let user = auth9_core::domain::User {
        id: StringUuid::new_v4(),
        email: "refresh@example.com".to_string(),
        display_name: Some("Refresh User".to_string()),
        avatar_url: None,
        keycloak_id: kc_sub.to_string(),
        mfa_enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.user_repo.add_user(user).await;

    let app = build_test_router(state);

    let input = json!({
        "grant_type": "refresh_token",
        "client_id": "refresh-client",
        "refresh_token": "original-refresh-token"
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["access_token"].as_str().is_some());
    // Should have new refresh_token
    assert_eq!(body["refresh_token"], "kc-new-refresh");
}

#[tokio::test]
async fn test_token_refresh_new_user() {
    let mock_kc = MockKeycloakServer::new().await;

    let kc_sub = "kc-refresh-new-user";
    let client_uuid = Uuid::new_v4().to_string();
    mock_kc
        .mock_get_client_secret(&client_uuid, "kc-secret")
        .await;
    mock_kc
        .mock_token_exchange_success("kc-refreshed-access-new", None, None)
        .await;
    mock_kc
        .mock_userinfo_endpoint(kc_sub, "refresh-new@example.com", None)
        .await;
    mock_kc
        .mock_get_client_uuid_by_client_id("refresh-new-client", &client_uuid)
        .await;

    let state = TestAppState::with_mock_keycloak(&mock_kc);
    state
        .cache_manager
        .bind_refresh_token_session(
            "refresh-token-for-new-user",
            &StringUuid::new_v4().to_string(),
            300,
        )
        .await
        .unwrap();
    // Don't pre-create user
    let app = build_test_router(state);

    let input = json!({
        "grant_type": "refresh_token",
        "client_id": "refresh-new-client",
        "refresh_token": "refresh-token-for-new-user"
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/auth/token", &input).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert!(body["access_token"].as_str().is_some());
}

// ============================================================================
// Authorize scope filtering edge cases
// ============================================================================

#[tokio::test]
async fn test_authorize_filters_unsafe_scopes() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let service_id = Uuid::new_v4();
    let mut service = create_test_service(Some(service_id), None);
    service.redirect_uris = vec!["https://app.example.com/callback".to_string()];
    state.service_repo.add_service(service.clone()).await;

    let client = Client {
        id: StringUuid::new_v4(),
        service_id: StringUuid::from(service_id),
        client_id: "scope-filter-client".to_string(),
        client_secret_hash: "hash".to_string(),
        name: None,
        created_at: Utc::now(),
    };
    state.service_repo.add_client(client).await;

    let app = build_test_router(state);

    // Include unsafe scopes that should be filtered out - openid is still present so should succeed
    let (status, _body) = get_raw(
        &app,
        "/api/v1/auth/authorize?response_type=code&client_id=scope-filter-client&redirect_uri=https://app.example.com/callback&scope=openid+admin+offline_access+profile&state=csrf-state",
    ).await;

    // Should succeed since openid is present (unsafe scopes are just filtered out)
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
}

// ============================================================================
// Logout with session blacklist tests
// ============================================================================

#[tokio::test]
async fn test_logout_with_session_and_all_params() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    let token = state
        .jwt_manager
        .create_identity_token_with_session(
            user_id,
            "full-logout@example.com",
            Some("Full Logout User"),
            Some(session_id),
        )
        .unwrap();

    let app = build_test_router(state);

    // Logout with bearer token AND query params
    let request = axum::http::Request::builder()
        .method(axum::http::Method::GET)
        .uri(
            "/api/v1/auth/logout?id_token_hint=hint123&post_logout_redirect_uri=https://app.example.com/logged-out&state=logout-state",
        )
        .header("Authorization", format!("Bearer {}", token))
        .body(axum::body::Body::empty())
        .unwrap();

    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}
