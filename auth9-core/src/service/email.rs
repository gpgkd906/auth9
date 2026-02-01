//! Email service for sending emails through configured providers

use crate::domain::{
    EmailAddress, EmailMessage, EmailProviderConfig, EmailSendResult, TenantEmailSettings,
};
use crate::email::{
    EmailProvider, EmailProviderError, SesEmailProvider, SmtpEmailProvider, TemplateEngine,
};
use crate::error::{AppError, Result};
use crate::repository::SystemSettingsRepository;
use crate::service::SystemSettingsService;
use std::sync::Arc;

/// Service for sending emails
///
/// Handles provider selection with tenant override support.
pub struct EmailService<R: SystemSettingsRepository> {
    settings_service: Arc<SystemSettingsService<R>>,
}

impl<R: SystemSettingsRepository> EmailService<R> {
    pub fn new(settings_service: Arc<SystemSettingsService<R>>) -> Self {
        Self { settings_service }
    }

    /// Send an email using the configured provider
    ///
    /// Uses tenant settings if provided, otherwise falls back to system settings.
    pub async fn send(
        &self,
        message: &EmailMessage,
        tenant_settings: Option<&TenantEmailSettings>,
    ) -> Result<EmailSendResult> {
        // Get the effective email configuration
        let config = self.get_effective_config(tenant_settings).await?;

        if !config.is_configured() {
            return Err(AppError::BadRequest(
                "Email provider not configured".to_string(),
            ));
        }

        // Create the appropriate provider
        let provider = self.create_provider(&config).await?;

        // Send the email
        provider
            .send(message)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Email send failed: {}", e)))
    }

    /// Send an email using a specific from address override
    pub async fn send_with_from(
        &self,
        to: EmailAddress,
        subject: &str,
        html_body: &str,
        text_body: Option<&str>,
        tenant_settings: Option<&TenantEmailSettings>,
    ) -> Result<EmailSendResult> {
        let mut message = EmailMessage::new(to, subject, html_body);
        if let Some(text) = text_body {
            message = message.with_text_body(text);
        }

        self.send(&message, tenant_settings).await
    }

    /// Test the email configuration by connecting to the provider
    pub async fn test_connection(
        &self,
        tenant_settings: Option<&TenantEmailSettings>,
    ) -> Result<()> {
        let config = self.get_effective_config(tenant_settings).await?;

        if !config.is_configured() {
            return Err(AppError::BadRequest(
                "Email provider not configured".to_string(),
            ));
        }

        let provider = self.create_provider(&config).await?;

        provider.test_connection().await.map_err(|e| match e {
            EmailProviderError::AuthenticationFailed(msg) => {
                AppError::Unauthorized(format!("Email authentication failed: {}", msg))
            }
            EmailProviderError::ConnectionError(msg) => {
                AppError::BadRequest(format!("Connection failed: {}", msg))
            }
            EmailProviderError::InvalidConfiguration(msg) => {
                AppError::Validation(format!("Invalid configuration: {}", msg))
            }
            e => AppError::Internal(anyhow::anyhow!("{}", e)),
        })
    }

    /// Send a password reset email
    pub async fn send_password_reset(
        &self,
        to_email: &str,
        reset_token: &str,
        user_name: Option<&str>,
    ) -> Result<EmailSendResult> {
        let display_name = user_name.unwrap_or("User");
        let reset_url = format!(
            "{}/reset-password?token={}",
            std::env::var("AUTH9_PORTAL_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            reset_token
        );

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Password Reset</title></head>
<body style="font-family: sans-serif; padding: 20px; max-width: 600px; margin: 0 auto;">
    <h1 style="color: #2563eb;">Password Reset Request</h1>
    <p>Hello {},</p>
    <p>We received a request to reset your password. Click the button below to set a new password:</p>
    <p style="text-align: center; margin: 30px 0;">
        <a href="{}" style="display: inline-block; padding: 12px 24px; background: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: bold;">
            Reset Password
        </a>
    </p>
    <p>If you didn't request this, you can safely ignore this email. The link will expire in 1 hour.</p>
    <p style="color: #666; font-size: 12px;">
        If the button doesn't work, copy and paste this link into your browser:<br>
        <a href="{}" style="color: #2563eb;">{}</a>
    </p>
    <hr style="margin: 20px 0; border: none; border-top: 1px solid #eee;">
    <p style="color: #666; font-size: 12px;">
        &copy; {} Auth9
    </p>
</body>
</html>"#,
            display_name,
            reset_url,
            reset_url,
            reset_url,
            chrono::Utc::now().format("%Y")
        );

        let text_body = format!(
            "Password Reset Request\n\nHello {},\n\nWe received a request to reset your password. Visit the link below to set a new password:\n\n{}\n\nIf you didn't request this, you can safely ignore this email. The link will expire in 1 hour.",
            display_name,
            reset_url
        );

        self.send_with_from(
            EmailAddress::new(to_email),
            "Password Reset Request",
            &html_body,
            Some(&text_body),
            None,
        )
        .await
    }

