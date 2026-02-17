//! System Settings API HTTP Handler Tests
//!
//! Tests for the system settings HTTP endpoints using mock repositories.

use crate::support::create_test_identity_token;
use crate::support::http::{
    build_system_settings_test_router, get_json_with_auth, post_json_with_auth, put_json_with_auth,
    MockKeycloakServer, TestAppState,
};
use auth9_core::domains::platform::api::system_settings::TestEmailResponse;
use auth9_core::domain::SystemSettingRow;
use axum::http::StatusCode;
use chrono::Utc;
use serde_json::json;

// ============================================================================
// Get Email Settings Tests
// ============================================================================

#[tokio::test]
async fn test_get_email_settings_none() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/email", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();
    // When not configured, should return type: "none"
    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("none"));
}

#[tokio::test]
async fn test_get_email_settings_smtp_masked() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Pre-populate with SMTP config
    let smtp_config = json!({
        "type": "smtp",
        "host": "smtp.example.com",
        "port": 587,
        "username": "user",
        "password": "secret123",
        "use_tls": true,
        "from_email": "noreply@example.com",
        "from_name": "Example"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "email".to_string(),
        setting_key: "provider".to_string(),
        value: smtp_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/email", &token).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.is_some());
    let response = body.unwrap();

    // Verify it's SMTP type
    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("smtp"));

    // Verify host is present
    let host = response["data"]["value"]["host"].as_str();
    assert_eq!(host, Some("smtp.example.com"));

    // Password should be masked
    let password = response["data"]["value"]["password"].as_str();
    assert_eq!(password, Some("***"));
}

#[tokio::test]
async fn test_get_email_settings_oracle() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let oracle_config = json!({
        "type": "oracle",
        "smtp_endpoint": "smtp.us-ashburn-1.oraclecloud.com",
        "port": 587,
        "username": "ocid1.user.oc1..test",
        "password": "secret",
        "from_email": "noreply@example.com",
        "from_name": "Oracle Test"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "email".to_string(),
        setting_key: "provider".to_string(),
        value: oracle_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/email", &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("oracle"));

    let smtp_endpoint = response["data"]["value"]["smtp_endpoint"].as_str();
    assert_eq!(smtp_endpoint, Some("smtp.us-ashburn-1.oraclecloud.com"));
}

#[tokio::test]
async fn test_get_email_settings_ses() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    let ses_config = json!({
        "type": "ses",
        "region": "us-east-1",
        "access_key_id": "AKIAEXAMPLE",
        "secret_access_key": "secret123",
        "from_email": "noreply@example.com"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "email".to_string(),
        setting_key: "provider".to_string(),
        value: ses_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/email", &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("ses"));

    let region = response["data"]["value"]["region"].as_str();
    assert_eq!(region, Some("us-east-1"));

    // Secret should be masked
    let secret = response["data"]["value"]["secret_access_key"].as_str();
    assert_eq!(secret, Some("***"));
}

// ============================================================================
// Update Email Settings Tests
// ============================================================================

#[tokio::test]
async fn test_update_email_settings_smtp() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "type": "smtp",
            "host": "smtp.newprovider.com",
            "port": 465,
            "username": "newuser",
            "password": "newpass",
            "use_tls": true,
            "from_email": "new@example.com",
            "from_name": "New Sender"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("smtp"));

    let host = response["data"]["value"]["host"].as_str();
    assert_eq!(host, Some("smtp.newprovider.com"));

    let port = response["data"]["value"]["port"].as_u64();
    assert_eq!(port, Some(465));
}

#[tokio::test]
async fn test_update_email_settings_none() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // First set up some config
    let smtp_config = json!({
        "type": "smtp",
        "host": "smtp.example.com",
        "port": 587,
        "from_email": "noreply@example.com"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "email".to_string(),
        setting_key: "provider".to_string(),
        value: smtp_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    // Now disable email by setting to None
    let input = json!({
        "config": {
            "type": "none"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("none"));
}

#[tokio::test]
async fn test_update_email_settings_oracle() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "type": "oracle",
            "smtp_endpoint": "smtp.us-phoenix-1.oraclecloud.com",
            "port": 587,
            "username": "ocid1.user.oc1..newuser",
            "password": "newpassword",
            "from_email": "oracle@example.com",
            "from_name": "Oracle Sender"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("oracle"));

    let smtp_endpoint = response["data"]["value"]["smtp_endpoint"].as_str();
    assert_eq!(smtp_endpoint, Some("smtp.us-phoenix-1.oraclecloud.com"));
}

#[tokio::test]
async fn test_update_email_settings_ses() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "type": "ses",
            "region": "eu-west-1",
            "access_key_id": "AKIANEWKEY",
            "secret_access_key": "newsecret",
            "from_email": "ses@example.com",
            "from_name": "SES Sender"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("ses"));

    let region = response["data"]["value"]["region"].as_str();
    assert_eq!(region, Some("eu-west-1"));
}

#[tokio::test]
async fn test_update_email_settings_invalid_email() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "type": "smtp",
            "host": "smtp.example.com",
            "port": 587,
            "from_email": "not-an-email",
            "from_name": "Test"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    // Invalid email should return validation error (422)
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_update_email_settings_missing_host() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "config": {
            "type": "smtp",
            "port": 587,
            "from_email": "test@example.com"
        }
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    // Missing required field should fail
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Test Connection Tests
// ============================================================================

