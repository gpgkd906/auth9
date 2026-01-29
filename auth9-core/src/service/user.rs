//! User business logic

use crate::domain::{AddUserToTenantInput, CreateUserInput, StringUuid, TenantUser, UpdateUserInput, User};
use crate::error::{AppError, Result};
use crate::repository::UserRepository;
use std::sync::Arc;
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

    pub async fn get(&self, id: StringUuid) -> Result<User> {
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

    pub async fn update(&self, id: StringUuid, input: UpdateUserInput) -> Result<User> {
        input.validate()?;
        let _ = self.get(id).await?;
        self.repo.update(id, &input).await
    }

    pub async fn delete(&self, id: StringUuid) -> Result<()> {
        let _ = self.get(id).await?;
        self.repo.delete(id).await
    }

    pub async fn set_mfa_enabled(&self, id: StringUuid, enabled: bool) -> Result<User> {
        let _ = self.get(id).await?;
        self.repo.update_mfa_enabled(id, enabled).await
    }

    pub async fn add_to_tenant(&self, input: AddUserToTenantInput) -> Result<TenantUser> {
        input.validate()?;
        self.repo.add_to_tenant(&input).await
    }

    pub async fn remove_from_tenant(&self, user_id: StringUuid, tenant_id: StringUuid) -> Result<()> {
        self.repo.remove_from_tenant(user_id, tenant_id).await
    }

    pub async fn list_tenant_users(
        &self,
        tenant_id: StringUuid,
        page: i64,
        per_page: i64,
    ) -> Result<Vec<User>> {
        let offset = (page - 1) * per_page;
        self.repo
            .find_tenant_users(tenant_id, offset, per_page)
            .await
    }

    pub async fn get_user_tenants(&self, user_id: StringUuid) -> Result<Vec<TenantUser>> {
        self.repo.find_user_tenants(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::user::MockUserRepository;
    use mockall::predicate::*;
    use uuid::Uuid;

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
                display_name: input.display_name.clone(),
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
        let user = result.unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.display_name, Some("Test User".to_string()));
    }

    #[tokio::test]
    async fn test_create_user_duplicate_email() {
        let mut mock = MockUserRepository::new();

        mock.expect_find_by_email()
            .with(eq("existing@example.com"))
            .returning(|_| Ok(Some(User {
                email: "existing@example.com".to_string(),
                ..Default::default()
            })));

        let service = UserService::new(Arc::new(mock));

        let input = CreateUserInput {
            email: "existing@example.com".to_string(),
            display_name: None,
            avatar_url: None,
        };

        let result = service.create("kc-123", input).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
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

    #[tokio::test]
    async fn test_get_user_success() {
        let mut mock = MockUserRepository::new();
        let user = User {
            email: "test@example.com".to_string(),
            ..Default::default()
        };
        let user_clone = user.clone();
        let id = user.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        let service = UserService::new(Arc::new(mock));

        let result = service.get(id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let mut mock = MockUserRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = UserService::new(Arc::new(mock));

        let result = service.get(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_by_email_success() {
        let mut mock = MockUserRepository::new();
        let user = User {
            email: "test@example.com".to_string(),
            ..Default::default()
        };
        let user_clone = user.clone();

        mock.expect_find_by_email()
            .with(eq("test@example.com"))
            .returning(move |_| Ok(Some(user_clone.clone())));

        let service = UserService::new(Arc::new(mock));

        let result = service.get_by_email("test@example.com").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_get_by_email_not_found() {
        let mut mock = MockUserRepository::new();

        mock.expect_find_by_email()
            .with(eq("nonexistent@example.com"))
            .returning(|_| Ok(None));

        let service = UserService::new(Arc::new(mock));

        let result = service.get_by_email("nonexistent@example.com").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_by_keycloak_id_success() {
        let mut mock = MockUserRepository::new();
        let user = User {
            keycloak_id: "kc-123".to_string(),
            ..Default::default()
        };
        let user_clone = user.clone();

        mock.expect_find_by_keycloak_id()
            .with(eq("kc-123"))
            .returning(move |_| Ok(Some(user_clone.clone())));

        let service = UserService::new(Arc::new(mock));

        let result = service.get_by_keycloak_id("kc-123").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().keycloak_id, "kc-123");
    }

    #[tokio::test]
    async fn test_get_by_keycloak_id_not_found() {
        let mut mock = MockUserRepository::new();

        mock.expect_find_by_keycloak_id()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let service = UserService::new(Arc::new(mock));

        let result = service.get_by_keycloak_id("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_users() {
        let mut mock = MockUserRepository::new();

        mock.expect_list()
            .with(eq(0), eq(10))
            .returning(|_, _| {
                Ok(vec![
                    User { email: "user1@example.com".to_string(), ..Default::default() },
                    User { email: "user2@example.com".to_string(), ..Default::default() },
                ])
            });

        mock.expect_count().returning(|| Ok(2));

        let service = UserService::new(Arc::new(mock));

        let result = service.list(1, 10).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(total, 2);
    }

    #[tokio::test]
    async fn test_list_users_pagination() {
        let mut mock = MockUserRepository::new();

        mock.expect_list()
            .with(eq(20), eq(10)) // offset = (page - 1) * per_page = (3 - 1) * 10 = 20
            .returning(|_, _| {
                Ok(vec![
                    User { email: "user21@example.com".to_string(), ..Default::default() },
                ])
            });

        mock.expect_count().returning(|| Ok(21));

        let service = UserService::new(Arc::new(mock));

        let result = service.list(3, 10).await;
        assert!(result.is_ok());
        let (users, total) = result.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(total, 21);
    }

    #[tokio::test]
    async fn test_update_user_success() {
        let mut mock = MockUserRepository::new();
        let user = User {
            display_name: Some("Old Name".to_string()),
            ..Default::default()
        };
        let user_clone = user.clone();
        let id = user.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        mock.expect_update().returning(|_, input| {
            Ok(User {
                display_name: input.display_name.clone(),
                avatar_url: input.avatar_url.clone(),
                ..Default::default()
            })
        });

        let service = UserService::new(Arc::new(mock));

        let input = UpdateUserInput {
            display_name: Some("New Name".to_string()),
            avatar_url: None,
        };

        let result = service.update(id, input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().display_name, Some("New Name".to_string()));
    }

    #[tokio::test]
    async fn test_update_user_not_found() {
        let mut mock = MockUserRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = UserService::new(Arc::new(mock));

        let input = UpdateUserInput {
            display_name: Some("New Name".to_string()),
            avatar_url: None,
        };

        let result = service.update(id, input).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_user_success() {
        let mut mock = MockUserRepository::new();
        let user = User::default();
        let user_clone = user.clone();
        let id = user.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        mock.expect_delete()
            .with(eq(id))
            .returning(|_| Ok(()));

        let service = UserService::new(Arc::new(mock));

        let result = service.delete(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_user_not_found() {
        let mut mock = MockUserRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = UserService::new(Arc::new(mock));

        let result = service.delete(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_set_mfa_enabled_success() {
        let mut mock = MockUserRepository::new();
        let user = User {
            mfa_enabled: false,
            ..Default::default()
        };
        let user_clone = user.clone();
        let id = user.id;

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        mock.expect_update_mfa_enabled()
            .with(eq(id), eq(true))
            .returning(|_, enabled| {
                Ok(User {
                    mfa_enabled: enabled,
                    ..Default::default()
                })
            });

        let service = UserService::new(Arc::new(mock));

        let result = service.set_mfa_enabled(id, true).await;
        assert!(result.is_ok());
        assert!(result.unwrap().mfa_enabled);
    }

    #[tokio::test]
    async fn test_set_mfa_enabled_not_found() {
        let mut mock = MockUserRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(id))
            .returning(|_| Ok(None));

        let service = UserService::new(Arc::new(mock));

        let result = service.set_mfa_enabled(id, true).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_add_to_tenant_success() {
        let mut mock = MockUserRepository::new();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        mock.expect_add_to_tenant().returning(|input| {
            Ok(TenantUser {
                id: StringUuid::new_v4(),
                tenant_id: StringUuid::from(input.tenant_id),
                user_id: StringUuid::from(input.user_id),
                role_in_tenant: input.role_in_tenant.clone(),
                joined_at: chrono::Utc::now(),
            })
        });

        let service = UserService::new(Arc::new(mock));

        let input = AddUserToTenantInput {
            user_id,
            tenant_id,
            role_in_tenant: "member".to_string(),
        };

        let result = service.add_to_tenant(input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().role_in_tenant, "member");
    }

    #[tokio::test]
    async fn test_remove_from_tenant_success() {
        let mut mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();

        mock.expect_remove_from_tenant()
            .with(eq(user_id), eq(tenant_id))
            .returning(|_, _| Ok(()));

        let service = UserService::new(Arc::new(mock));

        let result = service.remove_from_tenant(user_id, tenant_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_tenant_users() {
        let mut mock = MockUserRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_find_tenant_users()
            .with(eq(tenant_id), eq(0), eq(10))
            .returning(|_, _, _| {
                Ok(vec![
                    User { email: "user1@example.com".to_string(), ..Default::default() },
                    User { email: "user2@example.com".to_string(), ..Default::default() },
                ])
            });

        let service = UserService::new(Arc::new(mock));

        let result = service.list_tenant_users(tenant_id, 1, 10).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_user_tenants() {
        let mut mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_find_user_tenants()
            .with(eq(user_id))
            .returning(|uid| {
                Ok(vec![
                    TenantUser {
                        id: StringUuid::new_v4(),
                        tenant_id: StringUuid::new_v4(),
                        user_id: uid,
                        role_in_tenant: "member".to_string(),
                        joined_at: chrono::Utc::now(),
                    },
                ])
            });

        let service = UserService::new(Arc::new(mock));

        let result = service.get_user_tenants(user_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}
