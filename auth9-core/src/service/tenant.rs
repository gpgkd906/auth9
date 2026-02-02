//! Tenant business logic

use crate::cache::CacheManager;
use crate::domain::{CreateTenantInput, StringUuid, Tenant, TenantStatus, UpdateTenantInput};
use crate::error::{AppError, Result};
use crate::repository::{
    InvitationRepository, LoginEventRepository, RbacRepository, SecurityAlertRepository,
    ServiceRepository, TenantRepository, UserRepository, WebhookRepository,
};
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;
use validator::Validate;

pub struct TenantService<
    R: TenantRepository,
    SR: ServiceRepository,
    WR: WebhookRepository,
    IR: InvitationRepository,
    UR: UserRepository,
    RR: RbacRepository,
    LR: LoginEventRepository,
    SAR: SecurityAlertRepository,
> {
    repo: Arc<R>,
    service_repo: Arc<SR>,
    webhook_repo: Arc<WR>,
    invitation_repo: Arc<IR>,
    user_repo: Arc<UR>,
    rbac_repo: Arc<RR>,
    login_event_repo: Arc<LR>,
    security_alert_repo: Arc<SAR>,
    cache_manager: Option<CacheManager>,
}

impl<
        R: TenantRepository,
        SR: ServiceRepository,
        WR: WebhookRepository,
        IR: InvitationRepository,
        UR: UserRepository,
        RR: RbacRepository,
        LR: LoginEventRepository,
        SAR: SecurityAlertRepository,
    > TenantService<R, SR, WR, IR, UR, RR, LR, SAR>
{
    pub fn new(
        repo: Arc<R>,
        service_repo: Arc<SR>,
        webhook_repo: Arc<WR>,
        invitation_repo: Arc<IR>,
        user_repo: Arc<UR>,
        rbac_repo: Arc<RR>,
        login_event_repo: Arc<LR>,
        security_alert_repo: Arc<SAR>,
        cache_manager: Option<CacheManager>,
    ) -> Self {
        Self {
            repo,
            service_repo,
            webhook_repo,
            invitation_repo,
            user_repo,
            rbac_repo,
            login_event_repo,
            security_alert_repo,
            cache_manager,
        }
    }

    pub async fn create(&self, input: CreateTenantInput) -> Result<Tenant> {
        // Validate input
        input.validate()?;

        // Check for duplicate slug
        if self.repo.find_by_slug(&input.slug).await?.is_some() {
            return Err(AppError::Conflict(format!(
                "Tenant with slug '{}' already exists",
                input.slug
            )));
        }

        let tenant = self.repo.create(&input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache
                .set_tenant_config(Uuid::from(tenant.id), &tenant)
                .await;
        }
        Ok(tenant)
    }

    pub async fn get(&self, id: StringUuid) -> Result<Tenant> {
        if let Some(cache) = &self.cache_manager {
            if let Ok(Some(tenant)) = cache.get_tenant_config(Uuid::from(id)).await {
                return Ok(tenant);
            }
        }
        let tenant = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", id)))?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache
                .set_tenant_config(Uuid::from(tenant.id), &tenant)
                .await;
        }
        Ok(tenant)
    }

    pub async fn get_by_slug(&self, slug: &str) -> Result<Tenant> {
        let tenant = self
            .repo
            .find_by_slug(slug)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Tenant '{}' not found", slug)))?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache
                .set_tenant_config(Uuid::from(tenant.id), &tenant)
                .await;
        }
        Ok(tenant)
    }

    pub async fn list(&self, page: i64, per_page: i64) -> Result<(Vec<Tenant>, i64)> {
        let offset = (page - 1) * per_page;
        let tenants = self.repo.list(offset, per_page).await?;
        let total = self.repo.count().await?;
        Ok((tenants, total))
    }

    pub async fn search(
        &self,
        query: &str,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Tenant>, i64)> {
        let offset = (page - 1) * per_page;
        let tenants = self.repo.search(query, offset, per_page).await?;
        let total = self.repo.count_search(query).await?;
        Ok((tenants, total))
    }

    pub async fn update(&self, id: StringUuid, input: UpdateTenantInput) -> Result<Tenant> {
        input.validate()?;

        // Verify tenant exists
        let _ = self.get(id).await?;

        let tenant = self.repo.update(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_tenant_config(Uuid::from(id)).await;
        }
        Ok(tenant)
    }

    pub async fn delete(&self, id: StringUuid) -> Result<()> {
        // Verify tenant exists
        let _ = self.get(id).await?;

        // CASCADE DELETE:
        // 1. Get all services for this tenant and delete them (includes RBAC cascade)
        let services = self.service_repo.list_by_tenant(Uuid::from(id)).await?;
        for service in services {
            // Delete all clients for this service
            let deleted_clients = self
                .service_repo
                .delete_clients_by_service(service.id.0)
                .await?;
            warn!(
                service_id = %service.id,
                deleted_clients = deleted_clients,
                "Deleted clients for service"
            );

            // Clear parent_role_id references before deleting roles
            let cleared_refs = self
                .rbac_repo
                .clear_parent_role_references(service.id)
                .await?;
            warn!(
                service_id = %service.id,
                cleared_refs = cleared_refs,
                "Cleared parent role references"
            );

            // Delete role_permissions, user_tenant_roles, and roles for this service
            let deleted_roles = self.rbac_repo.delete_roles_by_service(service.id).await?;
            warn!(
                service_id = %service.id,
                deleted_roles = deleted_roles,
                "Deleted roles for service"
            );

            // Delete permissions for this service
            let deleted_perms = self
                .rbac_repo
                .delete_permissions_by_service(service.id)
                .await?;
            warn!(
                service_id = %service.id,
                deleted_perms = deleted_perms,
                "Deleted permissions for service"
            );

            // Delete the service itself
            self.service_repo.delete(service.id.0).await?;

            // Invalidate service cache
            if let Some(cache) = &self.cache_manager {
                let _ = cache.invalidate_service_config(service.id.0).await;
            }
        }

        // 2. Delete webhooks
        let deleted_webhooks = self.webhook_repo.delete_by_tenant(id).await?;
        warn!(tenant_id = %id, deleted_webhooks = deleted_webhooks, "Deleted webhooks");

        // 3. Delete invitations
        let deleted_invitations = self.invitation_repo.delete_by_tenant(id).await?;
        warn!(tenant_id = %id, deleted_invitations = deleted_invitations, "Deleted invitations");

        // 4. Get all tenant_users and delete their user_tenant_roles
        let tenant_user_ids = self.user_repo.list_tenant_user_ids_by_tenant(id).await?;
        for tenant_user_id in &tenant_user_ids {
            let deleted_roles = self
                .rbac_repo
                .delete_user_roles_by_tenant_user(*tenant_user_id)
                .await?;
            warn!(
                tenant_user_id = %tenant_user_id,
                deleted_roles = deleted_roles,
                "Deleted user tenant roles"
            );
        }

        // 5. Delete tenant_users (membership records, not the users themselves)
        let deleted_memberships = self
            .user_repo
            .delete_tenant_memberships_by_tenant(id)
            .await?;
        warn!(tenant_id = %id, deleted_memberships = deleted_memberships, "Deleted tenant memberships");

        // 6. Delete login_events for this tenant
        let deleted_login_events = self.login_event_repo.delete_by_tenant(id).await?;
        warn!(tenant_id = %id, deleted_login_events = deleted_login_events, "Deleted login events");

        // 7. Delete security_alerts for this tenant
        let deleted_alerts = self.security_alert_repo.delete_by_tenant(id).await?;
        warn!(tenant_id = %id, deleted_alerts = deleted_alerts, "Deleted security alerts");

        // 8. Delete the tenant itself
        self.repo.delete(id).await?;

        // 9. Clear cache
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_tenant_config(Uuid::from(id)).await;
        }

        Ok(())
    }

    pub async fn disable(&self, id: StringUuid) -> Result<Tenant> {
        let _ = self.get(id).await?;
        let input = UpdateTenantInput {
            name: None,
            logo_url: None,
            settings: None,
            status: Some(TenantStatus::Inactive),
        };
        let tenant = self.repo.update(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_tenant_config(Uuid::from(id)).await;
        }
        Ok(tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{StringUuid, TenantSettings};
    use crate::repository::invitation::MockInvitationRepository;
    use crate::repository::login_event::MockLoginEventRepository;
    use crate::repository::rbac::MockRbacRepository;
    use crate::repository::security_alert::MockSecurityAlertRepository;
    use crate::repository::service::MockServiceRepository;
    use crate::repository::tenant::MockTenantRepository;
    use crate::repository::user::MockUserRepository;
    use crate::repository::webhook::MockWebhookRepository;
    use mockall::predicate::*;

    /// Helper function to create a TenantService with mock repositories
    fn create_test_service(
        tenant_repo: MockTenantRepository,
    ) -> TenantService<
        MockTenantRepository,
        MockServiceRepository,
        MockWebhookRepository,
        MockInvitationRepository,
        MockUserRepository,
        MockRbacRepository,
        MockLoginEventRepository,
        MockSecurityAlertRepository,
    > {
        TenantService::new(
            Arc::new(tenant_repo),
            Arc::new(MockServiceRepository::new()),
            Arc::new(MockWebhookRepository::new()),
            Arc::new(MockInvitationRepository::new()),
            Arc::new(MockUserRepository::new()),
            Arc::new(MockRbacRepository::new()),
            Arc::new(MockLoginEventRepository::new()),
            Arc::new(MockSecurityAlertRepository::new()),
            None,
        )
    }

    /// Helper function to create a TenantService with all mock repositories customizable
    #[allow(clippy::too_many_arguments)]
    fn create_test_service_full(
        tenant_repo: MockTenantRepository,
        service_repo: MockServiceRepository,
        webhook_repo: MockWebhookRepository,
        invitation_repo: MockInvitationRepository,
        user_repo: MockUserRepository,
        rbac_repo: MockRbacRepository,
        login_event_repo: MockLoginEventRepository,
        security_alert_repo: MockSecurityAlertRepository,
    ) -> TenantService<
        MockTenantRepository,
        MockServiceRepository,
        MockWebhookRepository,
        MockInvitationRepository,
        MockUserRepository,
        MockRbacRepository,
        MockLoginEventRepository,
        MockSecurityAlertRepository,
    > {
        TenantService::new(
            Arc::new(tenant_repo),
            Arc::new(service_repo),
            Arc::new(webhook_repo),
            Arc::new(invitation_repo),
            Arc::new(user_repo),
            Arc::new(rbac_repo),
            Arc::new(login_event_repo),
            Arc::new(security_alert_repo),
            None,
        )
    }

    #[tokio::test]
    async fn test_create_tenant_success() {
        let mut mock = MockTenantRepository::new();

        mock.expect_find_by_slug()
            .with(eq("test-tenant"))
            .returning(|_| Ok(None));

        mock.expect_create().returning(|input| {
            Ok(Tenant {
                name: input.name.clone(),
                slug: input.slug.clone(),
                ..Default::default()
            })
        });

        let service = create_test_service(mock);

        let input = CreateTenantInput {
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            logo_url: None,
            settings: None,
        };

        let result = service.create(input).await;
        assert!(result.is_ok());

        let tenant = result.unwrap();
        assert_eq!(tenant.name, "Test Tenant");
        assert_eq!(tenant.slug, "test-tenant");
    }

    #[tokio::test]
    async fn test_create_tenant_with_settings() {
        let mut mock = MockTenantRepository::new();

        mock.expect_find_by_slug()
            .with(eq("custom-tenant"))
            .returning(|_| Ok(None));

        mock.expect_create().returning(|input| {
            Ok(Tenant {
                name: input.name.clone(),
                slug: input.slug.clone(),
                settings: input.settings.clone().unwrap_or_default(),
                ..Default::default()
            })
        });

        let service = create_test_service(mock);

        let settings = TenantSettings {
            require_mfa: true,
            session_timeout_secs: 7200,
            ..Default::default()
        };

        let input = CreateTenantInput {
            name: "Custom Tenant".to_string(),
            slug: "custom-tenant".to_string(),
            logo_url: Some("https://example.com/logo.png".to_string()),
            settings: Some(settings),
        };

        let result = service.create(input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_tenant_duplicate_slug() {
        let mut mock = MockTenantRepository::new();

        mock.expect_find_by_slug()
            .with(eq("existing-tenant"))
            .returning(|_| Ok(Some(Tenant::default())));

        let service = create_test_service(mock);

        let input = CreateTenantInput {
            name: "New Tenant".to_string(),
            slug: "existing-tenant".to_string(),
            logo_url: None,
            settings: None,
        };

        let result = service.create(input).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_create_tenant_invalid_slug() {
        let mock = MockTenantRepository::new();
        let service = create_test_service(mock);

        let input = CreateTenantInput {
            name: "Test".to_string(),
            slug: "Invalid Slug".to_string(), // Invalid format
            logo_url: None,
            settings: None,
        };

        let result = service.create(input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_create_tenant_empty_name() {
        let mock = MockTenantRepository::new();
        let service = create_test_service(mock);

        let input = CreateTenantInput {
            name: "".to_string(),
            slug: "valid-slug".to_string(),
            logo_url: None,
            settings: None,
        };

        let result = service.create(input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_get_tenant_success() {
        let mut mock = MockTenantRepository::new();
        let tenant = Tenant {
            name: "Test Tenant".to_string(),
            slug: "test".to_string(),
            ..Default::default()
        };
        let tenant_clone = tenant.clone();
        let id = tenant.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        let service = create_test_service(mock);

        let result = service.get(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Tenant");
    }

    #[tokio::test]
    async fn test_get_tenant_not_found() {
        let mut mock = MockTenantRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.get(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_by_slug_success() {
        let mut mock = MockTenantRepository::new();
        let tenant = Tenant {
            name: "Test".to_string(),
            slug: "test-slug".to_string(),
            ..Default::default()
        };
        let tenant_clone = tenant.clone();

        mock.expect_find_by_slug()
            .with(eq("test-slug"))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        let service = create_test_service(mock);

        let result = service.get_by_slug("test-slug").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().slug, "test-slug");
    }

    #[tokio::test]
    async fn test_get_by_slug_not_found() {
        let mut mock = MockTenantRepository::new();

        mock.expect_find_by_slug()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.get_by_slug("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_tenants() {
        let mut mock = MockTenantRepository::new();

        mock.expect_list().with(eq(0), eq(10)).returning(|_, _| {
            Ok(vec![
                Tenant {
                    name: "Tenant 1".to_string(),
                    ..Default::default()
                },
                Tenant {
                    name: "Tenant 2".to_string(),
                    ..Default::default()
                },
            ])
        });

        mock.expect_count().returning(|| Ok(2));

        let service = create_test_service(mock);

        let result = service.list(1, 10).await;
        assert!(result.is_ok());
        let (tenants, total) = result.unwrap();
        assert_eq!(tenants.len(), 2);
        assert_eq!(total, 2);
    }

    #[tokio::test]
    async fn test_list_tenants_pagination() {
        let mut mock = MockTenantRepository::new();

        mock.expect_list()
            .with(eq(10), eq(10)) // offset = (page - 1) * per_page = (2 - 1) * 10 = 10
            .returning(|_, _| {
                Ok(vec![Tenant {
                    name: "Tenant 11".to_string(),
                    ..Default::default()
                }])
            });

        mock.expect_count().returning(|| Ok(11));

        let service = create_test_service(mock);

        let result = service.list(2, 10).await;
        assert!(result.is_ok());
        let (tenants, total) = result.unwrap();
        assert_eq!(tenants.len(), 1);
        assert_eq!(total, 11);
    }

    #[tokio::test]
    async fn test_update_tenant_success() {
        let mut mock = MockTenantRepository::new();
        let tenant = Tenant {
            name: "Old Name".to_string(),
            slug: "test".to_string(),
            ..Default::default()
        };
        let tenant_clone = tenant.clone();
        let id = tenant.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        mock.expect_update().returning(|_, input| {
            Ok(Tenant {
                name: input.name.clone().unwrap_or_default(),
                ..Default::default()
            })
        });

        let service = create_test_service(mock);

        let input = UpdateTenantInput {
            name: Some("New Name".to_string()),
            logo_url: None,
            settings: None,
            status: None,
        };

        let result = service.update(id, input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "New Name");
    }

    #[tokio::test]
    async fn test_update_tenant_not_found() {
        let mut mock = MockTenantRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let input = UpdateTenantInput {
            name: Some("New Name".to_string()),
            logo_url: None,
            settings: None,
            status: None,
        };

        let result = service.update(id, input).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_update_tenant_invalid_name() {
        let mock = MockTenantRepository::new();
        let service = create_test_service(mock);
        let id = StringUuid::new_v4();

        let input = UpdateTenantInput {
            name: Some("".to_string()), // Empty name
            logo_url: None,
            settings: None,
            status: None,
        };

        let result = service.update(id, input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_delete_tenant_cascade_success() {
        let mut tenant_repo = MockTenantRepository::new();
        let mut service_repo = MockServiceRepository::new();
        let mut webhook_repo = MockWebhookRepository::new();
        let mut invitation_repo = MockInvitationRepository::new();
        let mut user_repo = MockUserRepository::new();
        let rbac_repo = MockRbacRepository::new();
        let mut login_event_repo = MockLoginEventRepository::new();
        let mut security_alert_repo = MockSecurityAlertRepository::new();

        let tenant = Tenant::default();
        let tenant_clone = tenant.clone();
        let id = tenant.id;

        // Setup expectations
        tenant_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        tenant_repo
            .expect_delete()
            .with(eq(id))
            .returning(|_| Ok(()));

        // No services for this tenant
        service_repo
            .expect_list_by_tenant()
            .returning(|_| Ok(vec![]));

        webhook_repo.expect_delete_by_tenant().returning(|_| Ok(0));
        invitation_repo
            .expect_delete_by_tenant()
            .returning(|_| Ok(0));

        user_repo
            .expect_list_tenant_user_ids_by_tenant()
            .returning(|_| Ok(vec![]));

        user_repo
            .expect_delete_tenant_memberships_by_tenant()
            .returning(|_| Ok(0));

        login_event_repo
            .expect_delete_by_tenant()
            .returning(|_| Ok(0));

        security_alert_repo
            .expect_delete_by_tenant()
            .returning(|_| Ok(0));

        let service = create_test_service_full(
            tenant_repo,
            service_repo,
            webhook_repo,
            invitation_repo,
            user_repo,
            rbac_repo,
            login_event_repo,
            security_alert_repo,
        );

        let result = service.delete(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_tenant_with_services() {
        let mut tenant_repo = MockTenantRepository::new();
        let mut service_repo = MockServiceRepository::new();
        let mut webhook_repo = MockWebhookRepository::new();
        let mut invitation_repo = MockInvitationRepository::new();
        let mut user_repo = MockUserRepository::new();
        let mut rbac_repo = MockRbacRepository::new();
        let mut login_event_repo = MockLoginEventRepository::new();
        let mut security_alert_repo = MockSecurityAlertRepository::new();

        let tenant = Tenant::default();
        let tenant_clone = tenant.clone();
        let id = tenant.id;
        let service_id = StringUuid::new_v4();

        // Setup expectations
        tenant_repo
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        tenant_repo
            .expect_delete()
            .with(eq(id))
            .returning(|_| Ok(()));

        // One service for this tenant
        service_repo.expect_list_by_tenant().returning(move |_| {
            Ok(vec![crate::domain::Service {
                id: service_id,
                tenant_id: Some(id),
                name: "Test Service".to_string(),
                ..Default::default()
            }])
        });

        service_repo
            .expect_delete_clients_by_service()
            .returning(|_| Ok(2));

        service_repo.expect_delete().returning(|_| Ok(()));

        rbac_repo
            .expect_clear_parent_role_references()
            .returning(|_| Ok(0));

        rbac_repo
            .expect_delete_roles_by_service()
            .returning(|_| Ok(3));

        rbac_repo
            .expect_delete_permissions_by_service()
            .returning(|_| Ok(5));

        webhook_repo.expect_delete_by_tenant().returning(|_| Ok(1));
        invitation_repo
            .expect_delete_by_tenant()
            .returning(|_| Ok(2));

        user_repo
            .expect_list_tenant_user_ids_by_tenant()
            .returning(|_| Ok(vec![StringUuid::new_v4()]));

        rbac_repo
            .expect_delete_user_roles_by_tenant_user()
            .returning(|_| Ok(2));

        user_repo
            .expect_delete_tenant_memberships_by_tenant()
            .returning(|_| Ok(1));

        login_event_repo
            .expect_delete_by_tenant()
            .returning(|_| Ok(10));

        security_alert_repo
            .expect_delete_by_tenant()
            .returning(|_| Ok(5));

        let service = create_test_service_full(
            tenant_repo,
            service_repo,
            webhook_repo,
            invitation_repo,
            user_repo,
            rbac_repo,
            login_event_repo,
            security_alert_repo,
        );

        let result = service.delete(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_tenant_not_found() {
        let mut mock = MockTenantRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.delete(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_disable_tenant_success() {
        let mut mock = MockTenantRepository::new();
        let tenant = Tenant {
            status: TenantStatus::Active,
            ..Default::default()
        };
        let tenant_clone = tenant.clone();
        let id = tenant.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        mock.expect_update().returning(|_, input| {
            Ok(Tenant {
                status: input.status.clone().unwrap_or(TenantStatus::Active),
                ..Default::default()
            })
        });

        let service = create_test_service(mock);

        let result = service.disable(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, TenantStatus::Inactive);
    }

    #[tokio::test]
    async fn test_disable_tenant_not_found() {
        let mut mock = MockTenantRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.disable(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
