//! Email OTP channel implementation

use super::channel::{OtpChannel, OtpChannelType};
use crate::domains::platform::service::email::EmailService;
use crate::email::{EmailTemplate, TemplateEngine};
use crate::error::Result;
use crate::models::email::{EmailAddress, EmailMessage};
use crate::repository::SystemSettingsRepository;
use async_trait::async_trait;
use std::sync::Arc;

/// Email OTP channel wrapping the existing EmailService
pub struct EmailOtpChannel<R: SystemSettingsRepository> {
    email_service: Arc<EmailService<R>>,
}

impl<R: SystemSettingsRepository> EmailOtpChannel<R> {
    pub fn new(email_service: Arc<EmailService<R>>) -> Self {
        Self { email_service }
    }
}

#[async_trait]
impl<R: SystemSettingsRepository + 'static> OtpChannel for EmailOtpChannel<R> {
    fn channel_type(&self) -> OtpChannelType {
        OtpChannelType::Email
    }

    async fn send_code(&self, destination: &str, code: &str, ttl_minutes: u32) -> Result<()> {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", destination)
            .set("verification_code", code)
            .set("expires_in_minutes", ttl_minutes.to_string())
            .set("app_name", "Auth9")
            .set("year", chrono::Utc::now().format("%Y").to_string());

        let rendered = engine.render_template(EmailTemplate::EmailMfa);

        let message = EmailMessage::new(
            EmailAddress::new(destination),
            &rendered.subject,
            &rendered.html_body,
        )
        .with_text_body(&rendered.text_body);

        self.email_service.send(&message, None).await?;
        Ok(())
    }
}
