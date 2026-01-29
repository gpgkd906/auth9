//! Tenant business logic

use crate::cache::CacheManager;
use crate::domain::{CreateTenantInput, Tenant, TenantStatus, UpdateTenantInput};
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
            let _ = cache.set_tenant_config(tenant.id, &tenant).await;
        }
        Ok(tenant)
    }

    pub async fn get(&self, id: Uuid) -> Result<Tenant> {
        if let Some(cache) = &self.cache_manager {
            if let Ok(Some(tenant)) = cache.get_tenant_config(id).await {
                return Ok(tenant);
            }
        }
        let tenant = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Tenant {} not found", id)))?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_tenant_config(tenant.id, &tenant).await;
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
            let _ = cache.set_tenant_config(tenant.id, &tenant).await;
        }
        Ok(tenant)
    }

    pub async fn list(&self, page: i64, per_page: i64) -> Result<(Vec<Tenant>, i64)> {
        let offset = (page - 1) * per_page;
        let tenants = self.repo.list(offset, per_page).await?;
        let total = self.repo.count().await?;
        Ok((tenants, total))
    }

    pub async fn update(&self, id: Uuid, input: UpdateTenantInput) -> Result<Tenant> {
        input.validate()?;

        // Verify tenant exists
        let _ = self.get(id).await?;

        let tenant = self.repo.update(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_tenant_config(id).await;
        }
        Ok(tenant)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        // Verify tenant exists
        let _ = self.get(id).await?;
        self.repo.delete(id).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_tenant_config(id).await;
        }
        Ok(())
    }

    pub async fn disable(&self, id: Uuid) -> Result<Tenant> {
        let _ = self.get(id).await?;
        let input = UpdateTenantInput {
            name: None,
            logo_url: None,
            settings: None,
            status: Some(TenantStatus::Inactive),
        };
        let tenant = self.repo.update(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_tenant_config(id).await;
        }
        Ok(tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn test_get_tenant_not_found() {
        let mut mock = MockTenantRepository::new();
        let id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = TenantService::new(Arc::new(mock), None);

        let result = service.get(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }
}
