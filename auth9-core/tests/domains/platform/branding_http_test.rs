//! Branding API HTTP Handler Tests
//!
//! Tests for the branding HTTP endpoints using mock repositories.

use crate::support::create_test_identity_token;
use crate::support::http::{
    build_branding_test_router, get_json, get_json_with_auth, put_json_with_auth,
    MockKeycloakServer, TestAppState,
};
use auth9_core::domain::SystemSettingRow;
use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;

// ============================================================================
// Get Public Branding Tests
// ============================================================================

#[tokio::test]
async fn test_get_public_branding_defaults() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, "/api/v1/public/branding").await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();

    // Should return default values
    let primary_color = response["data"]["primary_color"].as_str();
    assert_eq!(primary_color, Some("#007AFF"));

    let secondary_color = response["data"]["secondary_color"].as_str();
    assert_eq!(secondary_color, Some("#5856D6"));

    let background_color = response["data"]["background_color"].as_str();
    assert_eq!(background_color, Some("#F5F5F7"));

    let text_color = response["data"]["text_color"].as_str();
    assert_eq!(text_color, Some("#1D1D1F"));

    // Optional fields should be null
    assert!(response["data"]["logo_url"].is_null());
    assert!(response["data"]["company_name"].is_null());
    assert!(response["data"]["custom_css"].is_null());
    assert!(response["data"]["favicon_url"].is_null());
}

#[tokio::test]
async fn test_get_public_branding_custom() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Pre-populate with custom branding config
    let branding_config = json!({
        "logo_url": "https://example.com/logo.png",
        "primary_color": "#FF0000",
        "secondary_color": "#00FF00",
        "background_color": "#0000FF",
        "text_color": "#AABBCC",
        "company_name": "Test Corp",
        "custom_css": ".login { color: red; }"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "branding".to_string(),
        setting_key: "config".to_string(),
        value: branding_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_branding_test_router(state);

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, "/api/v1/public/branding").await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    assert_eq!(response["data"]["primary_color"].as_str(), Some("#FF0000"));
    assert_eq!(
        response["data"]["logo_url"].as_str(),
        Some("https://example.com/logo.png")
    );
    assert_eq!(response["data"]["company_name"].as_str(), Some("Test Corp"));
}

// ============================================================================
// Get Admin Branding Tests
// ============================================================================

#[tokio::test]
async fn test_get_admin_branding_defaults() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/branding", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();

    // Should return default values
    assert_eq!(response["data"]["primary_color"].as_str(), Some("#007AFF"));
}

// ============================================================================
// Update Branding Tests
// ============================================================================

#[tokio::test]
async fn test_update_branding_success() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "logo_url": "https://example.com/new-logo.png",
            "primary_color": "#123456",
            "secondary_color": "#654321",
            "background_color": "#AABBCC",
            "text_color": "#112233",
            "company_name": "New Company"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    assert_eq!(response["data"]["primary_color"].as_str(), Some("#123456"));
    assert_eq!(
        response["data"]["logo_url"].as_str(),
        Some("https://example.com/new-logo.png")
    );
    assert_eq!(
        response["data"]["company_name"].as_str(),
        Some("New Company")
    );
}

#[tokio::test]
async fn test_update_branding_minimal() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    // Only required fields (colors)
    let input = json!({
        "config": {
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    assert_eq!(response["data"]["primary_color"].as_str(), Some("#AABBCC"));
    assert!(response["data"]["logo_url"].is_null());
    assert!(response["data"]["company_name"].is_null());
}

#[tokio::test]
async fn test_update_branding_with_custom_css() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899",
            "custom_css": ".login-form { border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    assert!(response["data"]["custom_css"]
        .as_str()
        .unwrap()
        .contains("border-radius"));
}

#[tokio::test]
async fn test_update_branding_invalid_color() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "primary_color": "not-a-color",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    // Invalid color should return validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_update_branding_invalid_color_no_hash() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "primary_color": "AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    // Missing hash should return validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_update_branding_invalid_color_wrong_length() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "primary_color": "#FFF",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    // 3-digit hex should return validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_update_branding_invalid_logo_url() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "logo_url": "not-a-url",
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    // Invalid URL should return validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_update_branding_invalid_favicon_url() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "favicon_url": "not-a-url",
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    // Invalid favicon URL should return validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Roundtrip Tests
// ============================================================================

#[tokio::test]
async fn test_branding_roundtrip() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    // Update branding
    let input = json!({
        "config": {
            "logo_url": "https://example.com/logo.png",
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899",
            "company_name": "Roundtrip Corp",
            "favicon_url": "https://example.com/favicon.ico"
        }
    });

    let (status1, _): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;
    assert_eq!(status1, StatusCode::OK);

    // Read back branding
    let (status2, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/branding", &token).await;
    assert_eq!(status2, StatusCode::OK);

    let response = body.unwrap();
    assert_eq!(response["data"]["primary_color"].as_str(), Some("#AABBCC"));
    assert_eq!(
        response["data"]["company_name"].as_str(),
        Some("Roundtrip Corp")
    );
    assert_eq!(
        response["data"]["logo_url"].as_str(),
        Some("https://example.com/logo.png")
    );
}

#[tokio::test]
async fn test_branding_public_and_admin_match() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    // Update branding
    let input = json!({
        "config": {
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899",
            "company_name": "Test Corp"
        }
    });

    put_json_with_auth::<_, serde_json::Value>(&app, "/api/v1/system/branding", &input, &token)
        .await;

    // Read from public endpoint
    let (_, public_body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, "/api/v1/public/branding").await;

    // Read from admin endpoint
    let (_, admin_body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/branding", &token).await;

    // Both should return the same data
    let public_data = public_body.unwrap();
    let admin_data = admin_body.unwrap();

    assert_eq!(
        public_data["data"]["primary_color"],
        admin_data["data"]["primary_color"]
    );
    assert_eq!(
        public_data["data"]["company_name"],
        admin_data["data"]["company_name"]
    );
}

#[tokio::test]
async fn test_update_branding_lowercase_color() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    // Lowercase hex colors should work
    let input = json!({
        "config": {
            "primary_color": "#aabbcc",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    assert_eq!(response["data"]["primary_color"].as_str(), Some("#aabbcc"));
}

// ============================================================================
// Allow Registration Tests
// ============================================================================

#[tokio::test]
async fn test_branding_allow_registration_default_false() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);

    // Get defaults - allow_registration should be false
    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, "/api/v1/public/branding").await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    assert_eq!(
        response["data"]["allow_registration"].as_bool(),
        Some(false)
    );
}

#[tokio::test]
async fn test_update_branding_with_allow_registration() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    // Update with allow_registration = true
    let input = json!({
        "config": {
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899",
            "allow_registration": true
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    assert_eq!(response["data"]["allow_registration"].as_bool(), Some(true));

    // Verify it persists on read
    let (_, read_body): (StatusCode, Option<serde_json::Value>) =
        get_json(&app, "/api/v1/public/branding").await;
    let read_response = read_body.unwrap();
    assert_eq!(
        read_response["data"]["allow_registration"].as_bool(),
        Some(true)
    );
}

#[tokio::test]
async fn test_update_branding_without_allow_registration_defaults_false() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_branding_test_router(state);
    let token = create_test_identity_token();

    // Update without specifying allow_registration - should default to false
    let input = json!({
        "config": {
            "primary_color": "#AABBCC",
            "secondary_color": "#112233",
            "background_color": "#445566",
            "text_color": "#778899"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/branding", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    assert_eq!(
        response["data"]["allow_registration"].as_bool(),
        Some(false)
    );
}
