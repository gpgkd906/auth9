//! Email template system
//!
//! Provides simple variable substitution for email templates.
//! Variables are specified using {{variable_name}} syntax.

use crate::domain::{EmailTemplateContent, EmailTemplateType};
use std::collections::HashMap;

/// Available email templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailTemplate {
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

impl EmailTemplate {
    /// Get the subject line for this template
    pub fn subject(&self) -> &'static str {
        match self {
            Self::Invitation => "You've been invited to join {{tenant_name}}",
            Self::PasswordReset => "Reset your password",
            Self::EmailMfa => "Your verification code: {{verification_code}}",
            Self::Welcome => "Welcome to {{tenant_name}}!",
            Self::EmailVerification => "Verify your email address",
            Self::PasswordChanged => "Your password has been changed",
            Self::SecurityAlert => "Security Alert: {{event_type}}",
        }
    }

    /// Get the HTML body template
    pub fn html_body(&self) -> &'static str {
        match self {
            Self::Invitation => INVITATION_TEMPLATE,
            Self::PasswordReset => PASSWORD_RESET_TEMPLATE,
            Self::EmailMfa => EMAIL_MFA_TEMPLATE,
            Self::Welcome => WELCOME_TEMPLATE,
            Self::EmailVerification => EMAIL_VERIFICATION_TEMPLATE,
            Self::PasswordChanged => PASSWORD_CHANGED_TEMPLATE,
            Self::SecurityAlert => SECURITY_ALERT_TEMPLATE,
        }
    }

    /// Get the plain text body template
    pub fn text_body(&self) -> &'static str {
        match self {
            Self::Invitation => INVITATION_TEMPLATE_TEXT,
            Self::PasswordReset => PASSWORD_RESET_TEMPLATE_TEXT,
            Self::EmailMfa => EMAIL_MFA_TEMPLATE_TEXT,
            Self::Welcome => WELCOME_TEMPLATE_TEXT,
            Self::EmailVerification => EMAIL_VERIFICATION_TEMPLATE_TEXT,
            Self::PasswordChanged => PASSWORD_CHANGED_TEMPLATE_TEXT,
            Self::SecurityAlert => SECURITY_ALERT_TEMPLATE_TEXT,
        }
    }

    /// Convert from domain EmailTemplateType
    pub fn from_template_type(template_type: EmailTemplateType) -> Self {
        match template_type {
            EmailTemplateType::Invitation => Self::Invitation,
            EmailTemplateType::PasswordReset => Self::PasswordReset,
            EmailTemplateType::EmailMfa => Self::EmailMfa,
            EmailTemplateType::Welcome => Self::Welcome,
            EmailTemplateType::EmailVerification => Self::EmailVerification,
            EmailTemplateType::PasswordChanged => Self::PasswordChanged,
            EmailTemplateType::SecurityAlert => Self::SecurityAlert,
        }
    }

    /// Get default content for a template type
    pub fn default_content(template_type: EmailTemplateType) -> EmailTemplateContent {
        let template = Self::from_template_type(template_type);
        EmailTemplateContent {
            subject: template.subject().to_string(),
            html_body: template.html_body().to_string(),
            text_body: template.text_body().to_string(),
        }
    }
}

/// Template rendering engine with variable substitution
#[derive(Debug, Default)]
pub struct TemplateEngine {
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Set multiple variables from an iterator
    pub fn set_all<I, K, V>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in iter {
            self.variables.insert(k.into(), v.into());
        }
        self
    }

    /// Render a template string, replacing {{variable}} with values
    pub fn render(&self, template: &str) -> String {
        let mut result = template.to_string();

        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Render a complete email template
    pub fn render_template(&self, template: EmailTemplate) -> RenderedEmail {
        RenderedEmail {
            subject: self.render(template.subject()),
            html_body: self.render(template.html_body()),
            text_body: self.render(template.text_body()),
        }
    }
}

/// Rendered email with all variables substituted
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

// ============================================================================
// Email Templates
// ============================================================================