    /// Send a password changed notification
    pub async fn send_password_changed(
        &self,
        to_email: &str,
        user_name: Option<&str>,
    ) -> Result<EmailSendResult> {
        let display_name = user_name.unwrap_or("User");
        let now = chrono::Utc::now();

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Password Changed</title></head>
<body style="font-family: sans-serif; padding: 20px; max-width: 600px; margin: 0 auto;">
    <h1 style="color: #2563eb;">Password Changed Successfully</h1>
    <p>Hello {},</p>
    <p>Your password was changed on {}.</p>
    <p>If you made this change, you can safely ignore this email.</p>
    <p style="color: #dc2626; font-weight: bold;">
        If you did not make this change, please contact support immediately and secure your account.
    </p>
    <hr style="margin: 20px 0; border: none; border-top: 1px solid #eee;">
    <p style="color: #666; font-size: 12px;">
        &copy; {} Auth9
    </p>
</body>
</html>"#,
            display_name,
            now.format("%Y-%m-%d %H:%M:%S UTC"),
            now.format("%Y")
        );

        let text_body = format!(
            "Password Changed Successfully\n\nHello {},\n\nYour password was changed on {}.\n\nIf you made this change, you can safely ignore this email.\n\nIf you did not make this change, please contact support immediately and secure your account.",
            display_name,
            now.format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.send_with_from(
            EmailAddress::new(to_email),
            "Password Changed Successfully",
            &html_body,
            Some(&text_body),
            None,
        )
        .await
    }

    /// Send a test email to verify configuration works end-to-end
    pub async fn send_test_email(
        &self,
        to_email: &str,
        tenant_settings: Option<&TenantEmailSettings>,
    ) -> Result<EmailSendResult> {
        let mut engine = TemplateEngine::new();
        engine
            .set("app_name", "Auth9")
            .set("year", chrono::Utc::now().format("%Y").to_string());

        let html_body = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Test Email</title></head>
<body style="font-family: sans-serif; padding: 20px;">
    <h1 style="color: #2563eb;">Auth9 Test Email</h1>
    <p>This is a test email from your Auth9 installation.</p>
    <p>If you received this email, your email configuration is working correctly.</p>
    <hr style="margin: 20px 0; border: none; border-top: 1px solid #eee;">
    <p style="color: #666; font-size: 12px;">
        Sent at: {}<br>
        &copy; {} Auth9
    </p>
</body>
</html>"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            chrono::Utc::now().format("%Y")
        );

        let text_body = format!(
            "Auth9 Test Email\n\nThis is a test email from your Auth9 installation.\nIf you received this email, your email configuration is working correctly.\n\nSent at: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.send_with_from(
            EmailAddress::new(to_email),
            "Auth9 Test Email",
            &html_body,
            Some(&text_body),
            tenant_settings,
        )
        .await
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    async fn get_effective_config(
        &self,
        tenant_settings: Option<&TenantEmailSettings>,
    ) -> Result<EmailProviderConfig> {
        // Check tenant override first
        if let Some(settings) = tenant_settings {
            if let Some(provider) = &settings.provider {
                return Ok(provider.clone());
            }
        }

        // Fall back to system settings
        self.settings_service.get_email_config().await
    }

