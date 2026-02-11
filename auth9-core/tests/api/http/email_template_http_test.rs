//! HTTP handler tests for email template API
//!
//! Tests the handlers in `src/api/email_template.rs` using the production router
//! with `TestAppState` to achieve high coverage without external dependencies.

use super::{
    build_email_template_test_router, delete_json_with_auth, get_json_with_auth,
    post_json_with_auth, put_json_with_auth, TestAppState,
};
use crate::api::create_test_identity_token;
use auth9_core::api::SuccessResponse;
use auth9_core::domain::{
    EmailTemplateContent, EmailTemplateType, EmailTemplateWithContent, RenderedEmailPreview,
    SystemSettingRow,
};
use axum::http::StatusCode;
use chrono::Utc;
use serde::Deserialize;

/// Response structure for list templates
#[derive(Debug, Deserialize)]
struct ListTemplatesResponse {
    data: Vec<EmailTemplateWithContent>,
}

// ============================================================================
// List Templates Tests
// ============================================================================

#[tokio::test]
async fn test_list_templates_returns_all_default_templates() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (_, Option<ListTemplatesResponse>) =
        get_json_with_auth(&app, "/api/v1/system/email-templates", &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    // Should return all template types
    assert_eq!(response.data.len(), EmailTemplateType::all().len());

    // All should be default (not customized)
    for template in &response.data {
        assert!(!template.is_customized);
        assert!(template.updated_at.is_none());
    }
}