const INVITATION_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Invitation</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2563eb; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .button { display: inline-block; background-color: #2563eb; color: #ffffff; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600; }
        .button:hover { background-color: #1d4ed8; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .link { color: #2563eb; word-break: break-all; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>You're Invited!</h1>
        </div>
        <div class="content">
            <p>Hi there,</p>
            <p><strong>{{inviter_name}}</strong> has invited you to join <strong>{{tenant_name}}</strong>.</p>
            <p>Click the button below to accept the invitation and create your account:</p>
            <p style="text-align: center; margin: 30px 0;">
                <a href="{{invite_link}}" class="button">Accept Invitation</a>
            </p>
            <p style="font-size: 14px; color: #666;">
                Or copy and paste this link into your browser:<br>
                <a href="{{invite_link}}" class="link">{{invite_link}}</a>
            </p>
            <p style="font-size: 14px; color: #666;">
                This invitation will expire in {{expires_in_hours}} hours.
            </p>
        </div>
        <div class="footer">
            <p>If you didn't expect this invitation, you can safely ignore this email.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const INVITATION_TEMPLATE_TEXT: &str = r#"You're Invited!

Hi there,

{{inviter_name}} has invited you to join {{tenant_name}}.

Click the link below to accept the invitation and create your account:

{{invite_link}}

This invitation will expire in {{expires_in_hours}} hours.

If you didn't expect this invitation, you can safely ignore this email.

(c) {{year}} {{app_name}}"#;

const PASSWORD_RESET_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Password Reset</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2563eb; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .button { display: inline-block; background-color: #2563eb; color: #ffffff; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600; }
        .button:hover { background-color: #1d4ed8; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .link { color: #2563eb; word-break: break-all; }
        .warning { background-color: #fef3c7; border: 1px solid #f59e0b; padding: 12px; border-radius: 6px; margin: 20px 0; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Reset Your Password</h1>
        </div>
        <div class="content">
            <p>Hi {{user_name}},</p>
            <p>We received a request to reset your password for your {{app_name}} account.</p>
            <p>Click the button below to reset your password:</p>
            <p style="text-align: center; margin: 30px 0;">
                <a href="{{reset_link}}" class="button">Reset Password</a>
            </p>
            <p style="font-size: 14px; color: #666;">
                Or copy and paste this link into your browser:<br>
                <a href="{{reset_link}}" class="link">{{reset_link}}</a>
            </p>
            <div class="warning">
                <strong>Security Notice:</strong> This link will expire in {{expires_in_minutes}} minutes. If you didn't request a password reset, please ignore this email or contact support.
            </div>
        </div>
        <div class="footer">
            <p>This is an automated message. Please do not reply.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const PASSWORD_RESET_TEMPLATE_TEXT: &str = r#"Reset Your Password

Hi {{user_name}},

We received a request to reset your password for your {{app_name}} account.

Click the link below to reset your password:

{{reset_link}}

SECURITY NOTICE: This link will expire in {{expires_in_minutes}} minutes.
If you didn't request a password reset, please ignore this email or contact support.

This is an automated message. Please do not reply.

(c) {{year}} {{app_name}}"#;

// ============================================================================
// Email MFA Template
// ============================================================================

const EMAIL_MFA_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Verification Code</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2563eb; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .code-box { background-color: #f3f4f6; border-radius: 8px; padding: 20px; text-align: center; margin: 30px 0; }
        .code { font-size: 32px; font-weight: bold; letter-spacing: 8px; color: #1f2937; font-family: monospace; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .warning { background-color: #fef3c7; border: 1px solid #f59e0b; padding: 12px; border-radius: 6px; margin: 20px 0; font-size: 14px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Verification Code</h1>
        </div>
        <div class="content">
            <p>Hi {{user_name}},</p>
            <p>Use the following code to complete your sign-in:</p>
            <div class="code-box">
                <span class="code">{{verification_code}}</span>
            </div>
            <div class="warning">
                <strong>Important:</strong> This code will expire in {{expires_in_minutes}} minutes. Never share this code with anyone.
            </div>
            <p style="font-size: 14px; color: #666;">
                If you didn't attempt to sign in, please secure your account immediately by changing your password.
            </p>
        </div>
        <div class="footer">
            <p>This is an automated message. Please do not reply.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const EMAIL_MFA_TEMPLATE_TEXT: &str = r#"Verification Code

Hi {{user_name}},

Use the following code to complete your sign-in:

{{verification_code}}

IMPORTANT: This code will expire in {{expires_in_minutes}} minutes. Never share this code with anyone.

If you didn't attempt to sign in, please secure your account immediately by changing your password.

This is an automated message. Please do not reply.

(c) {{year}} {{app_name}}"#;

// ============================================================================
// Welcome Template
// ============================================================================

const WELCOME_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Welcome</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2563eb; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .button { display: inline-block; background-color: #2563eb; color: #ffffff; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600; }
        .button:hover { background-color: #1d4ed8; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .features { background-color: #f9fafb; border-radius: 8px; padding: 20px; margin: 20px 0; }
        .features ul { margin: 0; padding-left: 20px; }
        .features li { margin: 8px 0; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Welcome to {{tenant_name}}!</h1>
        </div>
        <div class="content">
            <p>Hi {{user_name}},</p>
            <p>Your account has been successfully created. You're now a member of <strong>{{tenant_name}}</strong>.</p>
            <div class="features">
                <p><strong>Getting Started:</strong></p>
                <ul>
                    <li>Complete your profile settings</li>
                    <li>Enable two-factor authentication for extra security</li>
                    <li>Explore the available features and services</li>
                </ul>
            </div>
            <p style="text-align: center; margin: 30px 0;">
                <a href="{{login_url}}" class="button">Go to Dashboard</a>
            </p>
        </div>
        <div class="footer">
            <p>If you have any questions, contact your organization administrator.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const WELCOME_TEMPLATE_TEXT: &str = r#"Welcome to {{tenant_name}}!

Hi {{user_name}},

Your account has been successfully created. You're now a member of {{tenant_name}}.

Getting Started:
- Complete your profile settings
- Enable two-factor authentication for extra security
- Explore the available features and services

Go to Dashboard: {{login_url}}

If you have any questions, contact your organization administrator.

(c) {{year}} {{app_name}}"#;

// ============================================================================
// Email Verification Template
// ============================================================================

const EMAIL_VERIFICATION_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Verify Your Email</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2563eb; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .button { display: inline-block; background-color: #2563eb; color: #ffffff; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600; }
        .button:hover { background-color: #1d4ed8; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .link { color: #2563eb; word-break: break-all; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Verify Your Email Address</h1>
        </div>
        <div class="content">
            <p>Hi {{user_name}},</p>
            <p>Please verify your email address by clicking the button below:</p>
            <p style="text-align: center; margin: 30px 0;">
                <a href="{{verification_link}}" class="button">Verify Email</a>
            </p>
            <p style="font-size: 14px; color: #666;">
                Or copy and paste this link into your browser:<br>
                <a href="{{verification_link}}" class="link">{{verification_link}}</a>
            </p>
            <p style="font-size: 14px; color: #666;">
                This link will expire in {{expires_in_hours}} hours.
            </p>
        </div>
        <div class="footer">
            <p>If you didn't request this verification, you can safely ignore this email.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const EMAIL_VERIFICATION_TEMPLATE_TEXT: &str = r#"Verify Your Email Address

Hi {{user_name}},

Please verify your email address by clicking the link below:

{{verification_link}}

This link will expire in {{expires_in_hours}} hours.

If you didn't request this verification, you can safely ignore this email.

(c) {{year}} {{app_name}}"#;

// ============================================================================
// Password Changed Template
// ============================================================================

const PASSWORD_CHANGED_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Password Changed</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #2563eb; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .info-box { background-color: #f3f4f6; border-radius: 8px; padding: 16px; margin: 20px 0; }
        .info-row { display: flex; margin: 8px 0; }
        .info-label { font-weight: 600; width: 120px; color: #6b7280; }
        .warning { background-color: #fef2f2; border: 1px solid #ef4444; padding: 12px; border-radius: 6px; margin: 20px 0; font-size: 14px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Password Changed</h1>
        </div>
        <div class="content">
            <p>Hi {{user_name}},</p>
            <p>Your password has been successfully changed.</p>
            <div class="info-box">
                <p style="margin: 0;"><strong>Change Details:</strong></p>
                <p style="margin: 8px 0 0 0; font-size: 14px;">
                    <span style="color: #6b7280;">Time:</span> {{changed_at}}<br>
                    <span style="color: #6b7280;">IP Address:</span> {{ip_address}}
                </p>
            </div>
            <div class="warning">
                <strong>Didn't make this change?</strong> If you didn't change your password, your account may be compromised. Please contact support immediately and reset your password.
            </div>
        </div>
        <div class="footer">
            <p>This is an automated security notification.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const PASSWORD_CHANGED_TEMPLATE_TEXT: &str = r#"Password Changed

Hi {{user_name}},

Your password has been successfully changed.

Change Details:
- Time: {{changed_at}}
- IP Address: {{ip_address}}

DIDN'T MAKE THIS CHANGE?
If you didn't change your password, your account may be compromised.
Please contact support immediately and reset your password.

This is an automated security notification.

(c) {{year}} {{app_name}}"#;

// ============================================================================
// Security Alert Template
// ============================================================================

const SECURITY_ALERT_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Security Alert</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; color: #333; margin: 0; padding: 0; background-color: #f5f5f5; }
        .container { max-width: 600px; margin: 40px auto; padding: 40px; background: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .header { text-align: center; margin-bottom: 30px; }
        .header h1 { color: #dc2626; margin: 0; font-size: 24px; }
        .content { margin-bottom: 30px; }
        .footer { text-align: center; font-size: 12px; color: #666; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; }
        .alert-box { background-color: #fef2f2; border: 1px solid #fecaca; border-radius: 8px; padding: 20px; margin: 20px 0; }
        .alert-icon { font-size: 48px; text-align: center; margin-bottom: 16px; }
        .info-box { background-color: #f3f4f6; border-radius: 8px; padding: 16px; margin: 20px 0; }
        .button { display: inline-block; background-color: #dc2626; color: #ffffff; padding: 14px 28px; text-decoration: none; border-radius: 6px; font-weight: 600; }
        .button:hover { background-color: #b91c1c; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Security Alert</h1>
        </div>
        <div class="content">
            <div class="alert-box">
                <p style="margin: 0; font-size: 18px;"><strong>{{event_type}}</strong></p>
            </div>
            <p>Hi {{user_name}},</p>
            <p>We detected unusual activity on your account:</p>
            <div class="info-box">
                <p style="margin: 0;"><strong>Activity Details:</strong></p>
                <p style="margin: 8px 0 0 0; font-size: 14px;">
                    <span style="color: #6b7280;">Event:</span> {{event_type}}<br>
                    <span style="color: #6b7280;">Device:</span> {{device_info}}<br>
                    <span style="color: #6b7280;">Location:</span> {{location}}<br>
                    <span style="color: #6b7280;">Time:</span> {{timestamp}}
                </p>
            </div>
            <p><strong>If this was you:</strong> You can safely ignore this email.</p>
            <p><strong>If this wasn't you:</strong> We recommend changing your password immediately and reviewing your account activity.</p>
        </div>
        <div class="footer">
            <p>This is an automated security notification.</p>
            <p>&copy; {{year}} {{app_name}}</p>
        </div>
    </div>
</body>
</html>"#;

const SECURITY_ALERT_TEMPLATE_TEXT: &str = r#"Security Alert

{{event_type}}

Hi {{user_name}},

We detected unusual activity on your account:

Activity Details:
- Event: {{event_type}}
- Device: {{device_info}}
- Location: {{location}}
- Time: {{timestamp}}

IF THIS WAS YOU:
You can safely ignore this email.

IF THIS WASN'T YOU:
We recommend changing your password immediately and reviewing your account activity.

This is an automated security notification.

(c) {{year}} {{app_name}}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_basic() {
        let mut engine = TemplateEngine::new();
        engine.set("name", "John");

        let result = engine.render("Hello, {{name}}!");
        assert_eq!(result, "Hello, John!");
    }

    #[test]
    fn test_template_engine_multiple_vars() {
        let mut engine = TemplateEngine::new();
        engine.set("first", "John");
        engine.set("last", "Doe");

        let result = engine.render("Hello, {{first}} {{last}}!");
        assert_eq!(result, "Hello, John Doe!");
    }

    #[test]
    fn test_template_engine_set_all() {
        let mut engine = TemplateEngine::new();
        engine.set_all([("a", "1"), ("b", "2")]);

        let result = engine.render("{{a}} + {{b}}");
        assert_eq!(result, "1 + 2");
    }

    #[test]
    fn test_template_engine_missing_var() {
        let engine = TemplateEngine::new();
        let result = engine.render("Hello, {{name}}!");
        // Missing variables are left as-is
        assert_eq!(result, "Hello, {{name}}!");
    }

    #[test]
    fn test_template_engine_repeated_var() {
        let mut engine = TemplateEngine::new();
        engine.set("name", "Alice");

        let result = engine.render("{{name}} loves {{name}}");
        assert_eq!(result, "Alice loves Alice");
    }

    #[test]
    fn test_invitation_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("inviter_name", "Admin User")
            .set("tenant_name", "Acme Corp")
            .set("invite_link", "https://example.com/invite/abc123")
            .set("expires_in_hours", "72")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::Invitation);

        assert!(rendered.subject.contains("Acme Corp"));
        assert!(rendered.html_body.contains("Admin User"));
        assert!(rendered
            .html_body
            .contains("https://example.com/invite/abc123"));
        assert!(rendered.text_body.contains("72 hours"));
    }

    #[test]
    fn test_password_reset_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", "John Doe")
            .set("reset_link", "https://example.com/reset/xyz")
            .set("expires_in_minutes", "30")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::PasswordReset);

        assert!(rendered.subject.contains("Reset"));
        assert!(rendered.html_body.contains("John Doe"));
        assert!(rendered.html_body.contains("30 minutes"));
        assert!(rendered.text_body.contains("xyz"));
    }

    #[test]
    fn test_email_template_subjects() {
        assert!(EmailTemplate::Invitation.subject().contains("invited"));
        assert!(EmailTemplate::PasswordReset.subject().contains("password"));
        assert!(EmailTemplate::EmailMfa
            .subject()
            .contains("verification_code"));
        assert!(EmailTemplate::Welcome.subject().contains("Welcome"));
        assert!(EmailTemplate::EmailVerification
            .subject()
            .contains("Verify"));
        assert!(EmailTemplate::PasswordChanged.subject().contains("changed"));
        assert!(EmailTemplate::SecurityAlert.subject().contains("Security"));
    }

    #[test]
    fn test_rendered_email_clone() {
        let rendered = RenderedEmail {
            subject: "Test".to_string(),
            html_body: "<p>Test</p>".to_string(),
            text_body: "Test".to_string(),
        };

        let cloned = rendered.clone();
        assert_eq!(cloned.subject, rendered.subject);
    }

    #[test]
    fn test_email_mfa_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", "Jane Doe")
            .set("verification_code", "123456")
            .set("expires_in_minutes", "10")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::EmailMfa);

        assert!(rendered.subject.contains("123456"));
        assert!(rendered.html_body.contains("Jane Doe"));
        assert!(rendered.html_body.contains("123456"));
        assert!(rendered.text_body.contains("10 minutes"));
    }

    #[test]
    fn test_welcome_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", "Jane Doe")
            .set("tenant_name", "Acme Corp")
            .set("login_url", "https://example.com/login")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::Welcome);

        assert!(rendered.subject.contains("Acme Corp"));
        assert!(rendered.html_body.contains("Jane Doe"));
        assert!(rendered.html_body.contains("https://example.com/login"));
    }

    #[test]
    fn test_email_verification_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", "Jane Doe")
            .set("verification_link", "https://example.com/verify/abc123")
            .set("expires_in_hours", "24")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::EmailVerification);

        assert!(rendered.subject.contains("Verify"));
        assert!(rendered
            .html_body
            .contains("https://example.com/verify/abc123"));
        assert!(rendered.text_body.contains("24 hours"));
    }

    #[test]
    fn test_password_changed_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", "Jane Doe")
            .set("changed_at", "January 31, 2026 at 10:30 AM UTC")
            .set("ip_address", "192.168.1.100")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::PasswordChanged);

        assert!(rendered.subject.contains("changed"));
        assert!(rendered.html_body.contains("Jane Doe"));
        assert!(rendered.html_body.contains("192.168.1.100"));
    }

    #[test]
    fn test_security_alert_template() {
        let mut engine = TemplateEngine::new();
        engine
            .set("user_name", "Jane Doe")
            .set("event_type", "New login from unknown device")
            .set("device_info", "Chrome on Windows")
            .set("location", "New York, US")
            .set("timestamp", "January 31, 2026 at 10:30 AM UTC")
            .set("year", "2026")
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::SecurityAlert);

        assert!(rendered.subject.contains("New login"));
        assert!(rendered.html_body.contains("Chrome on Windows"));
        assert!(rendered.html_body.contains("New York, US"));
    }

    #[test]
    fn test_from_template_type() {
        assert_eq!(
            EmailTemplate::from_template_type(EmailTemplateType::Invitation),
            EmailTemplate::Invitation
        );
        assert_eq!(
            EmailTemplate::from_template_type(EmailTemplateType::PasswordReset),
            EmailTemplate::PasswordReset
        );
        assert_eq!(
            EmailTemplate::from_template_type(EmailTemplateType::EmailMfa),
            EmailTemplate::EmailMfa
        );
    }

    #[test]
    fn test_default_content() {
        let content = EmailTemplate::default_content(EmailTemplateType::Invitation);
        assert!(content.subject.contains("invited"));
        assert!(!content.html_body.is_empty());
        assert!(!content.text_body.is_empty());
    }
}