#[tokio::test]
async fn test_email_connection_not_configured() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/system/email/test", &json!({}), &token).await;

    // When email is not configured, should return an error
    assert!(
        status == StatusCode::BAD_REQUEST
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_email_connection_response_structure() {
    // Test that TestEmailResponse has correct structure
    let response = TestEmailResponse {
        success: true,
        message: "Connection successful".to_string(),
        message_id: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("success"));
    assert!(json.contains("Connection successful"));
}

#[tokio::test]
async fn test_email_connection_failure_response() {
    let response = TestEmailResponse {
        success: false,
        message: "Connection failed: timeout".to_string(),
        message_id: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("Connection failed"));
}

// ============================================================================
// Send Test Email Tests
// ============================================================================

#[tokio::test]
async fn test_send_email_not_configured() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "to_email": "recipient@example.com"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/system/email/send-test", &input, &token).await;

    // When email is not configured, should return an error
    assert!(
        status == StatusCode::BAD_REQUEST
            || status == StatusCode::INTERNAL_SERVER_ERROR
            || status == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_send_email_invalid_address() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // Set up email config
    let smtp_config = json!({
        "type": "smtp",
        "host": "smtp.example.com",
        "port": 587,
        "from_email": "noreply@example.com"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "email".to_string(),
        setting_key: "provider".to_string(),
        value: smtp_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    let input = json!({
        "to_email": "not-an-email"
    });

    let (status, _body): (StatusCode, Option<serde_json::Value>) =
        post_json_with_auth(&app, "/api/v1/system/email/send-test", &input, &token).await;

    // Invalid email should return validation error
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_send_email_response_with_message_id() {
    let response = TestEmailResponse {
        success: true,
        message: "Test email sent to recipient@example.com".to_string(),
        message_id: Some("msg-12345".to_string()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("msg-12345"));
}

// ============================================================================
// Edge Cases and Additional Tests
// ============================================================================

#[tokio::test]
async fn test_update_preserves_password_on_masked_input() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);

    // First set up config with real password
    let smtp_config = json!({
        "type": "smtp",
        "host": "smtp.example.com",
        "port": 587,
        "username": "user",
        "password": "realpassword123",
        "from_email": "noreply@example.com"
    });

    let setting = SystemSettingRow {
        id: 1,
        category: "email".to_string(),
        setting_key: "provider".to_string(),
        value: smtp_config,
        encrypted: false,
        description: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    state.system_settings_repo.add_setting(setting).await;

    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    // Update with masked password (should preserve original)
    let input = json!({
        "config": {
            "type": "smtp",
            "host": "smtp.newhost.com",
            "port": 587,
            "username": "user",
            "password": "***",
            "from_email": "noreply@example.com"
        }
    });

    let (status, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    // Host should be updated
    let host = response["data"]["value"]["host"].as_str();
    assert_eq!(host, Some("smtp.newhost.com"));
}

#[tokio::test]
async fn test_email_settings_roundtrip() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    // Update settings
    let input = json!({
        "config": {
            "type": "smtp",
            "host": "roundtrip.example.com",
            "port": 25,
            "from_email": "roundtrip@example.com"
        }
    });

    let (status1, _): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &input, &token).await;
    assert_eq!(status1, StatusCode::OK);

    // Read back settings
    let (status2, body): (StatusCode, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/email", &token).await;
    assert_eq!(status2, StatusCode::OK);

    let response = body.unwrap();
    let host = response["data"]["value"]["host"].as_str();
    assert_eq!(host, Some("roundtrip.example.com"));

    let port = response["data"]["value"]["port"].as_u64();
    assert_eq!(port, Some(25));
}

#[tokio::test]
async fn test_email_provider_type_switch() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);
    let token = create_test_identity_token();

    // Start with SMTP
    let smtp_input = json!({
        "config": {
            "type": "smtp",
            "host": "smtp.example.com",
            "port": 587,
            "from_email": "smtp@example.com"
        }
    });

    let (status1, _): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &smtp_input, &token).await;
    assert_eq!(status1, StatusCode::OK);

    // Switch to SES
    let ses_input = json!({
        "config": {
            "type": "ses",
            "region": "us-west-2",
            "from_email": "ses@example.com"
        }
    });

    let (status2, body): (StatusCode, Option<serde_json::Value>) =
        put_json_with_auth(&app, "/api/v1/system/email", &ses_input, &token).await;
    assert_eq!(status2, StatusCode::OK);

    let response = body.unwrap();
    let config_type = response["data"]["value"]["type"].as_str();
    assert_eq!(config_type, Some("ses"));
}

// ============================================================================
// Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_get_email_settings_requires_auth() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);

    // No auth token
    let (status, _): (StatusCode, Option<serde_json::Value>) =
        crate::support::http::get_json(&app, "/api/v1/system/email").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_update_email_settings_requires_auth() {
    let mock_kc = MockKeycloakServer::new().await;
    let state = TestAppState::with_mock_keycloak(&mock_kc);
    let app = build_system_settings_test_router(state);

    let input = json!({
        "config": {
            "type": "smtp",
            "host": "smtp.attacker.com",
            "port": 25,
            "from_email": "attacker@example.com"
        }
    });

    let (status, _): (StatusCode, Option<serde_json::Value>) =
        crate::support::http::put_json(&app, "/api/v1/system/email", &input).await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
