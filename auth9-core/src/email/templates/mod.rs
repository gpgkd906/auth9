//! Email template system
//!
//! Provides simple variable substitution for email templates.
//! Variables are specified using {{variable_name}} syntax.

use std::collections::HashMap;

/// Available email templates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailTemplate {
    /// User invitation email
    Invitation,
    /// Password reset email
    PasswordReset,
}

impl EmailTemplate {
    /// Get the subject line for this template
    pub fn subject(&self) -> &'static str {
        match self {
            Self::Invitation => "You've been invited to join {{tenant_name}}",
            Self::PasswordReset => "Reset your password",
        }
    }

    /// Get the HTML body template
    pub fn html_body(&self) -> &'static str {
        match self {
            Self::Invitation => INVITATION_TEMPLATE,
            Self::PasswordReset => PASSWORD_RESET_TEMPLATE,
        }
    }

    /// Get the plain text body template
    pub fn text_body(&self) -> &'static str {
        match self {
            Self::Invitation => INVITATION_TEMPLATE_TEXT,
            Self::PasswordReset => PASSWORD_RESET_TEMPLATE_TEXT,
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
        engine.set_all([
            ("a", "1"),
            ("b", "2"),
        ]);

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
        assert!(rendered.html_body.contains("https://example.com/invite/abc123"));
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
}
