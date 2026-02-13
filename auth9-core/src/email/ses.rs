//! AWS SES email provider implementation
//!
//! Provides email sending via AWS Simple Email Service (SES) v2 API.

use super::provider::{EmailProvider, EmailProviderError};
use crate::domain::{EmailMessage, EmailSendResult, SesConfig};
use async_trait::async_trait;
use aws_sdk_sesv2::{
    config::Region,
    types::{Body, Content, Destination, EmailContent, Message},
    Client,
};

/// AWS SES email provider
///
/// Uses AWS SDK for Rust to send emails via SES v2 API.
/// Supports:
/// - IAM role credentials (when running in AWS)
/// - Explicit access key credentials
/// - Configuration sets for tracking
pub struct SesEmailProvider {
    client: Client,
    from_email: String,
    from_name: Option<String>,
    configuration_set: Option<String>,
}

impl SesEmailProvider {
    /// Create a new SES provider from configuration
    ///
    /// This is an async operation because AWS SDK needs to load credentials.
    pub async fn from_config(config: &SesConfig) -> Result<Self, EmailProviderError> {
        let region = Region::new(config.region.clone());

        let sdk_config = if let (Some(access_key), Some(secret_key)) =
            (&config.access_key_id, &config.secret_access_key)
        {
            // Use explicit credentials
            let credentials = aws_sdk_sesv2::config::Credentials::new(
                access_key.clone(),
                secret_key.clone(),
                None, // session token
                None, // expiration
                "auth9-ses",
            );

            aws_config::from_env()
                .region(region)
                .credentials_provider(credentials)
                .load()
                .await
        } else {
            // Use default credential chain (IAM role, env vars, etc.)
            aws_config::from_env().region(region).load().await
        };

        let client = Client::new(&sdk_config);

        Ok(Self {
            client,
            from_email: config.from_email.clone(),
            from_name: config.from_name.clone(),
            configuration_set: config.configuration_set.clone(),
        })
    }

    /// Build the "From" address string
    fn build_from_address(&self) -> String {
        if let Some(name) = &self.from_name {
            format!("{} <{}>", name, self.from_email)
        } else {
            self.from_email.clone()
        }
    }
}

