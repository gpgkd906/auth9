//! Email provider domain types

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Email provider configuration - supports multiple provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmailProviderConfig {
    /// No email provider configured
    #[default]
    None,

    /// SMTP email provider
    Smtp(SmtpConfig),

    /// AWS Simple Email Service
    Ses(SesConfig),

    /// Oracle Email Delivery (uses SMTP protocol)
    Oracle(OracleEmailConfig),
}

impl EmailProviderConfig {
    /// Check if email is configured (not None)
    pub fn is_configured(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get the provider type as a string
    pub fn provider_type(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Smtp(_) => "smtp",
            Self::Ses(_) => "ses",
            Self::Oracle(_) => "oracle",
        }
    }
}

/// SMTP configuration for email sending
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct SmtpConfig {
    /// SMTP server host
    #[validate(length(min = 1, max = 255))]
    pub host: String,

    /// SMTP server port (typically 587 for TLS, 465 for SSL, 25 for unencrypted)
    pub port: u16,

    /// Username for authentication (optional)
    pub username: Option<String>,

    /// Password for authentication (stored encrypted)
    /// When reading from API, this will be masked as "***"
    pub password: Option<String>,

    /// Use TLS encryption
    #[serde(default = "default_true")]
    pub use_tls: bool,

    /// From email address
    #[validate(email)]
    pub from_email: String,

    /// From name (optional)
    pub from_name: Option<String>,
}

/// AWS SES configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct SesConfig {
    /// AWS region (e.g., "us-east-1")
    #[validate(length(min = 1, max = 50))]
    pub region: String,

    /// AWS access key ID (optional - uses IAM role if not provided)
    pub access_key_id: Option<String>,

    /// AWS secret access key (stored encrypted)
    pub secret_access_key: Option<String>,

    /// From email address (must be verified in SES)
    #[validate(email)]
    pub from_email: String,

    /// From name (optional)
    pub from_name: Option<String>,

    /// Configuration set name (optional, for tracking)
    pub configuration_set: Option<String>,
}

/// Oracle Email Delivery configuration (uses SMTP protocol)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Validate)]
pub struct OracleEmailConfig {
    /// SMTP endpoint (e.g., "smtp.us-ashburn-1.oraclecloud.com")
    #[validate(length(min = 1, max = 255))]
    pub smtp_endpoint: String,

    /// SMTP port (typically 587)
    #[serde(default = "default_smtp_port")]
    pub port: u16,

    /// SMTP username (from OCI console)
    #[validate(length(min = 1))]
    pub username: String,

    /// SMTP password (from OCI console, stored encrypted)
    pub password: String,

    /// From email address (must be in approved sender list)
    #[validate(email)]
    pub from_email: String,

    /// From name (optional)
    pub from_name: Option<String>,
}

fn default_true() -> bool {
    true
}

fn default_smtp_port() -> u16 {
    587
}

/// Tenant-level email settings override
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantEmailSettings {
    /// Override email provider for this tenant
    /// If None, uses system default
    pub provider: Option<EmailProviderConfig>,

    /// Custom from email for this tenant
    pub from_email: Option<String>,

    /// Custom from name for this tenant
    pub from_name: Option<String>,
}

impl TenantEmailSettings {
    /// Check if tenant has custom email settings
    pub fn has_override(&self) -> bool {
        self.provider.is_some() || self.from_email.is_some() || self.from_name.is_some()
    }
}

/// Email address with optional display name
#[derive(Debug, Clone)]
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

impl EmailAddress {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: None,
        }
    }

    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: Some(name.into()),
        }
    }
}

/// Email message to be sent
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: Vec<EmailAddress>,
    pub subject: String,
    pub html_body: String,
    pub text_body: Option<String>,
}

impl EmailMessage {
    pub fn new(to: EmailAddress, subject: impl Into<String>, html_body: impl Into<String>) -> Self {
        Self {
            to: vec![to],
            subject: subject.into(),
            html_body: html_body.into(),
            text_body: None,
        }
    }

