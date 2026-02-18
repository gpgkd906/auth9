//! System settings API handlers

use crate::api::{require_platform_admin_with_db, write_audit_log_generic, SuccessResponse};
use crate::domain::EmailProviderConfig;
use crate::error::{AppError, Result};
use crate::middleware::auth::AuthUser;
use crate::state::{HasServices, HasSystemSettings};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for updating email settings
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateEmailSettingsRequest {
    pub config: EmailProviderConfig,
}

/// Response for test email request
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TestEmailResponse {
    pub success: bool,
    pub message: String,
    pub message_id: Option<String>,
}

/// Request body for sending test email
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SendTestEmailRequest {
    pub to_email: String,
}

/// Get email provider settings (with sensitive data masked)
#[utoipa::path(
    get,
    path = "/api/v1/system/email",
    tag = "Platform",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_email_settings<S: HasSystemSettings + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let settings = state
        .system_settings_service()
        .get_email_config_masked()
        .await?;
    Ok(Json(SuccessResponse::new(settings)))
}

/// Update email provider settings
#[utoipa::path(
    put,
    path = "/api/v1/system/email",
    tag = "Platform",
    request_body = UpdateEmailSettingsRequest,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_email_settings<S: HasSystemSettings + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(request): Json<UpdateEmailSettingsRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;

    // Validate the configuration
    state
        .system_settings_service()
        .validate_email_config(&request.config)?;

    // Update the settings
    state
        .system_settings_service()
        .update_email_config(request.config)
        .await?;

    // Return the masked settings
    let settings = state
        .system_settings_service()
        .get_email_config_masked()
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "system.email.update",
        "system_setting",
        None,
        None,
        serde_json::to_value(&settings).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(settings)))
}

/// Test email connection (verify credentials)
#[utoipa::path(
    post,
    path = "/api/v1/system/email/test",
    tag = "Platform",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn test_email_connection<S: HasSystemSettings + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;

    state.email_service().test_connection(None).await?;

    Ok(Json(TestEmailResponse {
        success: true,
        message: "Connection successful".to_string(),
        message_id: None,
    }))
}

/// Send a test email
#[utoipa::path(
    post,
    path = "/api/v1/system/email/send-test",
    tag = "Platform",
    request_body = SendTestEmailRequest,
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn send_test_email<S: HasSystemSettings + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Json(request): Json<SendTestEmailRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    // Basic email validation
    if !request.to_email.contains('@') {
        return Err(AppError::Validation("Invalid email address".to_string()));
    }

    let result = state
        .email_service()
        .send_test_email(&request.to_email, None)
        .await?;

    if result.success {
        Ok((
            StatusCode::OK,
            Json(TestEmailResponse {
                success: true,
                message: format!("Test email sent to {}", request.to_email),
                message_id: result.message_id,
            }),
        ))
    } else {
        Ok((
            StatusCode::OK,
            Json(TestEmailResponse {
                success: false,
                message: result.error.unwrap_or_else(|| "Unknown error".to_string()),
                message_id: None,
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_email_settings_request_deserialization() {
        let json = r#"{
            "config": {
                "type": "smtp",
                "host": "smtp.example.com",
                "port": 587,
                "username": "user",
                "password": "pass",
                "use_tls": true,
                "from_email": "noreply@example.com",
                "from_name": "Example"
            }
        }"#;

        let request: UpdateEmailSettingsRequest = serde_json::from_str(json).unwrap();

        match request.config {
            EmailProviderConfig::Smtp(config) => {
                assert_eq!(config.host, "smtp.example.com");
                assert_eq!(config.port, 587);
            }
            _ => panic!("Expected SMTP config"),
        }
    }

    #[test]
    fn test_update_email_settings_request_none() {
        let json = r#"{
            "config": {
                "type": "none"
            }
        }"#;

        let request: UpdateEmailSettingsRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.config, EmailProviderConfig::None));
    }

    #[test]
    fn test_send_test_email_request() {
        let json = r#"{"to_email": "test@example.com"}"#;
        let request: SendTestEmailRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.to_email, "test@example.com");
    }

    #[test]
    fn test_test_email_response_serialization() {
        let response = TestEmailResponse {
            success: true,
            message: "Email sent".to_string(),
            message_id: Some("msg-123".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message_id\":\"msg-123\""));
    }

    #[test]
    fn test_test_email_response_without_message_id() {
        let response = TestEmailResponse {
            success: false,
            message: "Connection failed".to_string(),
            message_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"message\":\"Connection failed\""));
        assert!(json.contains("\"message_id\":null"));
    }

    #[test]
    fn test_update_email_settings_request_oracle() {
        let json = r#"{
            "config": {
                "type": "oracle",
                "smtp_endpoint": "smtp.us-ashburn-1.oraclecloud.com",
                "port": 587,
                "username": "ocid1.user.oc1..test",
                "password": "secret",
                "from_email": "noreply@example.com",
                "from_name": "Oracle Test"
            }
        }"#;

        let request: UpdateEmailSettingsRequest = serde_json::from_str(json).unwrap();

        match request.config {
            EmailProviderConfig::Oracle(config) => {
                assert_eq!(config.smtp_endpoint, "smtp.us-ashburn-1.oraclecloud.com");
                assert_eq!(config.port, 587);
                assert_eq!(config.from_email, "noreply@example.com");
            }
            _ => panic!("Expected Oracle config"),
        }
    }

    #[test]
    fn test_update_email_settings_request_ses() {
        let json = r#"{
            "config": {
                "type": "ses",
                "region": "us-east-1",
                "access_key_id": "AKIAXXXXXXXX",
                "secret_access_key": "secret",
                "from_email": "noreply@example.com"
            }
        }"#;

        let request: UpdateEmailSettingsRequest = serde_json::from_str(json).unwrap();

        match request.config {
            EmailProviderConfig::Ses(config) => {
                assert_eq!(config.region, "us-east-1");
                assert_eq!(config.from_email, "noreply@example.com");
            }
            _ => panic!("Expected SES config"),
        }
    }

    #[test]
    fn test_send_test_email_request_empty() {
        let json = r#"{"to_email": ""}"#;
        let request: SendTestEmailRequest = serde_json::from_str(json).unwrap();
        assert!(request.to_email.is_empty());
    }
}
