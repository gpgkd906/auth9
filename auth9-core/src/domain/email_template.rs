//! Email template domain types
//!
//! Defines types for managing customizable email templates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

/// Available email template types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmailTemplateType {
    /// User invitation email
    Invitation,
    /// Password reset email
    PasswordReset,
    /// Email MFA verification code
    EmailMfa,
    /// Welcome email after accepting invitation
    Welcome,
    /// Email verification for email changes
    EmailVerification,
    /// Password changed notification
    PasswordChanged,
    /// Security alert (new login, suspicious activity)
    SecurityAlert,
}

impl EmailTemplateType {
    /// Get all template types
    pub fn all() -> &'static [EmailTemplateType] {
        &[
            EmailTemplateType::Invitation,
            EmailTemplateType::PasswordReset,
            EmailTemplateType::EmailMfa,
            EmailTemplateType::Welcome,
            EmailTemplateType::EmailVerification,
            EmailTemplateType::PasswordChanged,
            EmailTemplateType::SecurityAlert,
        ]
    }

    /// Get the string key for this template type (used in database)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invitation => "invitation",
            Self::PasswordReset => "password_reset",
            Self::EmailMfa => "email_mfa",
            Self::Welcome => "welcome",
            Self::EmailVerification => "email_verification",
            Self::PasswordChanged => "password_changed",
            Self::SecurityAlert => "security_alert",
        }
    }

    /// Get human-readable name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Invitation => "User Invitation",
            Self::PasswordReset => "Password Reset",
            Self::EmailMfa => "Email MFA",
            Self::Welcome => "Welcome",
            Self::EmailVerification => "Email Verification",
            Self::PasswordChanged => "Password Changed",
            Self::SecurityAlert => "Security Alert",
        }
    }

    /// Get description of the template
    pub fn description(&self) -> &'static str {
        match self {
            Self::Invitation => "Sent when inviting users to join a tenant",
            Self::PasswordReset => "Sent when a user requests to reset their password",
            Self::EmailMfa => "Sent when using email-based MFA verification",
            Self::Welcome => "Sent after a user accepts an invitation",
            Self::EmailVerification => "Sent when a user needs to verify their email address",
            Self::PasswordChanged => "Sent when a user's password has been changed",
            Self::SecurityAlert => "Sent for security events like new login from unknown device",
        }
    }

    /// Get available variables for this template type
    pub fn variables(&self) -> Vec<TemplateVariable> {
        let common = vec![
            TemplateVariable {
                name: "app_name".to_string(),
                description: "Application name (Auth9)".to_string(),
                example: "Auth9".to_string(),
            },
            TemplateVariable {
                name: "year".to_string(),
                description: "Current year".to_string(),
                example: "2026".to_string(),
            },
        ];

        let mut vars = match self {
            Self::Invitation => vec![
                TemplateVariable {
                    name: "inviter_name".to_string(),
                    description: "Name of the person sending the invitation".to_string(),
                    example: "John Doe".to_string(),
                },
                TemplateVariable {
                    name: "tenant_name".to_string(),
                    description: "Organization/tenant name".to_string(),
                    example: "Acme Corp".to_string(),
                },
                TemplateVariable {
                    name: "invite_link".to_string(),
                    description: "Invitation acceptance URL".to_string(),
                    example: "https://app.example.com/invite/abc123".to_string(),
                },
                TemplateVariable {
                    name: "expires_in_hours".to_string(),
                    description: "Hours until invitation expires".to_string(),
                    example: "72".to_string(),
                },
            ],
            Self::PasswordReset => vec![
                TemplateVariable {
                    name: "user_name".to_string(),
                    description: "Name of the user".to_string(),
                    example: "Jane Smith".to_string(),
                },
                TemplateVariable {
                    name: "reset_link".to_string(),
                    description: "Password reset URL".to_string(),
                    example: "https://app.example.com/reset/xyz789".to_string(),
                },
                TemplateVariable {
                    name: "expires_in_minutes".to_string(),
                    description: "Minutes until link expires".to_string(),
                    example: "30".to_string(),
                },
            ],
            Self::EmailMfa => vec![
                TemplateVariable {
                    name: "user_name".to_string(),
                    description: "Name of the user".to_string(),
                    example: "Jane Smith".to_string(),
                },
                TemplateVariable {
                    name: "verification_code".to_string(),
                    description: "MFA verification code".to_string(),
                    example: "123456".to_string(),
                },
                TemplateVariable {
                    name: "expires_in_minutes".to_string(),
                    description: "Minutes until code expires".to_string(),
                    example: "10".to_string(),
                },
            ],
            Self::Welcome => vec![
                TemplateVariable {
                    name: "user_name".to_string(),
                    description: "Name of the user".to_string(),
                    example: "Jane Smith".to_string(),
                },
                TemplateVariable {
                    name: "tenant_name".to_string(),
                    description: "Organization/tenant name".to_string(),
                    example: "Acme Corp".to_string(),
                },
                TemplateVariable {
                    name: "login_url".to_string(),
                    description: "Login page URL".to_string(),
                    example: "https://app.example.com/login".to_string(),
                },
            ],
            Self::EmailVerification => vec![
                TemplateVariable {
                    name: "user_name".to_string(),
                    description: "Name of the user".to_string(),
                    example: "Jane Smith".to_string(),
                },
                TemplateVariable {
                    name: "verification_link".to_string(),
                    description: "Email verification URL".to_string(),
                    example: "https://app.example.com/verify/abc123".to_string(),
                },
                TemplateVariable {
                    name: "expires_in_hours".to_string(),
                    description: "Hours until link expires".to_string(),
                    example: "24".to_string(),
                },
            ],
            Self::PasswordChanged => vec![
                TemplateVariable {
                    name: "user_name".to_string(),
                    description: "Name of the user".to_string(),
                    example: "Jane Smith".to_string(),
                },
                TemplateVariable {
                    name: "changed_at".to_string(),
                    description: "Date and time of password change".to_string(),
                    example: "January 31, 2026 at 10:30 AM UTC".to_string(),
                },
                TemplateVariable {
                    name: "ip_address".to_string(),
                    description: "IP address where change was made".to_string(),
                    example: "192.168.1.100".to_string(),
                },
            ],
            Self::SecurityAlert => vec![
                TemplateVariable {
                    name: "user_name".to_string(),
                    description: "Name of the user".to_string(),
                    example: "Jane Smith".to_string(),
                },
                TemplateVariable {
                    name: "event_type".to_string(),
                    description: "Type of security event".to_string(),
                    example: "New login from unknown device".to_string(),
                },
                TemplateVariable {
                    name: "device_info".to_string(),
                    description: "Device information".to_string(),
                    example: "Chrome on Windows".to_string(),
                },
                TemplateVariable {
                    name: "location".to_string(),
                    description: "Geographic location".to_string(),
                    example: "New York, US".to_string(),
                },
                TemplateVariable {
                    name: "timestamp".to_string(),
                    description: "Time of the event".to_string(),
                    example: "January 31, 2026 at 10:30 AM UTC".to_string(),
                },
            ],
        };

        vars.extend(common);
        vars
    }
}