#[tokio::test]
async fn test_list_templates_shows_customized() {
    let state = TestAppState::new("http://mock-keycloak:8080");

    // Add a custom template to the repository
    let custom_content = EmailTemplateContent {
        subject: "Custom Subject".to_string(),
        html_body: "<h1>Custom</h1>".to_string(),
        text_body: "Custom".to_string(),
    };
    state
        .system_settings_repo
        .add_setting(SystemSettingRow {
            id: 1,
            category: "email_templates".to_string(),
            setting_key: "invitation".to_string(),
            value: serde_json::to_value(&custom_content).unwrap(),
            encrypted: false,
            description: Some("Custom invitation template".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await;

    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (_, Option<ListTemplatesResponse>) =
        get_json_with_auth(&app, "/api/v1/system/email-templates", &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();

    // Find the invitation template
    let invitation = response
        .data
        .iter()
        .find(|t| t.metadata.template_type == EmailTemplateType::Invitation)
        .unwrap();
    assert!(invitation.is_customized);
    assert_eq!(invitation.content.subject, "Custom Subject");
}

// ============================================================================
// Get Template Tests
// ============================================================================

#[tokio::test]
async fn test_get_template_default() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (_, Option<SuccessResponse<EmailTemplateWithContent>>) =
        get_json_with_auth(&app, "/api/v1/system/email-templates/invitation", &token).await;

    assert_eq!(status, StatusCode::OK);
    let response = body.unwrap();
    let template = response.data;

    assert_eq!(
        template.metadata.template_type,
        EmailTemplateType::Invitation
    );
    assert!(!template.is_customized);
    // Default template has content from email/templates
    assert!(!template.content.subject.is_empty());
    assert!(!template.content.html_body.is_empty());
}

#[tokio::test]
async fn test_get_template_customized() {
    let state = TestAppState::new("http://mock-keycloak:8080");

    // Add custom template
    let custom_content = EmailTemplateContent {
        subject: "Welcome to {{app_name}}".to_string(),
        html_body: "<h1>Welcome!</h1>".to_string(),
        text_body: "Welcome!".to_string(),
    };
    state
        .system_settings_repo
        .add_setting(SystemSettingRow {
            id: 1,
            category: "email_templates".to_string(),
            setting_key: "welcome".to_string(),
            value: serde_json::to_value(&custom_content).unwrap(),
            encrypted: false,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await;

    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let (status, body): (_, Option<SuccessResponse<EmailTemplateWithContent>>) =
        get_json_with_auth(&app, "/api/v1/system/email-templates/welcome", &token).await;

    assert_eq!(status, StatusCode::OK);
    let template = body.unwrap().data;

    assert!(template.is_customized);
    assert_eq!(template.content.subject, "Welcome to {{app_name}}");
}

#[tokio::test]
async fn test_get_template_not_found() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let (status, _): (_, Option<serde_json::Value>) =
        get_json_with_auth(&app, "/api/v1/system/email-templates/nonexistent", &token).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_all_template_types() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state.clone());
    let token = create_test_identity_token();

    // Test each template type endpoint
    let template_types = [
        "invitation",
        "password_reset",
        "email_mfa",
        "welcome",
        "email_verification",
        "password_changed",
        "security_alert",
    ];

    for template_type in template_types {
        let path = format!("/api/v1/system/email-templates/{}", template_type);
        let (status, _): (_, Option<SuccessResponse<EmailTemplateWithContent>>) =
            get_json_with_auth(&app, &path, &token).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "Failed for template type: {}",
            template_type
        );
    }
}

// ============================================================================
// Update Template Tests
// ============================================================================

#[tokio::test]
async fn test_update_template_success() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let update_request = serde_json::json!({
        "subject": "Updated Subject {{name}}",
        "html_body": "<html><body>Updated HTML</body></html>",
        "text_body": "Updated text"
    });

    let (status, body): (_, Option<SuccessResponse<EmailTemplateWithContent>>) =
        put_json_with_auth(
            &app,
            "/api/v1/system/email-templates/invitation",
            &update_request,
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    let template = body.unwrap().data;

    assert!(template.is_customized);
    assert_eq!(template.content.subject, "Updated Subject {{name}}");
    assert_eq!(
        template.content.html_body,
        "<html><body>Updated HTML</body></html>"
    );
    assert!(template.updated_at.is_some());
}

#[tokio::test]
async fn test_update_template_invalid_type() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let update_request = serde_json::json!({
        "subject": "Test",
        "html_body": "<p>Test</p>",
        "text_body": "Test"
    });

    let (status, _): (_, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        "/api/v1/system/email-templates/invalid_type",
        &update_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_template_empty_subject() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let update_request = serde_json::json!({
        "subject": "",
        "html_body": "<p>Content</p>",
        "text_body": "Content"
    });

    let (status, _): (_, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        "/api/v1/system/email-templates/invitation",
        &update_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_update_template_empty_html_body() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let update_request = serde_json::json!({
        "subject": "Subject",
        "html_body": "",
        "text_body": "Text"
    });

    let (status, _): (_, Option<serde_json::Value>) = put_json_with_auth(
        &app,
        "/api/v1/system/email-templates/invitation",
        &update_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ============================================================================
// Reset Template Tests
// ============================================================================

#[tokio::test]
async fn test_reset_template_success() {
    let state = TestAppState::new("http://mock-keycloak:8080");

    // First, add a custom template
    let custom_content = EmailTemplateContent {
        subject: "Custom".to_string(),
        html_body: "<h1>Custom</h1>".to_string(),
        text_body: "Custom".to_string(),
    };
    state
        .system_settings_repo
        .add_setting(SystemSettingRow {
            id: 1,
            category: "email_templates".to_string(),
            setting_key: "password_reset".to_string(),
            value: serde_json::to_value(&custom_content).unwrap(),
            encrypted: false,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await;

    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    // Reset the template
    let (status, body): (_, Option<SuccessResponse<EmailTemplateWithContent>>) =
        delete_json_with_auth(
            &app,
            "/api/v1/system/email-templates/password_reset",
            &token,
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    let template = body.unwrap().data;

    // Should be default after reset
    assert!(!template.is_customized);
    assert!(template.updated_at.is_none());
    // Content should be the default template content
    assert_ne!(template.content.subject, "Custom");
}

#[tokio::test]
async fn test_reset_template_not_customized() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    // Reset a template that isn't customized
    let (status, body): (_, Option<SuccessResponse<EmailTemplateWithContent>>) =
        delete_json_with_auth(&app, "/api/v1/system/email-templates/email_mfa", &token).await;

    // Should still succeed and return default template
    assert_eq!(status, StatusCode::OK);
    let template = body.unwrap().data;
    assert!(!template.is_customized);
}

#[tokio::test]
async fn test_reset_template_invalid_type() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let (status, _): (_, Option<serde_json::Value>) = delete_json_with_auth(
        &app,
        "/api/v1/system/email-templates/not_a_template",
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ============================================================================
// Preview Template Tests
// ============================================================================

#[tokio::test]
async fn test_preview_template_success() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    // Use invitation template variables: inviter_name, tenant_name, invite_link, app_name
    let preview_request = serde_json::json!({
        "subject": "You're invited to {{tenant_name}}!",
        "html_body": "<h1>Hello! {{inviter_name}} has invited you to join {{app_name}}!</h1>",
        "text_body": "Hello! {{inviter_name}} has invited you to join {{app_name}}!"
    });

    let (status, body): (_, Option<SuccessResponse<RenderedEmailPreview>>) = post_json_with_auth(
        &app,
        "/api/v1/system/email-templates/invitation/preview",
        &preview_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let preview = body.unwrap().data;

    // Preview should have sample data substituted
    assert!(!preview.subject.contains("{{"));
    assert!(!preview.html_body.contains("{{"));
    assert!(!preview.text_body.contains("{{"));

    // Verify actual substitutions (from domain/email_template.rs example values)
    assert!(preview.subject.contains("Acme Corp")); // tenant_name example
    assert!(preview.html_body.contains("John Doe")); // inviter_name example
    assert!(preview.html_body.contains("Auth9")); // app_name example
}

#[tokio::test]
async fn test_preview_template_password_reset() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let preview_request = serde_json::json!({
        "subject": "Reset your password",
        "html_body": "<p>Click <a href='{{reset_link}}'>here</a> to reset.</p>",
        "text_body": "Reset: {{reset_link}}"
    });

    let (status, body): (_, Option<SuccessResponse<RenderedEmailPreview>>) = post_json_with_auth(
        &app,
        "/api/v1/system/email-templates/password_reset/preview",
        &preview_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let preview = body.unwrap().data;

    // reset_link should be substituted
    assert!(!preview.html_body.contains("{{reset_link}}"));
}

#[tokio::test]
async fn test_preview_template_email_mfa() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let preview_request = serde_json::json!({
        "subject": "Your verification code: {{verification_code}}",
        "html_body": "<p>Code: <strong>{{verification_code}}</strong></p>",
        "text_body": "Code: {{verification_code}}"
    });

    let (status, body): (_, Option<SuccessResponse<RenderedEmailPreview>>) = post_json_with_auth(
        &app,
        "/api/v1/system/email-templates/email_mfa/preview",
        &preview_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let preview = body.unwrap().data;

    // verification_code should be substituted
    assert!(!preview.subject.contains("{{verification_code}}"));
}

#[tokio::test]
async fn test_preview_template_invalid_type() {
    let state = TestAppState::new("http://mock-keycloak:8080");
    let app = build_email_template_test_router(state);
    let token = create_test_identity_token();

    let preview_request = serde_json::json!({
        "subject": "Test",
        "html_body": "<p>Test</p>",
        "text_body": "Test"
    });

    let (status, _): (_, Option<serde_json::Value>) = post_json_with_auth(
        &app,
        "/api/v1/system/email-templates/invalid/preview",
        &preview_request,
        &token,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}
