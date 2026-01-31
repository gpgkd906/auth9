//! SMTP email provider implementation using lettre

use super::provider::{EmailProvider, EmailProviderError};
use crate::domain::{EmailMessage, EmailSendResult, OracleEmailConfig, SmtpConfig};
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

/// SMTP-based email provider (works for standard SMTP and Oracle Email Delivery)
pub struct SmtpEmailProvider {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
    from_name: Option<String>,
    provider_name: &'static str,
}

impl SmtpEmailProvider {
    /// Create a new SMTP provider from configuration
    pub fn from_config(config: &SmtpConfig) -> Result<Self, EmailProviderError> {
        let mut builder = if config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)
                .map_err(|e| EmailProviderError::InvalidConfiguration(e.to_string()))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
        };

        builder = builder.port(config.port);

        // Add credentials if provided
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            let credentials = Credentials::new(username.clone(), password.clone());
            builder = builder.credentials(credentials);
        }

        let transport = builder.build();

        Ok(Self {
            transport,
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
            provider_name: "smtp",
        })
    }

    /// Create a provider for Oracle Email Delivery
    pub fn from_oracle_config(config: &OracleEmailConfig) -> Result<Self, EmailProviderError> {
        let builder = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_endpoint)
            .map_err(|e| EmailProviderError::InvalidConfiguration(e.to_string()))?
            .port(config.port)
            .credentials(Credentials::new(
                config.username.clone(),
                config.password.clone(),
            ));

        let transport = builder.build();

        Ok(Self {
            transport,
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
            provider_name: "oracle",
        })
    }

    fn build_from_mailbox(&self) -> Result<Mailbox, EmailProviderError> {
        let mailbox = if let Some(name) = &self.from_name {
            format!("{} <{}>", name, self.from_email)
        } else {
            self.from_email.clone()
        };

        mailbox.parse().map_err(|e| {
            EmailProviderError::InvalidConfiguration(format!("Invalid from address: {}", e))
        })
    }
}

#[async_trait]
impl EmailProvider for SmtpEmailProvider {
    async fn send(&self, message: &EmailMessage) -> Result<EmailSendResult, EmailProviderError> {
        let from = self.build_from_mailbox()?;

        // Build recipient list
        let mut to_list = Vec::new();
        for addr in &message.to {
            let mailbox: Mailbox = if let Some(name) = &addr.name {
                format!("{} <{}>", name, addr.email)
            } else {
                addr.email.clone()
            }
            .parse()
            .map_err(|e| {
                EmailProviderError::InvalidConfiguration(format!("Invalid to address: {}", e))
            })?;
            to_list.push(mailbox);
        }

        if to_list.is_empty() {
            return Err(EmailProviderError::InvalidConfiguration(
                "No recipients specified".to_string(),
            ));
        }

        // Build the message
        let mut email_builder = Message::builder().from(from).subject(&message.subject);

        for to in to_list {
            email_builder = email_builder.to(to);
        }

        // Build body (multipart if text body is provided)
        let email = if let Some(text_body) = &message.text_body {
            email_builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(text_body.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(message.html_body.clone()),
                        ),
                )
                .map_err(|e| EmailProviderError::SendFailed(e.to_string()))?
        } else {
            email_builder
                .header(ContentType::TEXT_HTML)
                .body(message.html_body.clone())
                .map_err(|e| EmailProviderError::SendFailed(e.to_string()))?
        };

        // Send the email
        match self.transport.send(email).await {
            Ok(response) => {
                // Get the first message from the response
                let message_id = response.message().next().map(|s| s.to_string());
                Ok(EmailSendResult::success(message_id))
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("authentication") || error_msg.contains("AUTH") {
                    Err(EmailProviderError::AuthenticationFailed(error_msg))
                } else if error_msg.contains("connection") || error_msg.contains("timeout") {
                    Err(EmailProviderError::ConnectionError(error_msg))
                } else {
                    Err(EmailProviderError::SendFailed(error_msg))
                }
            }
        }
    }

    async fn test_connection(&self) -> Result<(), EmailProviderError> {
        self.transport
            .test_connection()
            .await
            .map(|_| ()) // Convert bool to ()
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("authentication") || error_msg.contains("AUTH") {
                    EmailProviderError::AuthenticationFailed(error_msg)
                } else {
                    EmailProviderError::ConnectionError(error_msg)
                }
            })
    }

    fn provider_name(&self) -> &'static str {
        self.provider_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_smtp_config() -> SmtpConfig {
        SmtpConfig {
            host: "localhost".to_string(),
            port: 1025,
            username: None,
            password: None,
            use_tls: false,
            from_email: "test@example.com".to_string(),
            from_name: Some("Test Sender".to_string()),
        }
    }

    #[test]
    fn test_smtp_provider_creation() {
        let config = test_smtp_config();
        let provider = SmtpEmailProvider::from_config(&config);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.provider_name(), "smtp");
    }

    #[test]
    fn test_smtp_provider_with_auth() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: Some("user@example.com".to_string()),
            password: Some("password".to_string()),
            use_tls: true,
            from_email: "noreply@example.com".to_string(),
            from_name: None,
        };

        let provider = SmtpEmailProvider::from_config(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_oracle_provider_creation() {
        let config = OracleEmailConfig {
            smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com".to_string(),
            port: 587,
            username: "ocid1.user@ocid1.tenancy".to_string(),
            password: "password".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Oracle Test".to_string()),
        };

        let provider = SmtpEmailProvider::from_oracle_config(&config);
        assert!(provider.is_ok());

        let provider = provider.unwrap();
        assert_eq!(provider.provider_name(), "oracle");
    }

    #[test]
    fn test_build_from_mailbox() {
        let config = test_smtp_config();
        let provider = SmtpEmailProvider::from_config(&config).unwrap();

        let mailbox = provider.build_from_mailbox().unwrap();
        assert_eq!(mailbox.email.to_string(), "test@example.com");
    }

    #[test]
    fn test_build_from_mailbox_without_name() {
        let config = SmtpConfig {
            from_name: None,
            ..test_smtp_config()
        };
        let provider = SmtpEmailProvider::from_config(&config).unwrap();

        let mailbox = provider.build_from_mailbox().unwrap();
        assert_eq!(mailbox.email.to_string(), "test@example.com");
    }
}