impl fmt::Display for EmailTemplateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for EmailTemplateType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "invitation" => Ok(Self::Invitation),
            "password_reset" => Ok(Self::PasswordReset),
            "email_mfa" => Ok(Self::EmailMfa),
            "welcome" => Ok(Self::Welcome),
            "email_verification" => Ok(Self::EmailVerification),
            "password_changed" => Ok(Self::PasswordChanged),
            "security_alert" => Ok(Self::SecurityAlert),
            _ => Err(format!("Unknown email template type: {}", s)),
        }
    }
}

/// Email template content
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmailTemplateContent {
    /// Email subject line (can contain variables)
    pub subject: String,
    /// HTML body template
    pub html_body: String,
    /// Plain text body template
    pub text_body: String,
}

/// Template variable information for UI display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateVariable {
    /// Variable name (without braces)
    pub name: String,
    /// Description of what the variable contains
    pub description: String,
    /// Example value for preview
    pub example: String,
}

/// Template metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmailTemplateMetadata {
    /// Template type identifier
    pub template_type: EmailTemplateType,
    /// Human-readable name
    pub name: String,
    /// Description of when this template is used
    pub description: String,
    /// Available variables for this template
    pub variables: Vec<TemplateVariable>,
}

impl EmailTemplateMetadata {
    /// Create metadata from a template type
    pub fn from_type(template_type: EmailTemplateType) -> Self {
        Self {
            template_type,
            name: template_type.display_name().to_string(),
            description: template_type.description().to_string(),
            variables: template_type.variables(),
        }
    }
}

