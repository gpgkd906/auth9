//! Email provider domain types

use crate::keycloak::SmtpServerConfig;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use validator::Validate;

/// Version byte for SES SMTP password calculation
const SES_SMTP_PASSWORD_VERSION: u8 = 0x04;

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

    /// Convert to Keycloak SmtpServerConfig
    ///
    /// Returns None if conversion is not possible (e.g., SES without credentials)
    pub fn to_keycloak_smtp(&self) -> Option<SmtpServerConfig> {
        match self {
            Self::None => {
                // Return empty config to clear SMTP settings in Keycloak
                Some(SmtpServerConfig::default())
            }
            Self::Smtp(cfg) => Some(SmtpServerConfig {
                host: Some(cfg.host.clone()),
                port: Some(cfg.port.to_string()),
                from: Some(cfg.from_email.clone()),
                from_display_name: cfg.from_name.clone(),
                auth: Some(cfg.username.is_some().to_string()),
                user: cfg.username.clone(),
                password: cfg.password.clone(),
                ssl: Some((!cfg.use_tls && cfg.port == 465).to_string()),
                starttls: Some(cfg.use_tls.to_string()),
            }),
            Self::Ses(cfg) => {
                // SES requires credentials to compute SMTP password
                let (access_key_id, secret_access_key) =
                    match (&cfg.access_key_id, &cfg.secret_access_key) {
                        (Some(key_id), Some(secret)) => (key_id.clone(), secret.clone()),
                        _ => return None, // Cannot sync without credentials
                    };

                let smtp_password = compute_ses_smtp_password(&secret_access_key, &cfg.region);

                Some(SmtpServerConfig {
                    host: Some(format!("email-smtp.{}.amazonaws.com", cfg.region)),
                    port: Some("587".to_string()),
                    from: Some(cfg.from_email.clone()),
                    from_display_name: cfg.from_name.clone(),
                    auth: Some("true".to_string()),
                    user: Some(access_key_id),
                    password: Some(smtp_password),
                    ssl: Some("false".to_string()),
                    starttls: Some("true".to_string()),
                })
            }
            Self::Oracle(cfg) => Some(SmtpServerConfig {
                host: Some(cfg.smtp_endpoint.clone()),
                port: Some(cfg.port.to_string()),
                from: Some(cfg.from_email.clone()),
                from_display_name: cfg.from_name.clone(),
                auth: Some("true".to_string()),
                user: Some(cfg.username.clone()),
                password: Some(cfg.password.clone()),
                ssl: Some("false".to_string()),
                starttls: Some("true".to_string()),
            }),
        }
    }
}