    async fn create_provider(
        &self,
        config: &EmailProviderConfig,
    ) -> Result<Box<dyn EmailProvider>> {
        match config {
            EmailProviderConfig::None => Err(AppError::BadRequest(
                "Email provider not configured".to_string(),
            )),
            EmailProviderConfig::Smtp(smtp_config) => {
                let provider = SmtpEmailProvider::from_config(smtp_config).map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Failed to create SMTP provider: {}", e))
                })?;
                Ok(Box::new(provider))
            }
            EmailProviderConfig::Ses(ses_config) => {
                let provider = SesEmailProvider::from_config(ses_config)
                    .await
                    .map_err(|e| {
                        AppError::Internal(anyhow::anyhow!("Failed to create SES provider: {}", e))
                    })?;
                Ok(Box::new(provider))
            }
            EmailProviderConfig::Oracle(oracle_config) => {
                let provider =
                    SmtpEmailProvider::from_oracle_config(oracle_config).map_err(|e| {
                        AppError::Internal(anyhow::anyhow!(
                            "Failed to create Oracle Email provider: {}",
                            e
                        ))
                    })?;
                Ok(Box::new(provider))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SmtpConfig;
    use crate::domain::SystemSettingRow;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_send_not_configured() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| Ok(None));

        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let message = EmailMessage::new(
            EmailAddress::new("test@example.com"),
            "Test",
            "<p>Hello</p>",
        );

        let result = email_service.send(&message, None).await;
        assert!(result.is_err());

        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("not configured"));
        } else {
            panic!("Expected BadRequest error");
        }
    }

    #[tokio::test]
    async fn test_tenant_override_takes_precedence() {
        // Create a service with system SMTP config
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({
                        "type": "smtp",
                        "host": "system.smtp.com",
                        "port": 587,
                        "from_email": "system@example.com"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        // Tenant override with different config
        let tenant_settings = TenantEmailSettings {
            provider: Some(EmailProviderConfig::Smtp(SmtpConfig {
                host: "tenant.smtp.com".to_string(),
                port: 465,
                username: None,
                password: None,
                use_tls: true,
                from_email: "tenant@example.com".to_string(),
                from_name: None,
            })),
            from_email: None,
            from_name: None,
        };

        let config = email_service
            .get_effective_config(Some(&tenant_settings))
            .await
            .unwrap();

        if let EmailProviderConfig::Smtp(smtp) = config {
            assert_eq!(smtp.host, "tenant.smtp.com");
        } else {
            panic!("Expected SMTP config");
        }
    }

    #[tokio::test]
    async fn test_create_smtp_provider() {
        let mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "localhost".to_string(),
            port: 1025,
            username: None,
            password: None,
            use_tls: false,
            from_email: "test@example.com".to_string(),
            from_name: None,
        });

        let provider = email_service.create_provider(&config).await;
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().provider_name(), "smtp");
    }

    #[tokio::test]
    async fn test_create_none_provider_fails() {
        let mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let config = EmailProviderConfig::None;
        let result = email_service.create_provider(&config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_oracle_provider() {
        use crate::domain::OracleEmailConfig;

        let mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let config = EmailProviderConfig::Oracle(OracleEmailConfig {
            smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com".to_string(),
            port: 587,
            username: "ocid1.user.oc1..test".to_string(),
            password: "password".to_string(),
            from_email: "test@example.com".to_string(),
            from_name: None,
        });

        let provider = email_service.create_provider(&config).await;
        assert!(provider.is_ok());
        // Oracle uses SMTP protocol but identifies as "oracle" provider
        assert_eq!(provider.unwrap().provider_name(), "oracle");
    }

    #[tokio::test]
    async fn test_create_ses_provider() {
        use crate::domain::SesConfig;

        let mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        // Use explicit credentials for testing
        let config = EmailProviderConfig::Ses(SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            from_email: "test@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        });

        // Provider creation should succeed (it doesn't validate credentials at creation time)
        let result = email_service.create_provider(&config).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider_name(), "ses");
    }

    #[tokio::test]
    async fn test_connection_not_configured() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| Ok(None));

        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let result = email_service.test_connection(None).await;
        assert!(result.is_err());
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("not configured"));
        }
    }

    #[tokio::test]
    async fn test_get_effective_config_with_none_tenant_settings() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({
                        "type": "smtp",
                        "host": "system.smtp.com",
                        "port": 587,
                        "from_email": "system@example.com"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        // Tenant settings without provider override
        let tenant_settings = TenantEmailSettings {
            provider: None,
            from_email: Some("tenant@example.com".to_string()),
            from_name: None,
        };

        let config = email_service
            .get_effective_config(Some(&tenant_settings))
            .await
            .unwrap();

        // Should fall back to system config
        if let EmailProviderConfig::Smtp(smtp) = config {
            assert_eq!(smtp.host, "system.smtp.com");
        } else {
            panic!("Expected SMTP config");
        }
    }

    #[tokio::test]
    async fn test_send_test_email_not_configured() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| Ok(None));

        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let result = email_service
            .send_test_email("test@example.com", None)
            .await;
        assert!(result.is_err());
    }
}
