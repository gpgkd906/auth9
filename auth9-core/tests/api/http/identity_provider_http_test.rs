//! Identity Provider HTTP API handler tests
//!
//! Tests for identity provider management endpoints.

use super::{delete_json, get_json, post_json, put_json, MockKeycloakServer, TestAppState};
use crate::api::create_test_user;
use auth9_core::api::{MessageResponse, SuccessResponse};
use auth9_core::domain::{
    IdentityProvider, IdentityProviderTemplate, LinkedIdentity, LinkedIdentityInfo, StringUuid,
};
use auth9_core::repository::LinkedIdentityRepository;
use axum::http::StatusCode;
use chrono::Utc;

// ============================================================================
// Get Templates Tests
// ============================================================================

#[tokio::test]
async fn test_get_templates() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, body): (
        StatusCode,
        Option<SuccessResponse<Vec<IdentityProviderTemplate>>>,
    ) = get_json(&app, "/api/v1/identity-providers/templates").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let templates = body.unwrap().data;

    // Should have at least the common templates
    assert!(templates.len() >= 5);

    // Verify Google template exists
    let google = templates.iter().find(|t| t.provider_id == "google");
    assert!(google.is_some());
    let google = google.unwrap();
    assert_eq!(google.name, "Google");

    // Verify GitHub template exists
    let github = templates.iter().find(|t| t.provider_id == "github");
    assert!(github.is_some());

    // Verify Microsoft template exists
    let microsoft = templates.iter().find(|t| t.provider_id == "microsoft");
    assert!(microsoft.is_some());

    // Verify OIDC template exists
    let oidc = templates.iter().find(|t| t.provider_id == "oidc");
    assert!(oidc.is_some());

    // Verify SAML template exists
    let saml = templates.iter().find(|t| t.provider_id == "saml");
    assert!(saml.is_some());
}

// ============================================================================
// Linked Identities Tests
// ============================================================================

#[tokio::test]
async fn test_list_linked_identities_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Create a valid JWT token for the test
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_idp_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<LinkedIdentityInfo>>>) =
        super::get_json_with_auth(&app, "/api/v1/me/linked-identities", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let identities = body.unwrap().data;
    assert!(identities.is_empty());
}

#[tokio::test]
async fn test_list_linked_identities_with_data() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add linked identities
    let google_identity = LinkedIdentity {
        id: StringUuid::new_v4(),
        user_id,
        provider_type: "google".to_string(),
        provider_alias: "google".to_string(),
        external_user_id: "google-123".to_string(),
        external_email: Some("user@gmail.com".to_string()),
        linked_at: Utc::now(),
    };
    state
        .linked_identity_repo
        .add_identity(google_identity)
        .await;

    let github_identity = LinkedIdentity {
        id: StringUuid::new_v4(),
        user_id,
        provider_type: "github".to_string(),
        provider_alias: "github".to_string(),
        external_user_id: "github-456".to_string(),
        external_email: Some("user@github.com".to_string()),
        linked_at: Utc::now(),
    };
    state
        .linked_identity_repo
        .add_identity(github_identity)
        .await;

    // Create a valid JWT token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_idp_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<LinkedIdentityInfo>>>) =
        super::get_json_with_auth(&app, "/api/v1/me/linked-identities", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let identities = body.unwrap().data;
    assert_eq!(identities.len(), 2);

    // Verify Google identity
    let google = identities.iter().find(|i| i.provider_type == "google");
    assert!(google.is_some());
    let google = google.unwrap();
    assert_eq!(google.provider_display_name, Some("Google".to_string()));
    assert_eq!(google.external_email, Some("user@gmail.com".to_string()));

    // Verify GitHub identity
    let github = identities.iter().find(|i| i.provider_type == "github");
    assert!(github.is_some());
    let github = github.unwrap();
    assert_eq!(github.provider_display_name, Some("GitHub".to_string()));
}

#[tokio::test]
async fn test_list_linked_identities_unauthorized() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    // No auth header
    let (status, _): (StatusCode, Option<SuccessResponse<Vec<LinkedIdentityInfo>>>) =
        get_json(&app, "/api/v1/me/linked-identities").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Unlink Identity Tests
// ============================================================================

#[tokio::test]
async fn test_unlink_identity_success() {
    let mock_kc = MockKeycloakServer::new().await;
    // Mock the federated identity removal endpoint in Keycloak
    mock_kc.mock_remove_federated_identity_success().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let user = create_test_user(None);
    let user_id = user.id;
    state.user_repo.add_user(user).await;

    // Add a linked identity
    let identity = LinkedIdentity {
        id: StringUuid::new_v4(),
        user_id,
        provider_type: "google".to_string(),
        provider_alias: "google".to_string(),
        external_user_id: "google-123".to_string(),
        external_email: Some("user@gmail.com".to_string()),
        linked_at: Utc::now(),
    };
    let identity_id = identity.id;
    state.linked_identity_repo.add_identity(identity).await;

    // Create a valid JWT token
    let token = state
        .jwt_manager
        .create_identity_token(*user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let app = build_idp_test_router(state.clone());

    let (status, body): (StatusCode, Option<auth9_core::api::MessageResponse>) =
        super::delete_json_with_auth(
            &app,
            &format!("/api/v1/me/linked-identities/{}", identity_id),
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("unlinked"));

    // Verify identity is gone
    let identities = state
        .linked_identity_repo
        .list_by_user(user_id)
        .await
        .unwrap();
    assert!(identities.is_empty());
}

// ============================================================================
// List Providers Tests
// ============================================================================

#[tokio::test]
async fn test_list_providers_success() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc
        .mock_list_identity_providers(vec![("google", "google"), ("github", "github")])
        .await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<IdentityProvider>>>) =
        get_json(&app, "/api/v1/identity-providers").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let providers = body.unwrap().data;
    assert_eq!(providers.len(), 2);
}

