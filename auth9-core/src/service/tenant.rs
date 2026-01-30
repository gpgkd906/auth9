//! Tenant business logic

use crate::cache::CacheManager;
use crate::domain::{CreateTenantInput, StringUuid, Tenant, TenantStatus, UpdateTenantInput};
use crate::error::{AppError, Result};
use crate::repository::TenantRepository;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

pub struct TenantService<R: TenantRepository> {
    repo: Arc<R>,
    cache_manager: Option<CacheManager>,
}

impl<R: TenantRepository> TenantService<R> {
    pub fn new(repo: Arc<R>, cache_manager: Option<CacheManager>) -> Self {
        Self {
            repo,
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
        self.repo.delete(id).await?;
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
    use crate::repository::tenant::MockTenantRepository;
    use mockall::predicate::*;

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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
        let service = TenantService::new(Arc::new(mock), None);

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
        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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
        let service = TenantService::new(Arc::new(mock), None);
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
    async fn test_delete_tenant_success() {
        let mut mock = MockTenantRepository::new();
        let tenant = Tenant::default();
        let tenant_clone = tenant.clone();
        let id = tenant.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(tenant_clone.clone())));

        mock.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

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

        let service = TenantService::new(Arc::new(mock), None);

        let result = service.disable(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