/// Complete template information with content
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmailTemplateWithContent {
    /// Template metadata
    pub metadata: EmailTemplateMetadata,
    /// Template content (subject, html_body, text_body)
    pub content: EmailTemplateContent,
    /// Whether the template has been customized (false = using default)
    pub is_customized: bool,
    /// Last update timestamp (if customized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Rendered email preview
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RenderedEmailPreview {
    /// Rendered subject line
    pub subject: String,
    /// Rendered HTML body
    pub html_body: String,
    /// Rendered plain text body
    pub text_body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_all() {
        let all = EmailTemplateType::all();
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn test_template_type_as_str() {
        assert_eq!(EmailTemplateType::Invitation.as_str(), "invitation");
        assert_eq!(EmailTemplateType::PasswordReset.as_str(), "password_reset");
        assert_eq!(EmailTemplateType::EmailMfa.as_str(), "email_mfa");
        assert_eq!(EmailTemplateType::Welcome.as_str(), "welcome");
        assert_eq!(
            EmailTemplateType::EmailVerification.as_str(),
            "email_verification"
        );
        assert_eq!(
            EmailTemplateType::PasswordChanged.as_str(),
            "password_changed"
        );
        assert_eq!(EmailTemplateType::SecurityAlert.as_str(), "security_alert");
    }

    #[test]
    fn test_template_type_from_str() {
        assert_eq!(
            "invitation".parse::<EmailTemplateType>().unwrap(),
            EmailTemplateType::Invitation
        );
        assert_eq!(
            "password_reset".parse::<EmailTemplateType>().unwrap(),
            EmailTemplateType::PasswordReset
        );
        assert!("unknown".parse::<EmailTemplateType>().is_err());
    }

    #[test]
    fn test_template_type_display_name() {
        assert_eq!(
            EmailTemplateType::Invitation.display_name(),
            "User Invitation"
        );
        assert_eq!(
            EmailTemplateType::PasswordReset.display_name(),
            "Password Reset"
        );
    }

    #[test]
    fn test_template_type_variables() {
        let vars = EmailTemplateType::Invitation.variables();
        assert!(vars.iter().any(|v| v.name == "inviter_name"));
        assert!(vars.iter().any(|v| v.name == "tenant_name"));
        assert!(vars.iter().any(|v| v.name == "invite_link"));
        assert!(vars.iter().any(|v| v.name == "app_name")); // Common variable
        assert!(vars.iter().any(|v| v.name == "year")); // Common variable
    }

    #[test]
    fn test_email_template_metadata_from_type() {
        let metadata = EmailTemplateMetadata::from_type(EmailTemplateType::Invitation);
        assert_eq!(metadata.template_type, EmailTemplateType::Invitation);
        assert_eq!(metadata.name, "User Invitation");
        assert!(!metadata.variables.is_empty());
    }

    #[test]
    fn test_email_template_content_serialization() {
        let content = EmailTemplateContent {
            subject: "Hello {{name}}".to_string(),
            html_body: "<h1>Hello {{name}}</h1>".to_string(),
            text_body: "Hello {{name}}".to_string(),
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("subject"));
        assert!(json.contains("html_body"));
        assert!(json.contains("text_body"));

        let deserialized: EmailTemplateContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subject, content.subject);
    }

    #[test]
    fn test_email_template_with_content_serialization() {
        let template = EmailTemplateWithContent {
            metadata: EmailTemplateMetadata::from_type(EmailTemplateType::Invitation),
            content: EmailTemplateContent {
                subject: "Test".to_string(),
                html_body: "<p>Test</p>".to_string(),
                text_body: "Test".to_string(),
            },
            is_customized: true,
            updated_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&template).unwrap();
        assert!(json.contains("metadata"));
        assert!(json.contains("content"));
        assert!(json.contains("is_customized"));
    }

    #[test]
    fn test_email_template_with_content_default_no_updated_at() {
        let template = EmailTemplateWithContent {
            metadata: EmailTemplateMetadata::from_type(EmailTemplateType::Invitation),
            content: EmailTemplateContent {
                subject: "Test".to_string(),
                html_body: "<p>Test</p>".to_string(),
                text_body: "Test".to_string(),
            },
            is_customized: false,
            updated_at: None,
        };

        let json = serde_json::to_string(&template).unwrap();
        // updated_at should be skipped when None
        assert!(!json.contains("updated_at"));
    }

    #[test]
    fn test_rendered_email_preview() {
        let preview = RenderedEmailPreview {
            subject: "Hello John".to_string(),
            html_body: "<h1>Hello John</h1>".to_string(),
            text_body: "Hello John".to_string(),
        };

        let json = serde_json::to_string(&preview).unwrap();
        let deserialized: RenderedEmailPreview = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subject, preview.subject);
    }

    #[test]
    fn test_all_template_types_have_variables() {
        for template_type in EmailTemplateType::all() {
            let vars = template_type.variables();
            assert!(
                !vars.is_empty(),
                "{:?} should have variables",
                template_type
            );
            // All templates should have app_name and year
            assert!(
                vars.iter().any(|v| v.name == "app_name"),
                "{:?} should have app_name",
                template_type
            );
            assert!(
                vars.iter().any(|v| v.name == "year"),
                "{:?} should have year",
                template_type
            );
        }
    }

    #[test]
    fn test_template_type_serde_roundtrip() {
        for template_type in EmailTemplateType::all() {
            let json = serde_json::to_string(template_type).unwrap();
            let deserialized: EmailTemplateType = serde_json::from_str(&json).unwrap();
            assert_eq!(*template_type, deserialized);
        }
    }
}