#[tokio::test]
async fn test_list_providers_empty() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_list_identity_providers_empty().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<Vec<IdentityProvider>>>) =
        get_json(&app, "/api/v1/identity-providers").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let providers = body.unwrap().data;
    assert!(providers.is_empty());
}

// ============================================================================
// Get Provider Tests
// ============================================================================

#[tokio::test]
async fn test_get_provider_success() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_get_identity_provider("google", "google").await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, body): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        get_json(&app, "/api/v1/identity-providers/google").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let provider = body.unwrap().data;
    assert_eq!(provider.alias, "google");
}

#[tokio::test]
async fn test_get_provider_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_get_identity_provider_not_found().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, _): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        get_json(&app, "/api/v1/identity-providers/nonexistent").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Create Provider Tests
// ============================================================================

#[tokio::test]
async fn test_create_provider_success() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_identity_provider_success().await;
    mock_kc.mock_get_identity_provider("google", "google").await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let input = serde_json::json!({
        "alias": "google",
        "display_name": "Google",
        "provider_id": "google",
        "enabled": true,
        "trust_email": false,
        "store_token": false,
        "link_only": false,
        "config": {
            "clientId": "test-client-id",
            "clientSecret": "test-secret"
        }
    });

    let (status, body): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        post_json(&app, "/api/v1/identity-providers", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let provider = body.unwrap().data;
    assert_eq!(provider.alias, "google");
}

#[tokio::test]
async fn test_create_provider_conflict() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_create_identity_provider_conflict().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let input = serde_json::json!({
        "alias": "google",
        "display_name": "Google",
        "provider_id": "google",
        "enabled": true,
        "config": {
            "clientId": "test-client-id",
            "clientSecret": "test-client-secret"
        }
    });

    let (status, _): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        post_json(&app, "/api/v1/identity-providers", &input).await;

    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_provider_validation_error() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    // Missing required field 'alias'
    let input = serde_json::json!({
        "display_name": "Google",
        "provider_id": "google"
    });

    let (status, _): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        post_json(&app, "/api/v1/identity-providers", &input).await;

    // Bad request due to missing required field
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_create_provider_missing_config_fields() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    // Google provider without required clientId
    let input = serde_json::json!({
        "alias": "google-missing",
        "provider_id": "google",
        "enabled": true,
        "config": {
            "clientSecret": "test-secret"
        }
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        post_json(&app, "/api/v1/identity-providers", &input).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Update Provider Tests
// ============================================================================

#[tokio::test]
async fn test_update_provider_success() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_get_identity_provider("google", "google").await;
    mock_kc.mock_update_identity_provider_success().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let input = serde_json::json!({
        "display_name": "Updated Google",
        "enabled": false
    });

    let (status, body): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        put_json(&app, "/api/v1/identity-providers/google", &input).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
}

#[tokio::test]
async fn test_update_provider_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_get_identity_provider_not_found().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let input = serde_json::json!({
        "enabled": false
    });

    let (status, _): (StatusCode, Option<SuccessResponse<IdentityProvider>>) =
        put_json(&app, "/api/v1/identity-providers/nonexistent", &input).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Delete Provider Tests
// ============================================================================

#[tokio::test]
async fn test_delete_provider_success() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_delete_identity_provider_success().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, body): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, "/api/v1/identity-providers/google").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    assert!(body.unwrap().message.contains("deleted"));
}

#[tokio::test]
async fn test_delete_provider_not_found() {
    let mock_kc = MockKeycloakServer::new().await;
    mock_kc.mock_delete_identity_provider_not_found().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let app = build_idp_test_router(state);

    let (status, _): (StatusCode, Option<MessageResponse>) =
        delete_json(&app, "/api/v1/identity-providers/nonexistent").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Test Router Builder
// ============================================================================

fn build_idp_test_router(state: TestAppState) -> axum::Router {
    use auth9_core::api::identity_provider;
    use axum::routing::{delete, get};

    axum::Router::new()
        .route(
            "/api/v1/identity-providers",
            get(identity_provider::list_providers::<TestAppState>)
                .post(identity_provider::create_provider::<TestAppState>),
        )
        .route(
            "/api/v1/identity-providers/templates",
            get(identity_provider::get_templates::<TestAppState>),
        )
        .route(
            "/api/v1/identity-providers/{alias}",
            get(identity_provider::get_provider::<TestAppState>)
                .put(identity_provider::update_provider::<TestAppState>)
                .delete(identity_provider::delete_provider::<TestAppState>),
        )
        .route(
            "/api/v1/me/linked-identities",
            get(identity_provider::list_my_linked_identities::<TestAppState>),
        )
        .route(
            "/api/v1/me/linked-identities/{identity_id}",
            delete(identity_provider::unlink_identity::<TestAppState>),
        )
        .with_state(state)
}
