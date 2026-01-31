//! Email provider trait and error types

use crate::domain::{EmailMessage, EmailSendResult};
use async_trait::async_trait;
use thiserror::Error;

/// Email provider error types
#[derive(Error, Debug)]
pub enum EmailProviderError {
    #[error("Email provider not configured")]
    NotConfigured,

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Rate limited")]
    RateLimited,
}

/// Trait for email providers
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email message
    async fn send(&self, message: &EmailMessage) -> Result<EmailSendResult, EmailProviderError>;

    /// Test connection to the email provider
    async fn test_connection(&self) -> Result<(), EmailProviderError>;

    /// Get the provider name
    fn provider_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::EmailAddress;

    #[tokio::test]
    async fn test_mock_email_provider() {
        let mut mock = MockEmailProvider::new();

        mock.expect_provider_name()
            .returning(|| "mock");

        mock.expect_test_connection()
            .returning(|| Ok(()));

        mock.expect_send()
            .returning(|_| Ok(EmailSendResult::success(Some("msg-123".to_string()))));

        assert_eq!(mock.provider_name(), "mock");
        assert!(mock.test_connection().await.is_ok());

        let message = EmailMessage::new(
            EmailAddress::new("test@example.com"),
            "Test",
            "<p>Hello</p>",
        );
        let result = mock.send(&message).await.unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_email_provider_error_display() {
        let errors = vec![
            EmailProviderError::NotConfigured,
            EmailProviderError::ConnectionError("timeout".to_string()),
            EmailProviderError::AuthenticationFailed("bad password".to_string()),
            EmailProviderError::SendFailed("recipient rejected".to_string()),
            EmailProviderError::InvalidConfiguration("missing host".to_string()),
            EmailProviderError::RateLimited,
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }
}
