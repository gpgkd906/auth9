//! Email template API handlers

use crate::api::{require_platform_admin_with_db, write_audit_log_generic, SuccessResponse};
use crate::domain::{
    EmailAddress, EmailMessage, EmailTemplateContent, EmailTemplateType, EmailTemplateWithContent,
};
use crate::error::{AppError, Result};
use crate::middleware::auth::AuthUser;
use crate::state::{HasEmailTemplates, HasServices, HasSystemSettings};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Response for list templates
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListTemplatesResponse {
    pub data: Vec<EmailTemplateWithContent>,
}

/// Request body for updating a template
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateTemplateRequest {
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

impl From<UpdateTemplateRequest> for EmailTemplateContent {
    fn from(req: UpdateTemplateRequest) -> Self {
        EmailTemplateContent {
            subject: req.subject,
            html_body: req.html_body,
            text_body: req.text_body,
        }
    }
}

/// Request body for previewing a template
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PreviewTemplateRequest {
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

impl From<PreviewTemplateRequest> for EmailTemplateContent {
    fn from(req: PreviewTemplateRequest) -> Self {
        EmailTemplateContent {
            subject: req.subject,
            html_body: req.html_body,
            text_body: req.text_body,
        }
    }
}

/// Request body for sending a test email with a template
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SendTestEmailRequest {
    pub to_email: String,
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

/// Response for sending a test email
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SendTestEmailResponse {
    pub success: bool,
    pub message: String,
    pub message_id: Option<String>,
}

/// List all email templates
///
/// GET /api/v1/system/email-templates
#[utoipa::path(
    get,
    path = "/api/v1/system/email-templates",
    tag = "Platform",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list_templates<S: HasEmailTemplates + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let templates = state.email_template_service().list_templates().await?;
    Ok(Json(ListTemplatesResponse { data: templates }))
}

/// Get a specific email template
///
/// GET /api/v1/system/email-templates/:type
#[utoipa::path(
    get,
    path = "/api/v1/system/email-templates/{type}",
    tag = "Platform",
    params(
        ("type" = String, Path, description = "Template type")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_template<S: HasEmailTemplates + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(template_type): Path<String>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let template_type = parse_template_type(&template_type)?;
    let template = state
        .email_template_service()
        .get_template(template_type)
        .await?;
    Ok(Json(SuccessResponse::new(template)))
}

/// Update an email template
///
/// PUT /api/v1/system/email-templates/:type
#[utoipa::path(
    put,
    path = "/api/v1/system/email-templates/{type}",
    tag = "Platform",
    params(
        ("type" = String, Path, description = "Template type")
    ),
    request_body = UpdateTemplateRequest,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_template<S: HasEmailTemplates + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(template_type): Path<String>,
    Json(request): Json<UpdateTemplateRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let template_type = parse_template_type(&template_type)?;
    let content: EmailTemplateContent = request.into();
    let template = state
        .email_template_service()
        .update_template(template_type, content)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "system.email_templates.update",
        "email_template",
        None,
        None,
        serde_json::to_value(&template).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(template)))
}

/// Reset a template to default
///
/// DELETE /api/v1/system/email-templates/:type
#[utoipa::path(
    delete,
    path = "/api/v1/system/email-templates/{type}",
    tag = "Platform",
    params(
        ("type" = String, Path, description = "Template type")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn reset_template<S: HasEmailTemplates + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(template_type): Path<String>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let template_type = parse_template_type(&template_type)?;
    let template = state
        .email_template_service()
        .reset_template(template_type)
        .await?;
    Ok(Json(SuccessResponse::new(template)))
}

/// Preview a template with sample data
///
/// POST /api/v1/system/email-templates/:type/preview
#[utoipa::path(
    post,
    path = "/api/v1/system/email-templates/{type}/preview",
    tag = "Platform",
    params(
        ("type" = String, Path, description = "Template type")
    ),
    request_body = PreviewTemplateRequest,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn preview_template<S: HasEmailTemplates + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(template_type): Path<String>,
    Json(request): Json<PreviewTemplateRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let template_type = parse_template_type(&template_type)?;
    let content: EmailTemplateContent = request.into();
    let preview = state
        .email_template_service()
        .preview_template(template_type, &content)
        .await?;
    Ok(Json(SuccessResponse::new(preview)))
}