#[async_trait]
impl EmailProvider for SesEmailProvider {
    async fn send(&self, message: &EmailMessage) -> Result<EmailSendResult, EmailProviderError> {
        // Build recipient list
        let to_addresses: Vec<String> = message
            .to
            .iter()
            .map(|addr| {
                if let Some(name) = &addr.name {
                    format!("{} <{}>", name, addr.email)
                } else {
                    addr.email.clone()
                }
            })
            .collect();

        if to_addresses.is_empty() {
            return Err(EmailProviderError::InvalidConfiguration(
                "No recipients specified".to_string(),
            ));
        }

        // Build destination
        let destination = Destination::builder()
            .set_to_addresses(Some(to_addresses))
            .build();

        // Build email content
        let subject = Content::builder()
            .data(&message.subject)
            .charset("UTF-8")
            .build()
            .map_err(|e| EmailProviderError::InvalidConfiguration(e.to_string()))?;

        let html_body = Content::builder()
            .data(&message.html_body)
            .charset("UTF-8")
            .build()
            .map_err(|e| EmailProviderError::InvalidConfiguration(e.to_string()))?;

        let mut body_builder = Body::builder().html(html_body);

        // Add text body if provided
        if let Some(text) = &message.text_body {
            let text_body = Content::builder()
                .data(text)
                .charset("UTF-8")
                .build()
                .map_err(|e| EmailProviderError::InvalidConfiguration(e.to_string()))?;
            body_builder = body_builder.text(text_body);
        }

        let body = body_builder.build();

        let ses_message = Message::builder().subject(subject).body(body).build();

        let email_content = EmailContent::builder().simple(ses_message).build();

        // Build and send the request
        let mut request = self
            .client
            .send_email()
            .from_email_address(self.build_from_address())
            .destination(destination)
            .content(email_content);

        // Add configuration set if configured
        if let Some(config_set) = &self.configuration_set {
            request = request.configuration_set_name(config_set);
        }

        let response = request.send().await.map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("AccessDenied")
                || error_msg.contains("InvalidClientTokenId")
                || error_msg.contains("SignatureDoesNotMatch")
            {
                EmailProviderError::AuthenticationFailed(error_msg)
            } else if error_msg.contains("Throttling") || error_msg.contains("rate") {
                EmailProviderError::RateLimited
            } else if error_msg.contains("connection") || error_msg.contains("timeout") {
                EmailProviderError::ConnectionError(error_msg)
            } else {
                EmailProviderError::SendFailed(error_msg)
            }
        })?;

        Ok(EmailSendResult::success(response.message_id))
    }

    async fn test_connection(&self) -> Result<(), EmailProviderError> {
        // Try to get the account details - this validates credentials and connectivity
        self.client
            .get_account()
            .send()
            .await
            .map(|_| ())
            .map_err(|e| {
                let error_msg = e.to_string();
                if error_msg.contains("AccessDenied")
                    || error_msg.contains("InvalidClientTokenId")
                    || error_msg.contains("SignatureDoesNotMatch")
                {
                    EmailProviderError::AuthenticationFailed(error_msg)
                } else if error_msg.contains("connection") || error_msg.contains("timeout") {
                    EmailProviderError::ConnectionError(error_msg)
                } else {
                    EmailProviderError::ConnectionError(format!(
                        "Failed to connect to SES: {}",
                        error_msg
                    ))
                }
            })
    }

    fn provider_name(&self) -> &'static str {
        "ses"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ses_config() -> SesConfig {
        SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Test Sender".to_string()),
            configuration_set: Some("tracking-set".to_string()),
        }
    }

    #[test]
    fn test_build_from_address_with_name() {
        // We can't easily test the full provider creation without AWS credentials,
        // so we test the address building logic separately
        let from_email = "test@example.com".to_string();
        let from_name = Some("Test User".to_string());

        let address = if let Some(name) = &from_name {
            format!("{} <{}>", name, from_email)
        } else {
            from_email.clone()
        };

        assert_eq!(address, "Test User <test@example.com>");
    }

    #[test]
    fn test_build_from_address_without_name() {
        let from_email = "test@example.com".to_string();
        let from_name: Option<String> = None;

        let address = if let Some(name) = &from_name {
            format!("{} <{}>", name, from_email)
        } else {
            from_email.clone()
        };

        assert_eq!(address, "test@example.com");
    }

    #[test]
    fn test_ses_config_validation() {
        let config = test_ses_config();
        assert!(!config.region.is_empty());
        assert!(config.access_key_id.is_some());
        assert!(config.secret_access_key.is_some());
    }

    #[test]
    fn test_ses_config_with_iam_role() {
        let config = SesConfig {
            region: "eu-west-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            from_email: "noreply@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        };

        assert!(config.access_key_id.is_none());
        assert!(config.secret_access_key.is_none());
    }

    #[tokio::test]
    async fn test_ses_provider_from_config_with_explicit_credentials() {
        let config = test_ses_config();
        let provider = SesEmailProvider::from_config(&config).await;
        assert!(provider.is_ok());
        let provider = provider.unwrap();
        assert_eq!(provider.provider_name(), "ses");
        assert_eq!(provider.from_email, "noreply@example.com");
        assert_eq!(provider.from_name, Some("Test Sender".to_string()));
        assert_eq!(provider.configuration_set, Some("tracking-set".to_string()));
    }

    #[tokio::test]
    async fn test_ses_provider_from_config_with_iam_role() {
        let config = SesConfig {
            region: "eu-west-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            from_email: "noreply@iam.example.com".to_string(),
            from_name: None,
            configuration_set: None,
        };
        let provider = SesEmailProvider::from_config(&config).await;
        assert!(provider.is_ok());
        let provider = provider.unwrap();
        assert_eq!(provider.from_email, "noreply@iam.example.com");
        assert!(provider.from_name.is_none());
        assert!(provider.configuration_set.is_none());
    }

    #[tokio::test]
    async fn test_ses_provider_build_from_address_with_name() {
        let config = test_ses_config();
        let provider = SesEmailProvider::from_config(&config).await.unwrap();
        let address = provider.build_from_address();
        assert_eq!(address, "Test Sender <noreply@example.com>");
    }

    #[tokio::test]
    async fn test_ses_provider_build_from_address_without_name() {
        let config = SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            from_email: "noreply@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        };
        let provider = SesEmailProvider::from_config(&config).await.unwrap();
        let address = provider.build_from_address();
        assert_eq!(address, "noreply@example.com");
    }

    #[tokio::test]
    async fn test_ses_provider_name() {
        let config = test_ses_config();
        let provider = SesEmailProvider::from_config(&config).await.unwrap();
        assert_eq!(provider.provider_name(), "ses");
    }

    #[tokio::test]
    async fn test_ses_send_no_recipients() {
        let config = test_ses_config();
        let provider = SesEmailProvider::from_config(&config).await.unwrap();
        let message = EmailMessage {
            to: vec![],
            subject: "Test".to_string(),
            html_body: "<p>Test</p>".to_string(),
            text_body: None,
        };
        let result = provider.send(&message).await;
        assert!(result.is_err());
        match result {
            Err(EmailProviderError::InvalidConfiguration(msg)) => {
                assert!(msg.contains("No recipients"));
            }
            other => panic!("Expected InvalidConfiguration error, got {:?}", other),
        }
    }
}