    pub fn with_text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }
}

/// Result of sending an email
#[derive(Debug)]
pub struct EmailSendResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

impl EmailSendResult {
    pub fn success(message_id: Option<String>) -> Self {
        Self {
            success: true,
            message_id,
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            message_id: None,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_provider_config_default() {
        let config = EmailProviderConfig::default();
        assert!(matches!(config, EmailProviderConfig::None));
        assert!(!config.is_configured());
    }

    #[test]
    fn test_email_provider_config_smtp() {
        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            use_tls: true,
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Example".to_string()),
        });

        assert!(config.is_configured());
        assert_eq!(config.provider_type(), "smtp");
    }

    #[test]
    fn test_email_provider_config_ses() {
        let config = EmailProviderConfig::Ses(SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: Some("AKIA...".to_string()),
            secret_access_key: Some("secret".to_string()),
            from_email: "noreply@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        });

        assert!(config.is_configured());
        assert_eq!(config.provider_type(), "ses");
    }

    #[test]
    fn test_email_provider_config_oracle() {
        let config = EmailProviderConfig::Oracle(OracleEmailConfig {
            smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com".to_string(),
            port: 587,
            username: "ocid1.user...".to_string(),
            password: "password".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: None,
        });

        assert!(config.is_configured());
        assert_eq!(config.provider_type(), "oracle");
    }

    #[test]
    fn test_email_provider_config_serialization() {
        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            use_tls: true,
            from_email: "test@example.com".to_string(),
            from_name: None,
        });

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"type\":\"smtp\""));

        let parsed: EmailProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_email_provider_config_deserialization() {
        let json = r#"{"type": "none"}"#;
        let config: EmailProviderConfig = serde_json::from_str(json).unwrap();
        assert!(matches!(config, EmailProviderConfig::None));
    }

    #[test]
    fn test_tenant_email_settings_default() {
        let settings = TenantEmailSettings::default();
        assert!(!settings.has_override());
    }

    #[test]
    fn test_tenant_email_settings_with_override() {
        let settings = TenantEmailSettings {
            provider: None,
            from_email: Some("custom@tenant.com".to_string()),
            from_name: None,
        };
        assert!(settings.has_override());
    }

    #[test]
    fn test_email_address() {
        let addr = EmailAddress::new("test@example.com");
        assert_eq!(addr.email, "test@example.com");
        assert!(addr.name.is_none());

        let addr = EmailAddress::with_name("test@example.com", "Test User");
        assert_eq!(addr.email, "test@example.com");
        assert_eq!(addr.name.unwrap(), "Test User");
    }

    #[test]
    fn test_email_message() {
        let msg = EmailMessage::new(
            EmailAddress::new("to@example.com"),
            "Subject",
            "<p>Hello</p>",
        );

        assert_eq!(msg.to.len(), 1);
        assert_eq!(msg.subject, "Subject");
        assert_eq!(msg.html_body, "<p>Hello</p>");
        assert!(msg.text_body.is_none());

        let msg = msg.with_text_body("Hello");
        assert_eq!(msg.text_body.unwrap(), "Hello");
    }

    #[test]
    fn test_email_send_result() {
        let success = EmailSendResult::success(Some("msg-123".to_string()));
        assert!(success.success);
        assert_eq!(success.message_id.unwrap(), "msg-123");
        assert!(success.error.is_none());

        let failure = EmailSendResult::failure("Connection refused");
        assert!(!failure.success);
        assert!(failure.message_id.is_none());
        assert_eq!(failure.error.unwrap(), "Connection refused");
    }

    #[test]
    fn test_smtp_config_validation() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            use_tls: true,
            from_email: "valid@example.com".to_string(),
            from_name: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_smtp_config_invalid_email() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: None,
            password: None,
            use_tls: true,
            from_email: "not-an-email".to_string(),
            from_name: None,
        };

        assert!(config.validate().is_err());
    }
}
