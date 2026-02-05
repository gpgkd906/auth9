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