/// Send a test email using a template
///
/// POST /api/v1/system/email-templates/:type/send-test
#[utoipa::path(
    post,
    path = "/api/v1/system/email-templates/{type}/send-test",
    tag = "Platform",
    params(
        ("type" = String, Path, description = "Template type")
    ),
    request_body = SendTestEmailRequest,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn send_test_email<S: HasEmailTemplates + HasSystemSettings + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(template_type): Path<String>,
    Json(request): Json<SendTestEmailRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let template_type = parse_template_type(&template_type)?;

    // Build the template content from the request
    let content = EmailTemplateContent {
        subject: request.subject,
        html_body: request.html_body,
        text_body: request.text_body,
    };

    // Render the template with the provided variables
    let rendered = state
        .email_template_service()
        .render_template_with_variables(template_type, &content, &request.variables);

    // Create the email message
    let message = EmailMessage::new(
        EmailAddress::new(&request.to_email),
        &rendered.subject,
        &rendered.html_body,
    )
    .with_text_body(&rendered.text_body);

    // Send the email using the email service
    match state.email_service().send(&message, None).await {
        Ok(result) => Ok(Json(SendTestEmailResponse {
            success: true,
            message: "Test email sent successfully".to_string(),
            message_id: result.message_id,
        })),
        Err(e) => Ok(Json(SendTestEmailResponse {
            success: false,
            message: e.to_string(),
            message_id: None,
        })),
    }
}

/// Parse template type from path parameter
fn parse_template_type(s: &str) -> Result<EmailTemplateType> {
    s.parse::<EmailTemplateType>()
        .map_err(|_| AppError::NotFound(format!("Unknown template type: {}", s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_type_valid() {
        assert_eq!(
            parse_template_type("invitation").unwrap(),
            EmailTemplateType::Invitation
        );
        assert_eq!(
            parse_template_type("password_reset").unwrap(),
            EmailTemplateType::PasswordReset
        );
        assert_eq!(
            parse_template_type("email_mfa").unwrap(),
            EmailTemplateType::EmailMfa
        );
        assert_eq!(
            parse_template_type("welcome").unwrap(),
            EmailTemplateType::Welcome
        );
        assert_eq!(
            parse_template_type("email_verification").unwrap(),
            EmailTemplateType::EmailVerification
        );
        assert_eq!(
            parse_template_type("password_changed").unwrap(),
            EmailTemplateType::PasswordChanged
        );
        assert_eq!(
            parse_template_type("security_alert").unwrap(),
            EmailTemplateType::SecurityAlert
        );
    }

    #[test]
    fn test_parse_template_type_invalid() {
        let result = parse_template_type("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_update_template_request_into_content() {
        let req = UpdateTemplateRequest {
            subject: "Test Subject".to_string(),
            html_body: "<p>HTML</p>".to_string(),
            text_body: "Text".to_string(),
        };

        let content: EmailTemplateContent = req.into();
        assert_eq!(content.subject, "Test Subject");
        assert_eq!(content.html_body, "<p>HTML</p>");
        assert_eq!(content.text_body, "Text");
    }

    #[test]
    fn test_preview_template_request_into_content() {
        let req = PreviewTemplateRequest {
            subject: "Preview Subject".to_string(),
            html_body: "<h1>Preview</h1>".to_string(),
            text_body: "Preview".to_string(),
        };

        let content: EmailTemplateContent = req.into();
        assert_eq!(content.subject, "Preview Subject");
    }

    #[test]
    fn test_update_template_request_deserialization() {
        let json = r#"{
            "subject": "Hello {{name}}",
            "html_body": "<h1>Hello {{name}}</h1>",
            "text_body": "Hello {{name}}"
        }"#;

        let req: UpdateTemplateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.subject, "Hello {{name}}");
    }

    #[test]
    fn test_list_templates_response_serialization() {
        let response = ListTemplatesResponse { data: vec![] };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"data\":[]"));
    }

    #[test]
    fn test_send_test_email_request_deserialization() {
        let json = r#"{
            "to_email": "test@example.com",
            "subject": "Hello {{name}}",
            "html_body": "<h1>Hello {{name}}</h1>",
            "text_body": "Hello {{name}}",
            "variables": {"name": "World"}
        }"#;

        let req: SendTestEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.to_email, "test@example.com");
        assert_eq!(req.subject, "Hello {{name}}");
        assert_eq!(req.variables.get("name"), Some(&"World".to_string()));
    }

    #[test]
    fn test_send_test_email_request_without_variables() {
        let json = r#"{
            "to_email": "test@example.com",
            "subject": "Test",
            "html_body": "<p>Test</p>",
            "text_body": "Test"
        }"#;

        let req: SendTestEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.to_email, "test@example.com");
        assert!(req.variables.is_empty());
    }

    #[test]
    fn test_send_test_email_response_serialization() {
        let response = SendTestEmailResponse {
            success: true,
            message: "Sent successfully".to_string(),
            message_id: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message_id\":\"abc123\""));
    }

    #[test]
    fn test_send_test_email_response_without_message_id() {
        let response = SendTestEmailResponse {
            success: false,
            message: "Failed to send".to_string(),
            message_id: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"message_id\":null"));
    }
}
