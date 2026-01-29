//! User business logic

use crate::domain::{AddUserToTenantInput, CreateUserInput, TenantUser, UpdateUserInput, User};
use crate::error::{AppError, Result};
use crate::repository::UserRepository;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

pub struct UserService<R: UserRepository> {
    repo: Arc<R>,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn create(&self, keycloak_id: &str, input: CreateUserInput) -> Result<User> {
        input.validate()?;

        // Check for duplicate email
        if self.repo.find_by_email(&input.email).await?.is_some() {
            return Err(AppError::Conflict(format!(
                "User with email '{}' already exists",
                input.email
            )));
        }

        self.repo.create(keycloak_id, &input).await
    }

    pub async fn get(&self, id: Uuid) -> Result<User> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
    }

    pub async fn get_by_email(&self, email: &str) -> Result<User> {
        self.repo
            .find_by_email(email)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User '{}' not found", email)))
    }

    pub async fn get_by_keycloak_id(&self, keycloak_id: &str) -> Result<User> {
        self.repo
            .find_by_keycloak_id(keycloak_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    pub async fn list(&self, page: i64, per_page: i64) -> Result<(Vec<User>, i64)> {
        let offset = (page - 1) * per_page;
        let users = self.repo.list(offset, per_page).await?;
        let total = self.repo.count().await?;
        Ok((users, total))
    }

    pub async fn update(&self, id: Uuid, input: UpdateUserInput) -> Result<User> {
        input.validate()?;
        let _ = self.get(id).await?;
        self.repo.update(id, &input).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let _ = self.get(id).await?;
        self.repo.delete(id).await
    }

    pub async fn set_mfa_enabled(&self, id: Uuid, enabled: bool) -> Result<User> {
        let _ = self.get(id).await?;
        self.repo.update_mfa_enabled(id, enabled).await
    }

    pub async fn add_to_tenant(&self, input: AddUserToTenantInput) -> Result<TenantUser> {
        input.validate()?;
        self.repo.add_to_tenant(&input).await
    }

    pub async fn remove_from_tenant(&self, user_id: Uuid, tenant_id: Uuid) -> Result<()> {
        self.repo.remove_from_tenant(user_id, tenant_id).await
    }

    pub async fn list_tenant_users(
        &self,
        tenant_id: Uuid,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<User>> {
        let offset = (page - 1) * per_page;
        self.repo
            .find_tenant_users(tenant_id, offset, per_page)
            .await
    }

    pub async fn get_user_tenants(&self, user_id: Uuid) -> Result<Vec<TenantUser>> {
        self.repo.find_user_tenants(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::user::MockUserRepository;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_create_user_success() {
        let mut mock = MockUserRepository::new();

        mock.expect_find_by_email()
            .with(eq("test@example.com"))
            .returning(|_| Ok(None));

        mock.expect_create().returning(|keycloak_id, input| {
            Ok(User {
                keycloak_id: keycloak_id.to_string(),
                email: input.email.clone(),
                ..Default::default()
            })
        });

        let service = UserService::new(Arc::new(mock));

        let input = CreateUserInput {
            email: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            avatar_url: None,
        };

        let result = service.create("kc-123", input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_user_invalid_email() {
        let mock = MockUserRepository::new();
        let service = UserService::new(Arc::new(mock));

        let input = CreateUserInput {
            email: "invalid-email".to_string(),
            display_name: None,
            avatar_url: None,
        };

        let result = service.create("kc-123", input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }
}
