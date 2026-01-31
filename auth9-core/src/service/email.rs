//! Email service for sending emails through configured providers

use crate::domain::{
    EmailAddress, EmailMessage, EmailProviderConfig, EmailSendResult, TenantEmailSettings,
};
use crate::email::{EmailProvider, EmailProviderError, SmtpEmailProvider, TemplateEngine};
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
        let provider = self.create_provider(&config)?;

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

        let provider = self.create_provider(&config)?;

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

    fn create_provider(
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
            EmailProviderConfig::Ses(_ses_config) => {
                // SES support would be added here
                // For now, return an error indicating it's not yet implemented
                Err(AppError::BadRequest(
                    "AWS SES provider is not yet implemented. Use SMTP instead.".to_string(),
                ))
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
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use crate::domain::SystemSettingRow;
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

        let provider = email_service.create_provider(&config);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().provider_name(), "smtp");
    }

    #[tokio::test]
    async fn test_create_none_provider_fails() {
        let mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let config = EmailProviderConfig::None;
        let result = email_service.create_provider(&config);

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

        let provider = email_service.create_provider(&config);
        assert!(provider.is_ok());
        // Oracle uses SMTP protocol but identifies as "oracle" provider
        assert_eq!(provider.unwrap().provider_name(), "oracle");
    }

    #[tokio::test]
    async fn test_create_ses_provider_not_implemented() {
        use crate::domain::SesConfig;

        let mock = MockSystemSettingsRepository::new();
        let settings_service = Arc::new(SystemSettingsService::new(Arc::new(mock), None));
        let email_service = EmailService::new(settings_service);

        let config = EmailProviderConfig::Ses(SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            from_email: "test@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        });

        let result = email_service.create_provider(&config);
        assert!(result.is_err());
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("not yet implemented"));
        }
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

        let result = email_service.send_test_email("test@example.com", None).await;
        assert!(result.is_err());
    }
}
