//! Email template service
//!
//! Manages customizable email templates stored in system_settings.

use crate::domain::{
    EmailTemplateContent, EmailTemplateMetadata, EmailTemplateType, EmailTemplateWithContent,
    RenderedEmailPreview, SystemSettingRow, UpsertSystemSettingInput,
};
use crate::email::templates::{EmailTemplate, TemplateEngine};
use crate::error::{AppError, Result};
use crate::repository::SystemSettingsRepository;
use std::sync::Arc;

/// Category for email templates in system_settings
const EMAIL_TEMPLATES_CATEGORY: &str = "email_templates";

/// Service for managing email templates
pub struct EmailTemplateService<R: SystemSettingsRepository> {
    repo: Arc<R>,
}

impl<R: SystemSettingsRepository> EmailTemplateService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    /// List all templates with their content (customized or default)
    pub async fn list_templates(&self) -> Result<Vec<EmailTemplateWithContent>> {
        // Get all custom templates from database
        let custom_templates = self.repo.list_by_category(EMAIL_TEMPLATES_CATEGORY).await?;

        // Build a map of custom templates by key
        let custom_map: std::collections::HashMap<String, SystemSettingRow> = custom_templates
            .into_iter()
            .map(|row| (row.setting_key.clone(), row))
            .collect();

        // Build result with all template types
        let mut templates = Vec::new();
        for template_type in EmailTemplateType::all() {
            let template = self.build_template_with_content(*template_type, custom_map.get(template_type.as_str()));
            templates.push(template);
        }

        Ok(templates)
    }

    /// Get a specific template (returns default if not customized)
    pub async fn get_template(
        &self,
        template_type: EmailTemplateType,
    ) -> Result<EmailTemplateWithContent> {
        let custom = self
            .repo
            .get(EMAIL_TEMPLATES_CATEGORY, template_type.as_str())
            .await?;

        Ok(self.build_template_with_content(template_type, custom.as_ref()))
    }

    /// Update a template with custom content
    pub async fn update_template(
        &self,
        template_type: EmailTemplateType,
        content: EmailTemplateContent,
    ) -> Result<EmailTemplateWithContent> {
        // Validate content
        self.validate_content(&content)?;

        // Store in database
        let value = serde_json::to_value(&content)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to serialize content: {}", e)))?;

        let input = UpsertSystemSettingInput {
            category: EMAIL_TEMPLATES_CATEGORY.to_string(),
            setting_key: template_type.as_str().to_string(),
            value,
            encrypted: false,
            description: Some(format!("Custom {} email template", template_type.display_name())),
        };

        let row = self.repo.upsert(&input).await?;

        Ok(self.build_template_with_content(template_type, Some(&row)))
    }

    /// Reset a template to default (delete custom version)
    pub async fn reset_template(
        &self,
        template_type: EmailTemplateType,
    ) -> Result<EmailTemplateWithContent> {
        // Delete custom template from database
        self.repo
            .delete(EMAIL_TEMPLATES_CATEGORY, template_type.as_str())
            .await?;

        // Return default template
        Ok(self.build_template_with_content(template_type, None))
    }

    /// Preview a template with sample data
    pub async fn preview_template(
        &self,
        template_type: EmailTemplateType,
        content: &EmailTemplateContent,
    ) -> Result<RenderedEmailPreview> {
        let mut engine = TemplateEngine::new();

        // Set example values for all variables
        for var in template_type.variables() {
            engine.set(&var.name, &var.example);
        }

        Ok(RenderedEmailPreview {
            subject: engine.render(&content.subject),
            html_body: engine.render(&content.html_body),
            text_body: engine.render(&content.text_body),
        })
    }

    /// Render a template with custom variable values
    ///
    /// First fills in all variables with example values, then overrides with custom values.
    /// This ensures all placeholders are replaced even if not all custom values are provided.
    pub fn render_template_with_variables(
        &self,
        template_type: EmailTemplateType,
        content: &EmailTemplateContent,
        custom_variables: &std::collections::HashMap<String, String>,
    ) -> RenderedEmailPreview {
        let mut engine = TemplateEngine::new();

        // First, set example values for all variables as defaults
        for var in template_type.variables() {
            engine.set(&var.name, &var.example);
        }

        // Then override with custom values
        for (key, value) in custom_variables {
            engine.set(key, value);
        }

        RenderedEmailPreview {
            subject: engine.render(&content.subject),
            html_body: engine.render(&content.html_body),
            text_body: engine.render(&content.text_body),
        }
    }

    /// Get the content for a template (used by email sending code)
    /// Returns custom content if available, otherwise default content
    pub async fn get_content(&self, template_type: EmailTemplateType) -> Result<EmailTemplateContent> {
        let custom = self
            .repo
            .get(EMAIL_TEMPLATES_CATEGORY, template_type.as_str())
            .await?;

        match custom {
            Some(row) => self.parse_content(&row.value),
            None => Ok(EmailTemplate::default_content(template_type)),
        }
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    fn build_template_with_content(
        &self,
        template_type: EmailTemplateType,
        custom: Option<&SystemSettingRow>,
    ) -> EmailTemplateWithContent {
        let metadata = EmailTemplateMetadata::from_type(template_type);

        match custom {
            Some(row) => {
                // Try to parse custom content, fall back to default on error
                let content = self
                    .parse_content(&row.value)
                    .unwrap_or_else(|_| EmailTemplate::default_content(template_type));

                EmailTemplateWithContent {
                    metadata,
                    content,
                    is_customized: true,
                    updated_at: Some(row.updated_at),
                }
            }
            None => {
                // Return default template
                EmailTemplateWithContent {
                    metadata,
                    content: EmailTemplate::default_content(template_type),
                    is_customized: false,
                    updated_at: None,
                }
            }
        }
    }

    fn parse_content(&self, value: &serde_json::Value) -> Result<EmailTemplateContent> {
        serde_json::from_value(value.clone())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse template content: {}", e)))
    }

    fn validate_content(&self, content: &EmailTemplateContent) -> Result<()> {
        if content.subject.trim().is_empty() {
            return Err(AppError::Validation("Subject cannot be empty".to_string()));
        }
        if content.html_body.trim().is_empty() {
            return Err(AppError::Validation("HTML body cannot be empty".to_string()));
        }
        if content.text_body.trim().is_empty() {
            return Err(AppError::Validation("Text body cannot be empty".to_string()));
        }
        // Max length checks
        if content.subject.len() > 500 {
            return Err(AppError::Validation(
                "Subject exceeds maximum length of 500 characters".to_string(),
            ));
        }
        if content.html_body.len() > 100_000 {
            return Err(AppError::Validation(
                "HTML body exceeds maximum length of 100,000 characters".to_string(),
            ));
        }
        if content.text_body.len() > 50_000 {
            return Err(AppError::Validation(
                "Text body exceeds maximum length of 50,000 characters".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_list_templates_all_default() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_list_by_category()
            .with(eq(EMAIL_TEMPLATES_CATEGORY))
            .returning(|_| Ok(vec![]));

        let service = EmailTemplateService::new(Arc::new(mock));
        let templates = service.list_templates().await.unwrap();

        assert_eq!(templates.len(), 7);
        assert!(templates.iter().all(|t| !t.is_customized));
    }

    #[tokio::test]
    async fn test_list_templates_with_custom() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_list_by_category()
            .with(eq(EMAIL_TEMPLATES_CATEGORY))
            .returning(|_| {
                Ok(vec![SystemSettingRow {
                    id: 1,
                    category: EMAIL_TEMPLATES_CATEGORY.to_string(),
                    setting_key: "invitation".to_string(),
                    value: serde_json::json!({
                        "subject": "Custom Subject",
                        "html_body": "<p>Custom</p>",
                        "text_body": "Custom"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }])
            });

        let service = EmailTemplateService::new(Arc::new(mock));
        let templates = service.list_templates().await.unwrap();

        let invitation = templates
            .iter()
            .find(|t| t.metadata.template_type == EmailTemplateType::Invitation)
            .unwrap();

        assert!(invitation.is_customized);
        assert_eq!(invitation.content.subject, "Custom Subject");

        // Other templates should be default
        let password_reset = templates
            .iter()
            .find(|t| t.metadata.template_type == EmailTemplateType::PasswordReset)
            .unwrap();
        assert!(!password_reset.is_customized);
    }

    #[tokio::test]
    async fn test_get_template_default() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq(EMAIL_TEMPLATES_CATEGORY), eq("invitation"))
            .returning(|_, _| Ok(None));

        let service = EmailTemplateService::new(Arc::new(mock));
        let template = service
            .get_template(EmailTemplateType::Invitation)
            .await
            .unwrap();

        assert!(!template.is_customized);
        assert!(template.content.subject.contains("invited"));
    }

    #[tokio::test]
    async fn test_get_template_custom() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq(EMAIL_TEMPLATES_CATEGORY), eq("invitation"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: EMAIL_TEMPLATES_CATEGORY.to_string(),
                    setting_key: "invitation".to_string(),
                    value: serde_json::json!({
                        "subject": "Join Us!",
                        "html_body": "<h1>Welcome</h1>",
                        "text_body": "Welcome"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = EmailTemplateService::new(Arc::new(mock));
        let template = service
            .get_template(EmailTemplateType::Invitation)
            .await
            .unwrap();

        assert!(template.is_customized);
        assert_eq!(template.content.subject, "Join Us!");
    }

    #[tokio::test]
    async fn test_update_template() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_upsert().returning(|input| {
            Ok(SystemSettingRow {
                id: 1,
                category: input.category.clone(),
                setting_key: input.setting_key.clone(),
                value: input.value.clone(),
                encrypted: false,
                description: input.description.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        });

        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Custom Subject".to_string(),
            html_body: "<p>Custom HTML</p>".to_string(),
            text_body: "Custom Text".to_string(),
        };

        let result = service
            .update_template(EmailTemplateType::Invitation, content)
            .await
            .unwrap();

        assert!(result.is_customized);
        assert_eq!(result.content.subject, "Custom Subject");
    }

    #[tokio::test]
    async fn test_update_template_validation_empty_subject() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "   ".to_string(),
            html_body: "<p>HTML</p>".to_string(),
            text_body: "Text".to_string(),
        };

        let result = service
            .update_template(EmailTemplateType::Invitation, content)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_reset_template() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_delete()
            .with(eq(EMAIL_TEMPLATES_CATEGORY), eq("invitation"))
            .returning(|_, _| Ok(()));

        let service = EmailTemplateService::new(Arc::new(mock));
        let result = service
            .reset_template(EmailTemplateType::Invitation)
            .await
            .unwrap();

        assert!(!result.is_customized);
        assert!(result.content.subject.contains("invited"));
    }

    #[tokio::test]
    async fn test_preview_template() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Welcome {{user_name}}!".to_string(),
            html_body: "<h1>Hello {{user_name}}</h1>".to_string(),
            text_body: "Hello {{user_name}}".to_string(),
        };

        let preview = service
            .preview_template(EmailTemplateType::Welcome, &content)
            .await
            .unwrap();

        // Should use example values from template type
        assert!(preview.subject.contains("Jane Smith"));
        assert!(preview.html_body.contains("Jane Smith"));
    }

    #[tokio::test]
    async fn test_get_content_default() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq(EMAIL_TEMPLATES_CATEGORY), eq("invitation"))
            .returning(|_, _| Ok(None));

        let service = EmailTemplateService::new(Arc::new(mock));
        let content = service
            .get_content(EmailTemplateType::Invitation)
            .await
            .unwrap();

        assert!(content.subject.contains("invited"));
    }

    #[tokio::test]
    async fn test_get_content_custom() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq(EMAIL_TEMPLATES_CATEGORY), eq("invitation"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: EMAIL_TEMPLATES_CATEGORY.to_string(),
                    setting_key: "invitation".to_string(),
                    value: serde_json::json!({
                        "subject": "Custom",
                        "html_body": "<p>Custom</p>",
                        "text_body": "Custom"
                    }),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let service = EmailTemplateService::new(Arc::new(mock));
        let content = service
            .get_content(EmailTemplateType::Invitation)
            .await
            .unwrap();

        assert_eq!(content.subject, "Custom");
    }

    #[test]
    fn test_render_template_with_variables_custom_overrides() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Welcome {{user_name}} to {{tenant_name}}!".to_string(),
            html_body: "<h1>Hello {{user_name}}</h1><p>Welcome to {{tenant_name}}</p>".to_string(),
            text_body: "Hello {{user_name}}, Welcome to {{tenant_name}}".to_string(),
        };

        let mut custom_vars = std::collections::HashMap::new();
        custom_vars.insert("user_name".to_string(), "Alice".to_string());
        custom_vars.insert("tenant_name".to_string(), "My Company".to_string());

        let preview = service.render_template_with_variables(
            EmailTemplateType::Welcome,
            &content,
            &custom_vars,
        );

        assert!(preview.subject.contains("Alice"));
        assert!(preview.subject.contains("My Company"));
        assert!(preview.html_body.contains("Alice"));
        assert!(preview.text_body.contains("My Company"));
    }

    #[test]
    fn test_render_template_with_variables_falls_back_to_examples() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "{{inviter_name}} invited you to {{tenant_name}}".to_string(),
            html_body: "<p>{{inviter_name}} invited you</p>".to_string(),
            text_body: "{{inviter_name}} invited you".to_string(),
        };

        // Only provide partial custom variables
        let mut custom_vars = std::collections::HashMap::new();
        custom_vars.insert("inviter_name".to_string(), "Bob".to_string());

        let preview = service.render_template_with_variables(
            EmailTemplateType::Invitation,
            &content,
            &custom_vars,
        );

        // inviter_name should use custom value
        assert!(preview.subject.contains("Bob"));
        // tenant_name should fall back to example value from template type ("Acme Corp")
        assert!(preview.subject.contains("Acme Corp"));
    }

    #[test]
    fn test_render_template_with_empty_custom_variables() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Welcome {{user_name}}!".to_string(),
            html_body: "<h1>Hello {{user_name}}</h1>".to_string(),
            text_body: "Hello {{user_name}}".to_string(),
        };

        let custom_vars = std::collections::HashMap::new();

        let preview = service.render_template_with_variables(
            EmailTemplateType::Welcome,
            &content,
            &custom_vars,
        );

        // Should use example values
        assert!(preview.subject.contains("Jane Smith"));
    }

    #[test]
    fn test_validate_content_success() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Valid Subject".to_string(),
            html_body: "<p>Valid HTML</p>".to_string(),
            text_body: "Valid Text".to_string(),
        };

        assert!(service.validate_content(&content).is_ok());
    }

    #[test]
    fn test_validate_content_empty_html() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Subject".to_string(),
            html_body: "".to_string(),
            text_body: "Text".to_string(),
        };

        assert!(service.validate_content(&content).is_err());
    }

    #[test]
    fn test_validate_content_subject_too_long() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "x".repeat(501),
            html_body: "<p>HTML</p>".to_string(),
            text_body: "Text".to_string(),
        };

        assert!(service.validate_content(&content).is_err());
    }

    #[tokio::test]
    async fn test_list_templates_metadata() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_list_by_category()
            .with(eq(EMAIL_TEMPLATES_CATEGORY))
            .returning(|_| Ok(vec![]));

        let service = EmailTemplateService::new(Arc::new(mock));
        let templates = service.list_templates().await.unwrap();

        // Check that all templates have metadata
        for template in &templates {
            assert!(!template.metadata.name.is_empty());
            assert!(!template.metadata.description.is_empty());
            assert!(!template.metadata.variables.is_empty());
        }
    }

    #[tokio::test]
    async fn test_preview_template_invitation() {
        let mock = MockSystemSettingsRepository::new();
        let service = EmailTemplateService::new(Arc::new(mock));

        let content = EmailTemplateContent {
            subject: "Join {{tenant_name}}".to_string(),
            html_body: "<p>{{inviter_name}} invited you</p>".to_string(),
            text_body: "{{inviter_name}} invited you".to_string(),
        };

        let preview = service
            .preview_template(EmailTemplateType::Invitation, &content)
            .await
            .unwrap();

        assert!(preview.subject.contains("Acme Corp"));
        assert!(preview.html_body.contains("John Doe"));
    }
}