/// Compute AWS SES SMTP password from secret access key
///
/// AWS SES uses Signature Version 4 algorithm to derive SMTP credentials.
/// Reference: https://docs.aws.amazon.com/ses/latest/dg/smtp-credentials.html
pub fn compute_ses_smtp_password(secret_access_key: &str, region: &str) -> String {
    use base64::Engine;

    type HmacSha256 = Hmac<Sha256>;

    // Step 1: Create a signature using the secret access key
    let date = "11111111"; // SES uses a fixed date for SMTP
    let service = "ses";

    // kDate = HMAC("AWS4" + secret, Date)
    let key = format!("AWS4{}", secret_access_key);
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts any key size");
    mac.update(date.as_bytes());
    let k_date = mac.finalize().into_bytes();

    // kRegion = HMAC(kDate, Region)
    let mut mac =
        HmacSha256::new_from_slice(&k_date).expect("HMAC accepts derived key of any size");
    mac.update(region.as_bytes());
    let k_region = mac.finalize().into_bytes();

    // kService = HMAC(kRegion, Service)
    let mut mac =
        HmacSha256::new_from_slice(&k_region).expect("HMAC accepts derived key of any size");
    mac.update(service.as_bytes());
    let k_service = mac.finalize().into_bytes();

    // kSigning = HMAC(kService, "aws4_request")
    let mut mac =
        HmacSha256::new_from_slice(&k_service).expect("HMAC accepts derived key of any size");
    mac.update(b"aws4_request");
    let k_signing = mac.finalize().into_bytes();

    // signature = HMAC(kSigning, "SendRawEmail")
    let mut mac =
        HmacSha256::new_from_slice(&k_signing).expect("HMAC accepts derived key of any size");
    mac.update(b"SendRawEmail");
    let signature = mac.finalize().into_bytes();

    // Final SMTP password = Base64(version_byte + signature)
    let mut password_bytes = vec![SES_SMTP_PASSWORD_VERSION];
    password_bytes.extend_from_slice(&signature);

    base64::engine::general_purpose::STANDARD.encode(&password_bytes)
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
    fn test_compute_ses_smtp_password() {
        // Test with known values
        // The algorithm should produce a base64 string starting with version byte 0x04
        let password = compute_ses_smtp_password("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY", "us-east-1");

        // Password should be base64 encoded and start with the version byte
        assert!(!password.is_empty());

        // Decode and verify version byte
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&password)
            .unwrap();
        assert_eq!(decoded[0], 0x04); // Version byte
        assert_eq!(decoded.len(), 33); // 1 version byte + 32 bytes HMAC-SHA256
    }

    #[test]
    fn test_compute_ses_smtp_password_deterministic() {
        // Same input should always produce same output
        let password1 = compute_ses_smtp_password("secret-key", "us-west-2");
        let password2 = compute_ses_smtp_password("secret-key", "us-west-2");
        assert_eq!(password1, password2);

        // Different region should produce different password
        let password3 = compute_ses_smtp_password("secret-key", "eu-west-1");
        assert_ne!(password1, password3);
    }

    #[test]
    fn test_to_keycloak_smtp_none() {
        let config = EmailProviderConfig::None;
        let smtp = config.to_keycloak_smtp().unwrap();

        // Should return empty config to clear SMTP settings
        assert!(smtp.host.is_none());
        assert!(smtp.port.is_none());
        assert!(smtp.from.is_none());
    }

    #[test]
    fn test_to_keycloak_smtp_smtp_provider() {
        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: Some("user@example.com".to_string()),
            password: Some("password123".to_string()),
            use_tls: true,
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Auth9".to_string()),
        });

        let smtp = config.to_keycloak_smtp().unwrap();

        assert_eq!(smtp.host, Some("smtp.example.com".to_string()));
        assert_eq!(smtp.port, Some("587".to_string()));
        assert_eq!(smtp.from, Some("noreply@example.com".to_string()));
        assert_eq!(smtp.from_display_name, Some("Auth9".to_string()));
        assert_eq!(smtp.auth, Some("true".to_string()));
        assert_eq!(smtp.user, Some("user@example.com".to_string()));
        assert_eq!(smtp.password, Some("password123".to_string()));
        assert_eq!(smtp.starttls, Some("true".to_string()));
    }

    #[test]
    fn test_to_keycloak_smtp_smtp_no_auth() {
        let config = EmailProviderConfig::Smtp(SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 25,
            username: None,
            password: None,
            use_tls: false,
            from_email: "noreply@example.com".to_string(),
            from_name: None,
        });

        let smtp = config.to_keycloak_smtp().unwrap();

        assert_eq!(smtp.auth, Some("false".to_string()));
        assert!(smtp.user.is_none());
        assert!(smtp.password.is_none());
        assert_eq!(smtp.starttls, Some("false".to_string()));
    }

    #[test]
    fn test_to_keycloak_smtp_ses_provider() {
        let config = EmailProviderConfig::Ses(SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Auth9".to_string()),
            configuration_set: None,
        });

        let smtp = config.to_keycloak_smtp().unwrap();

        assert_eq!(
            smtp.host,
            Some("email-smtp.us-east-1.amazonaws.com".to_string())
        );
        assert_eq!(smtp.port, Some("587".to_string()));
        assert_eq!(smtp.from, Some("noreply@example.com".to_string()));
        assert_eq!(smtp.auth, Some("true".to_string()));
        assert_eq!(smtp.user, Some("AKIAIOSFODNN7EXAMPLE".to_string()));
        // Password should be computed SES SMTP password
        assert!(smtp.password.is_some());
        assert_ne!(
            smtp.password.as_ref().unwrap(),
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        );
        assert_eq!(smtp.starttls, Some("true".to_string()));
    }

    #[test]
    fn test_to_keycloak_smtp_ses_no_credentials() {
        let config = EmailProviderConfig::Ses(SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: None,
            secret_access_key: None,
            from_email: "noreply@example.com".to_string(),
            from_name: None,
            configuration_set: None,
        });

        // Should return None when credentials are missing
        let smtp = config.to_keycloak_smtp();
        assert!(smtp.is_none());
    }

    #[test]
    fn test_to_keycloak_smtp_oracle_provider() {
        let config = EmailProviderConfig::Oracle(OracleEmailConfig {
            smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com".to_string(),
            port: 587,
            username: "ocid1.user.oc1..example".to_string(),
            password: "oracle-password".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Auth9".to_string()),
        });

        let smtp = config.to_keycloak_smtp().unwrap();

        assert_eq!(
            smtp.host,
            Some("smtp.us-ashburn-1.oraclecloud.com".to_string())
        );
        assert_eq!(smtp.port, Some("587".to_string()));
        assert_eq!(smtp.from, Some("noreply@example.com".to_string()));
        assert_eq!(smtp.from_display_name, Some("Auth9".to_string()));
        assert_eq!(smtp.auth, Some("true".to_string()));
        assert_eq!(smtp.user, Some("ocid1.user.oc1..example".to_string()));
        assert_eq!(smtp.password, Some("oracle-password".to_string()));
        assert_eq!(smtp.starttls, Some("true".to_string()));
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
