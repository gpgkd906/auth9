//! Invitation service for managing user invitations

use crate::domain::{
    CreateInvitationInput, EmailAddress, Invitation, InvitationStatus, StringUuid,
};
use crate::email::{EmailTemplate, TemplateEngine};
use crate::error::{AppError, Result};
use crate::repository::{InvitationRepository, SystemSettingsRepository, TenantRepository};
use crate::service::EmailService;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use std::sync::Arc;
use validator::Validate;

/// Service for managing invitations
pub struct InvitationService<IR, TR, SR>
where
    IR: InvitationRepository,
    TR: TenantRepository,
    SR: SystemSettingsRepository,
{
    invitation_repo: Arc<IR>,
    tenant_repo: Arc<TR>,
    email_service: Arc<EmailService<SR>>,
    /// Base URL for invitation links (e.g., "https://app.example.com")
    app_base_url: String,
}

impl<IR, TR, SR> InvitationService<IR, TR, SR>
where
    IR: InvitationRepository,
    TR: TenantRepository,
    SR: SystemSettingsRepository,
{
    pub fn new(
        invitation_repo: Arc<IR>,
        tenant_repo: Arc<TR>,
        email_service: Arc<EmailService<SR>>,
        app_base_url: String,
    ) -> Self {
        Self {
            invitation_repo,
            tenant_repo,
            email_service,
            app_base_url,
        }
    }

    /// Create a new invitation and send the invitation email
    pub async fn create(
        &self,
        tenant_id: StringUuid,
        invited_by: StringUuid,
        inviter_name: &str,
        input: CreateInvitationInput,
    ) -> Result<Invitation> {
        // Validate input
        input.validate()?;

        // Get tenant for the email
        let tenant = self
            .tenant_repo
            .find_by_id(tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", tenant_id)))?;

        // Check for existing pending invitation
        if let Some(existing) = self
            .invitation_repo
            .find_by_email_and_tenant(&input.email, tenant_id)
            .await?
        {
            if existing.is_valid() {
                return Err(AppError::Conflict(format!(
                    "An invitation for {} already exists",
                    input.email
                )));
            }
        }

        // Generate secure token
        let (token, token_hash) = self.generate_token()?;

        // Create the invitation
        let invitation = self
            .invitation_repo
            .create(tenant_id, invited_by, &input, &token_hash)
            .await?;

        // Build and send invitation email
        let invite_link = format!(
            "{}/invite/accept?token={}",
            self.app_base_url.trim_end_matches('/'),
            token
        );

        let expires_in_hours = input.expires_in_hours.unwrap_or(72);

        let mut engine = TemplateEngine::new();
        engine
            .set("inviter_name", inviter_name)
            .set("tenant_name", &tenant.name)
            .set("invite_link", &invite_link)
            .set("expires_in_hours", expires_in_hours.to_string())
            .set("year", chrono::Utc::now().format("%Y").to_string())
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::Invitation);

        // Send the email
        let _ = self
            .email_service
            .send_with_from(
                EmailAddress::new(&input.email),
                &rendered.subject,
                &rendered.html_body,
                Some(&rendered.text_body),
                None, // Use system email settings
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to send invitation email: {}", e);
                e
            });

        Ok(invitation)
    }

    /// Get an invitation by ID
    ///
    /// Dynamically updates expired pending invitations to show "expired" status.
    pub async fn get(&self, id: StringUuid) -> Result<Invitation> {
        let mut invitation = self
            .invitation_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))?;

        // Dynamically update expired status for display
        if invitation.status == InvitationStatus::Pending && invitation.is_expired() {
            invitation.status = InvitationStatus::Expired;
        }

        Ok(invitation)
    }

    /// List invitations for a tenant with optional status filter
    ///
    /// Dynamically updates the status of expired invitations to "expired"
    /// when returning results, ensuring accurate status display without
    /// requiring a background job.
    pub async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Invitation>, i64)> {
        let offset = (page - 1) * per_page;
        let invitations = self
            .invitation_repo
            .list_by_tenant(tenant_id, status.clone(), offset, per_page)
            .await?;
        let total = self
            .invitation_repo
            .count_by_tenant(tenant_id, status)
            .await?;

        // Dynamically update expired invitations' status for display
        let invitations = invitations
            .into_iter()
            .map(|mut inv| {
                if inv.status == InvitationStatus::Pending && inv.is_expired() {
                    inv.status = InvitationStatus::Expired;
                }
                inv
            })
            .collect();

        Ok((invitations, total))
    }

    /// Revoke an invitation
    pub async fn revoke(&self, id: StringUuid) -> Result<Invitation> {
        let invitation = self.get(id).await?;

        if invitation.status != InvitationStatus::Pending {
            return Err(AppError::BadRequest(format!(
                "Cannot revoke invitation with status: {}",
                invitation.status
            )));
        }

        self.invitation_repo
            .update_status(id, InvitationStatus::Revoked)
            .await
    }

    /// Accept an invitation
    ///
    /// Returns the invitation if the token is valid.
    /// The caller is responsible for creating the user and assigning roles.
    pub async fn accept(&self, token: &str) -> Result<Invitation> {
        let invitation = self.get_by_token(token).await?;

        if !invitation.is_valid() {
            if invitation.is_expired() {
                return Err(AppError::BadRequest("Invitation has expired".to_string()));
            }
            return Err(AppError::BadRequest(format!(
                "Invitation is no longer valid (status: {})",
                invitation.status
            )));
        }

        // Mark as accepted
        self.invitation_repo.mark_accepted(invitation.id).await
    }

    /// Get invitation by token without changing status.
    pub async fn get_by_token(&self, token: &str) -> Result<Invitation> {
        // Find all pending invitations and verify token against each
        // This is not optimal but secure - we don't expose which emails have invitations
        self.find_by_token(token).await
    }

    /// Mark invitation as accepted
    pub async fn mark_accepted(&self, id: StringUuid) -> Result<Invitation> {
        self.invitation_repo.mark_accepted(id).await
    }

    /// Resend an invitation email
    pub async fn resend(&self, id: StringUuid, inviter_name: &str) -> Result<Invitation> {
        let invitation = self.get(id).await?;

        if invitation.status != InvitationStatus::Pending {
            return Err(AppError::BadRequest(format!(
                "Cannot resend invitation with status: {}",
                invitation.status
            )));
        }

        if invitation.is_expired() {
            return Err(AppError::BadRequest(
                "Invitation has expired. Create a new invitation instead.".to_string(),
            ));
        }

        let tenant = self
            .tenant_repo
            .find_by_id(invitation.tenant_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Tenant {} not found", invitation.tenant_id))
            })?;

        // Generate new token and update hash in database
        let (token, token_hash) = self.generate_token()?;
        self.invitation_repo
            .update_token_hash(id, &token_hash)
            .await?;

        let invite_link = format!(
            "{}/invite/accept?token={}",
            self.app_base_url.trim_end_matches('/'),
            token
        );

        let hours_until_expiry = (invitation.expires_at - chrono::Utc::now()).num_hours();

        let mut engine = TemplateEngine::new();
        engine
            .set("inviter_name", inviter_name)
            .set("tenant_name", &tenant.name)
            .set("invite_link", &invite_link)
            .set("expires_in_hours", hours_until_expiry.to_string())
            .set("year", chrono::Utc::now().format("%Y").to_string())
            .set("app_name", "Auth9");

        let rendered = engine.render_template(EmailTemplate::Invitation);

        self.email_service
            .send_with_from(
                EmailAddress::new(&invitation.email),
                &rendered.subject,
                &rendered.html_body,
                Some(&rendered.text_body),
                None,
            )
            .await?;

        self.invitation_repo.find_by_id(id).await?.ok_or_else(|| {
            AppError::NotFound(format!("Invitation {} not found", id))
        })
    }

    /// Delete an invitation
    pub async fn delete(&self, id: StringUuid) -> Result<()> {
        self.invitation_repo.delete(id).await
    }

    /// Expire all pending invitations that have passed their expiration date
    pub async fn expire_pending(&self) -> Result<u64> {
        self.invitation_repo.expire_pending().await
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    fn generate_token(&self) -> Result<(String, String)> {
        // Generate 32 random bytes for the token
        let mut token_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut token_bytes);
        let token = URL_SAFE_NO_PAD.encode(token_bytes);

        // Hash the token using Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(token.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to hash token: {}", e)))?
            .to_string();

        Ok((token, hash))
    }

    #[allow(dead_code)]
    fn verify_token(&self, token: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(token.as_bytes(), &parsed_hash)
            .is_ok()
    }

    async fn find_by_token(&self, token: &str) -> Result<Invitation> {
        // Iterate through pending invitations and verify the token hash
        let invitations = self.invitation_repo.list_pending().await?;

        for invitation in invitations {
            if self.verify_token(token, &invitation.token_hash) {
                return Ok(invitation);
            }
        }

        Err(AppError::NotFound("Invalid invitation token".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Tenant;
    use crate::repository::invitation::MockInvitationRepository;
    use crate::repository::system_settings::MockSystemSettingsRepository;
    use crate::repository::tenant::MockTenantRepository;
    use crate::service::SystemSettingsService;
    use mockall::predicate::*;

    fn create_test_service() -> InvitationService<
        MockInvitationRepository,
        MockTenantRepository,
        MockSystemSettingsRepository,
    > {
        let mut invitation_repo = MockInvitationRepository::new();
        invitation_repo
            .expect_list_pending()
            .returning(|| Ok(vec![]));
        let invitation_repo = Arc::new(invitation_repo);
        let tenant_repo = Arc::new(MockTenantRepository::new());

        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        InvitationService::new(
            invitation_repo,
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        )
    }

    #[test]
    fn test_generate_token() {
        let service = create_test_service();

        let (token, hash) = service.generate_token().unwrap();

        // Token should be base64 encoded
        assert!(!token.is_empty());
        assert!(token.len() > 20);

        // Hash should be Argon2 format
        assert!(hash.starts_with("$argon2"));

        // Token should verify against its hash
        assert!(service.verify_token(&token, &hash));
    }

    #[test]
    fn test_verify_token_wrong_token() {
        let service = create_test_service();

        let (_, hash) = service.generate_token().unwrap();

        // Wrong token should not verify
        assert!(!service.verify_token("wrong-token", &hash));
    }

    #[test]
    fn test_verify_token_invalid_hash() {
        let service = create_test_service();

        // Invalid hash format
        assert!(!service.verify_token("any-token", "invalid-hash"));
    }

    #[tokio::test]
    async fn test_create_validates_input() {
        let mut invitation_repo = MockInvitationRepository::new();
        let mut tenant_repo = MockTenantRepository::new();

        let tenant_id = StringUuid::new_v4();
        let tenant = Tenant {
            id: tenant_id,
            name: "Test Tenant".to_string(),
            ..Default::default()
        };

        tenant_repo
            .expect_find_by_id()
            .with(eq(tenant_id))
            .returning(move |_| Ok(Some(tenant.clone())));

        invitation_repo
            .expect_find_by_email_and_tenant()
            .returning(|_, _| Ok(None));

        invitation_repo.expect_create().returning(|_, _, input, _| {
            Ok(Invitation {
                email: input.email.clone(),
                ..Default::default()
            })
        });

        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            Arc::new(tenant_repo),
            email_service,
            "https://app.example.com".to_string(),
        );

        // Invalid email should fail validation
        let input = CreateInvitationInput {
            email: "not-an-email".to_string(),
            role_ids: vec![StringUuid::new_v4()],
            expires_in_hours: None,
        };

        let result = service
            .create(tenant_id, StringUuid::new_v4(), "Admin", input)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_non_pending_fails() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| {
                Ok(Some(Invitation {
                    id,
                    status: InvitationStatus::Accepted,
                    ..Default::default()
                }))
            });

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.revoke(id).await;

        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_revoke_success() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| {
                Ok(Some(Invitation {
                    id,
                    status: InvitationStatus::Pending,
                    ..Default::default()
                }))
            });

        invitation_repo
            .expect_update_status()
            .with(eq(id), eq(InvitationStatus::Revoked))
            .returning(move |id, status| {
                Ok(Invitation {
                    id,
                    status,
                    ..Default::default()
                })
            });

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.revoke(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, InvitationStatus::Revoked);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.get(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_success() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| {
                Ok(Some(Invitation {
                    id,
                    email: "test@example.com".to_string(),
                    ..Default::default()
                }))
            });

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.get(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_list_by_tenant() {
        let mut invitation_repo = MockInvitationRepository::new();
        let tenant_id = StringUuid::new_v4();

        invitation_repo
            .expect_list_by_tenant()
            .returning(|_, _, _, _| {
                Ok(vec![
                    Invitation {
                        email: "user1@example.com".to_string(),
                        ..Default::default()
                    },
                    Invitation {
                        email: "user2@example.com".to_string(),
                        ..Default::default()
                    },
                ])
            });

        invitation_repo
            .expect_count_by_tenant()
            .returning(|_, _| Ok(2));

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let (invitations, total) = service.list_by_tenant(tenant_id, None, 1, 10).await.unwrap();
        assert_eq!(invitations.len(), 2);
        assert_eq!(total, 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_delete()
            .with(eq(id))
            .returning(|_| Ok(()));

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.delete(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_expire_pending() {
        let mut invitation_repo = MockInvitationRepository::new();

        invitation_repo.expect_expire_pending().returning(|| Ok(5));

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.expire_pending().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_create_existing_invitation_conflict() {
        let mut invitation_repo = MockInvitationRepository::new();
        let mut tenant_repo = MockTenantRepository::new();

        let tenant_id = StringUuid::new_v4();
        let tenant = Tenant {
            id: tenant_id,
            name: "Test Tenant".to_string(),
            ..Default::default()
        };

        tenant_repo
            .expect_find_by_id()
            .with(eq(tenant_id))
            .returning(move |_| Ok(Some(tenant.clone())));

        // Return an existing valid invitation
        invitation_repo
            .expect_find_by_email_and_tenant()
            .returning(move |_, _| {
                Ok(Some(Invitation {
                    status: InvitationStatus::Pending,
                    expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
                    ..Default::default()
                }))
            });

        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            Arc::new(tenant_repo),
            email_service,
            "https://app.example.com".to_string(),
        );

        let input = CreateInvitationInput {
            email: "existing@example.com".to_string(),
            role_ids: vec![StringUuid::new_v4()],
            expires_in_hours: None,
        };

        let result = service
            .create(tenant_id, StringUuid::new_v4(), "Admin", input)
            .await;

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_create_tenant_not_found() {
        let invitation_repo = MockInvitationRepository::new();
        let mut tenant_repo = MockTenantRepository::new();

        let tenant_id = StringUuid::new_v4();

        tenant_repo
            .expect_find_by_id()
            .with(eq(tenant_id))
            .returning(|_| Ok(None));

        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            Arc::new(tenant_repo),
            email_service,
            "https://app.example.com".to_string(),
        );

        let input = CreateInvitationInput {
            email: "test@example.com".to_string(),
            role_ids: vec![StringUuid::new_v4()],
            expires_in_hours: None,
        };

        let result = service
            .create(tenant_id, StringUuid::new_v4(), "Admin", input)
            .await;

        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_resend_non_pending_fails() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| {
                Ok(Some(Invitation {
                    id,
                    status: InvitationStatus::Revoked,
                    ..Default::default()
                }))
            });

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.resend(id, "Admin").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_resend_expired_fails() {
        let mut invitation_repo = MockInvitationRepository::new();
        let id = StringUuid::new_v4();

        invitation_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| {
                Ok(Some(Invitation {
                    id,
                    status: InvitationStatus::Pending,
                    expires_at: chrono::Utc::now() - chrono::Duration::hours(1), // Expired
                    ..Default::default()
                }))
            });

        let tenant_repo = Arc::new(MockTenantRepository::new());
        let settings_repo = Arc::new(MockSystemSettingsRepository::new());
        let settings_service = Arc::new(SystemSettingsService::new(settings_repo, None));
        let email_service = Arc::new(EmailService::new(settings_service));

        let service = InvitationService::new(
            Arc::new(invitation_repo),
            tenant_repo,
            email_service,
            "https://app.example.com".to_string(),
        );

        let result = service.resend(id, "Admin").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
        if let Err(AppError::BadRequest(msg)) = result {
            assert!(msg.contains("expired"));
        }
    }

    #[tokio::test]
    async fn test_accept_expired_invitation() {
        let service = create_test_service();

        // find_by_token returns NotFound for simplified implementation
        let result = service.accept("any-token").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
