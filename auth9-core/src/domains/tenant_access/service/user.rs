//! User business logic

use crate::domain::{
    AddUserToTenantInput, CreateUserInput, StringUuid, TenantUser, TenantUserWithTenant,
    UpdateUserInput, User, WebhookEvent,
};
use crate::domains::integration::service::WebhookEventPublisher;
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakClient;
use crate::repository::{
    AuditRepository, LinkedIdentityRepository, LoginEventRepository, PasswordResetRepository,
    RbacRepository, SecurityAlertRepository, SessionRepository, UserRepository,
};
use chrono::Utc;
use sqlx::MySqlPool;
use std::sync::Arc;
use tracing::warn;
use validator::Validate;

/// Repository bundle for UserService
pub struct UserRepositoryBundle<
    R: UserRepository,
    S: SessionRepository,
    P: PasswordResetRepository,
    L: LinkedIdentityRepository,
    LE: LoginEventRepository,
    SA: SecurityAlertRepository,
    A: AuditRepository,
    Rbac: RbacRepository,
> {
    pub user: Arc<R>,
    pub session: Arc<S>,
    pub password_reset: Arc<P>,
    pub linked_identity: Arc<L>,
    pub login_event: Arc<LE>,
    pub security_alert: Arc<SA>,
    pub audit: Arc<A>,
    pub rbac: Arc<Rbac>,
}

impl<R, S, P, L, LE, SA, A, Rbac> UserRepositoryBundle<R, S, P, L, LE, SA, A, Rbac>
where
    R: UserRepository,
    S: SessionRepository,
    P: PasswordResetRepository,
    L: LinkedIdentityRepository,
    LE: LoginEventRepository,
    SA: SecurityAlertRepository,
    A: AuditRepository,
    Rbac: RbacRepository,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user: Arc<R>,
        session: Arc<S>,
        password_reset: Arc<P>,
        linked_identity: Arc<L>,
        login_event: Arc<LE>,
        security_alert: Arc<SA>,
        audit: Arc<A>,
        rbac: Arc<Rbac>,
    ) -> Self {
        Self {
            user,
            session,
            password_reset,
            linked_identity,
            login_event,
            security_alert,
            audit,
            rbac,
        }
    }
}

pub struct UserService<
    R: UserRepository,
    S: SessionRepository,
    P: PasswordResetRepository,
    L: LinkedIdentityRepository,
    LE: LoginEventRepository,
    SA: SecurityAlertRepository,
    A: AuditRepository,
    Rbac: RbacRepository,
> {
    repo: Arc<R>,
    session_repo: Arc<S>,
    password_reset_repo: Arc<P>,
    linked_identity_repo: Arc<L>,
    login_event_repo: Arc<LE>,
    security_alert_repo: Arc<SA>,
    audit_repo: Arc<A>,
    rbac_repo: Arc<Rbac>,
    keycloak: Option<KeycloakClient>,
    webhook_publisher: Option<Arc<dyn WebhookEventPublisher>>,
    /// Database pool for transactional cascade deletes.
    /// When available, delete operations are wrapped in a transaction.
    pool: Option<MySqlPool>,
}

impl<
        R: UserRepository,
        S: SessionRepository,
        P: PasswordResetRepository,
        L: LinkedIdentityRepository,
        LE: LoginEventRepository,
        SA: SecurityAlertRepository,
        A: AuditRepository,
        Rbac: RbacRepository,
    > UserService<R, S, P, L, LE, SA, A, Rbac>
{
    /// Create a new UserService with repository bundle, keycloak client, and webhook publisher
    pub fn new(
        repos: UserRepositoryBundle<R, S, P, L, LE, SA, A, Rbac>,
        keycloak: Option<KeycloakClient>,
        webhook_publisher: Option<Arc<dyn WebhookEventPublisher>>,
    ) -> Self {
        Self {
            repo: repos.user,
            session_repo: repos.session,
            password_reset_repo: repos.password_reset,
            linked_identity_repo: repos.linked_identity,
            login_event_repo: repos.login_event,
            security_alert_repo: repos.security_alert,
            audit_repo: repos.audit,
            rbac_repo: repos.rbac,
            keycloak,
            webhook_publisher,
            pool: None,
        }
    }

    /// Set the database pool for transactional cascade deletes
    pub fn with_pool(mut self, pool: MySqlPool) -> Self {
        self.pool = Some(pool);
        self
    }

    pub async fn create(&self, keycloak_id: &str, input: CreateUserInput) -> Result<User> {
        input.validate()?;

        // Check for duplicate keycloak_id (not email — multiple IdP users may share an email)
        if self.repo.find_by_keycloak_id(keycloak_id).await?.is_some() {
            return Err(AppError::Conflict(format!(
                "User with keycloak_id '{}' already exists",
                keycloak_id
            )));
        }

        let user = self.repo.create(keycloak_id, &input).await?;

        // Trigger user.created webhook event
        if let Some(publisher) = &self.webhook_publisher {
            if let Err(e) = publisher
                .trigger_event(WebhookEvent {
                    event_type: "user.created".to_string(),
                    timestamp: Utc::now(),
                    data: serde_json::json!({
                        "user_id": user.id.to_string(),
                        "email": user.email,
                        "display_name": user.display_name,
                    }),
                })
                .await
            {
                tracing::warn!("Failed to trigger user.created webhook event: {}", e);
            }
        }

        Ok(user)
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

    pub async fn search(&self, query: &str, page: i64, per_page: i64) -> Result<(Vec<User>, i64)> {
        let offset = (page - 1) * per_page;
        let users = self.repo.search(query, offset, per_page).await?;
        let total = self.repo.search_count(query).await?;
        Ok((users, total))
    }

    pub async fn update(&self, id: StringUuid, input: UpdateUserInput) -> Result<User> {
        input.validate()?;
        let _ = self.get(id).await?;
        let user = self.repo.update(id, &input).await?;

        // Trigger user.updated webhook event
        if let Some(publisher) = &self.webhook_publisher {
            if let Err(e) = publisher
                .trigger_event(WebhookEvent {
                    event_type: "user.updated".to_string(),
                    timestamp: Utc::now(),
                    data: serde_json::json!({
                        "user_id": user.id.to_string(),
                        "email": user.email,
                        "display_name": user.display_name,
                    }),
                })
                .await
            {
                tracing::warn!("Failed to trigger user.updated webhook event: {}", e);
            }
        }

        Ok(user)
    }

    /// Delete a user with cascade delete of all related data.
    ///
    /// When a database pool is available, all cascade operations run within a single
    /// transaction. External operations (Keycloak delete, webhooks) run after commit.
    ///
    /// Cascade order (within transaction):
    /// 1. Delete user_tenant_roles for all tenant memberships
    /// 2. Delete tenant_users (user's tenant memberships)
    /// 3. Delete sessions
    /// 4. Delete password_reset_tokens
    /// 5. Delete linked_identities
    /// 6. Nullify user_id in login_events (preserve audit trail)
    /// 7. Nullify user_id in security_alerts (preserve audit trail)
    /// 8. Nullify actor_id in audit_logs (preserve audit trail)
    /// 9. Delete users record
    ///
    /// After commit:
    /// 10. Delete user from Keycloak (tolerant of NotFound)
    /// 11. Trigger user.deleted webhook event
    pub async fn delete(&self, id: StringUuid) -> Result<()> {
        let user = self.get(id).await?;

        if let Some(ref pool) = self.pool {
            // Transactional path: wrap all DB cascade operations in a single transaction
            let mut tx = pool.begin().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to begin transaction: {}", e))
            })?;
            let id_str = id.to_string();

            // 1. Delete user_tenant_roles via tenant_user IDs
            sqlx::query(
                "DELETE utr FROM user_tenant_roles utr \
                 INNER JOIN tenant_users tu ON utr.tenant_user_id = tu.id \
                 WHERE tu.user_id = ?",
            )
            .bind(&id_str)
            .execute(tx.as_mut())
            .await
            .map_err(AppError::Database)?;

            // 2. Delete tenant memberships
            sqlx::query("DELETE FROM tenant_users WHERE user_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 3. Delete sessions
            sqlx::query("DELETE FROM sessions WHERE user_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 4. Delete password reset tokens
            sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 5. Delete linked identities
            sqlx::query("DELETE FROM linked_identities WHERE user_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 6. Nullify user_id in login_events
            sqlx::query("UPDATE login_events SET user_id = NULL WHERE user_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 7. Nullify user_id in security_alerts
            sqlx::query("UPDATE security_alerts SET user_id = NULL WHERE user_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 8. Nullify actor_id in audit_logs
            sqlx::query("UPDATE audit_logs SET actor_id = NULL WHERE actor_id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            // 9. Delete user record
            sqlx::query("DELETE FROM users WHERE id = ?")
                .bind(&id_str)
                .execute(tx.as_mut())
                .await
                .map_err(AppError::Database)?;

            tx.commit().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to commit transaction: {}", e))
            })?;
        } else {
            // Non-transactional path (tests with mock repositories)
            // 1. Get all tenant_user IDs and delete their role assignments
            let tenant_user_ids = self.repo.list_tenant_user_ids(id).await?;
            for tu_id in tenant_user_ids {
                self.rbac_repo
                    .delete_user_roles_by_tenant_user(tu_id)
                    .await?;
            }

            // 2. Delete tenant memberships
            self.repo.delete_all_tenant_memberships(id).await?;

            // 3. Delete sessions
            self.session_repo.delete_by_user(id).await?;

            // 4. Delete password reset tokens
            self.password_reset_repo.delete_by_user(id).await?;

            // 5. Delete linked identities
            self.linked_identity_repo.delete_by_user(id).await?;

            // 6. Nullify user_id in login_events (preserve audit trail)
            self.login_event_repo.nullify_user_id(id).await?;

            // 7. Nullify user_id in security_alerts (preserve audit trail)
            self.security_alert_repo.nullify_user_id(id).await?;

            // 8. Nullify actor_id in audit_logs (preserve audit trail)
            self.audit_repo.nullify_actor_id(id).await?;

            // 9. Delete user record
            self.repo.delete(id).await?;
        }

        // 10. Delete from Keycloak AFTER transaction commit
        // (external operation cannot be rolled back, so run after DB is consistent)
        if let Some(ref keycloak) = self.keycloak {
            match keycloak.delete_user(&user.keycloak_id).await {
                Ok(_) => {}
                Err(AppError::NotFound(_)) => {
                    warn!(
                        "User {} not found in Keycloak during delete, continuing",
                        user.keycloak_id
                    );
                }
                Err(e) => return Err(e),
            }
        }

        // 11. Trigger user.deleted webhook event
        if let Some(publisher) = &self.webhook_publisher {
            if let Err(e) = publisher
                .trigger_event(WebhookEvent {
                    event_type: "user.deleted".to_string(),
                    timestamp: Utc::now(),
                    data: serde_json::json!({
                        "user_id": id.to_string(),
                        "email": user.email,
                    }),
                })
                .await
            {
                tracing::warn!("Failed to trigger user.deleted webhook event: {}", e);
            }
        }

        Ok(())
    }

    pub async fn set_mfa_enabled(&self, id: StringUuid, enabled: bool) -> Result<User> {
        let _ = self.get(id).await?;
        self.repo.update_mfa_enabled(id, enabled).await
    }

    pub async fn add_to_tenant(&self, input: AddUserToTenantInput) -> Result<TenantUser> {
        input.validate()?;
        self.repo.add_to_tenant(&input).await.map_err(|e| {
            if let AppError::Database(ref db_err) = e {
                let err_str = db_err.to_string().to_lowercase();
                if err_str.contains("duplicate") || err_str.contains("unique") {
                    return AppError::Conflict(
                        "User is already a member of this tenant".to_string(),
                    );
                }
            }
            e
        })
    }

    pub async fn update_role_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        role: String,
    ) -> Result<TenantUser> {
        if role.is_empty() || role.len() > 50 {
            return Err(AppError::Validation(
                "Role must be between 1 and 50 characters".to_string(),
            ));
        }
        self.repo
            .update_role_in_tenant(user_id, tenant_id, &role)
            .await
    }

    /// Remove a user from a tenant with cascade delete of role assignments.
    ///
    /// Cascade order:
    /// 1. Delete user_tenant_roles for this tenant membership
    /// 2. Delete tenant_users record
    pub async fn remove_from_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<()> {
        // 1. Find tenant_user_id and delete role assignments
        if let Some(tenant_user_id) = self
            .rbac_repo
            .find_tenant_user_id(user_id, tenant_id)
            .await?
        {
            self.rbac_repo
                .delete_user_roles_by_tenant_user(tenant_user_id)
                .await?;
        }

        // 2. Delete tenant_users record
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

    pub async fn get_user_tenants_with_tenant(
        &self,
        user_id: StringUuid,
    ) -> Result<Vec<TenantUserWithTenant>> {
        self.repo.find_user_tenants_with_tenant(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::audit::MockAuditRepository;
    use crate::repository::linked_identity::MockLinkedIdentityRepository;
    use crate::repository::login_event::MockLoginEventRepository;
    use crate::repository::password_reset::MockPasswordResetRepository;
    use crate::repository::rbac::MockRbacRepository;
    use crate::repository::security_alert::MockSecurityAlertRepository;
    use crate::repository::session::MockSessionRepository;
    use crate::repository::user::MockUserRepository;
    use mockall::predicate::*;
    use uuid::Uuid;

    /// Helper to create a sqlx::Database error for testing duplicate key scenarios
    #[derive(Debug)]
    struct TestDbError(String);
    impl std::fmt::Display for TestDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl std::error::Error for TestDbError {}
    impl sqlx::error::DatabaseError for TestDbError {
        fn message(&self) -> &str {
            &self.0
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::UniqueViolation
        }
    }

    fn create_test_service(
        mock_user: MockUserRepository,
    ) -> UserService<
        MockUserRepository,
        MockSessionRepository,
        MockPasswordResetRepository,
        MockLinkedIdentityRepository,
        MockLoginEventRepository,
        MockSecurityAlertRepository,
        MockAuditRepository,
        MockRbacRepository,
    > {
        let repos = UserRepositoryBundle::new(
            Arc::new(mock_user),
            Arc::new(MockSessionRepository::new()),
            Arc::new(MockPasswordResetRepository::new()),
            Arc::new(MockLinkedIdentityRepository::new()),
            Arc::new(MockLoginEventRepository::new()),
            Arc::new(MockSecurityAlertRepository::new()),
            Arc::new(MockAuditRepository::new()),
            Arc::new(MockRbacRepository::new()),
        );
        UserService::new(repos, None, None)
    }

    #[tokio::test]
    async fn test_create_user_success() {
        let mut mock = MockUserRepository::new();

        mock.expect_find_by_keycloak_id()
            .with(eq("kc-123"))
            .returning(|_| Ok(None));

        mock.expect_create().returning(|keycloak_id, input| {
            Ok(User {
                keycloak_id: keycloak_id.to_string(),
                email: input.email.clone(),
                display_name: input.display_name.clone(),
                ..Default::default()
            })
        });

        let service = create_test_service(mock);

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
    async fn test_create_user_duplicate_keycloak_id() {
        let mut mock = MockUserRepository::new();

        mock.expect_find_by_keycloak_id()
            .with(eq("kc-existing"))
            .returning(|_| {
                Ok(Some(User {
                    keycloak_id: "kc-existing".to_string(),
                    email: "existing@example.com".to_string(),
                    ..Default::default()
                }))
            });

        let service = create_test_service(mock);

        let input = CreateUserInput {
            email: "existing@example.com".to_string(),
            display_name: None,
            avatar_url: None,
        };

        let result = service.create("kc-existing", input).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_create_user_invalid_email() {
        let mock = MockUserRepository::new();
        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

        let result = service.get_by_keycloak_id("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_users() {
        let mut mock = MockUserRepository::new();

        mock.expect_list().with(eq(0), eq(10)).returning(|_, _| {
            Ok(vec![
                User {
                    email: "user1@example.com".to_string(),
                    ..Default::default()
                },
                User {
                    email: "user2@example.com".to_string(),
                    ..Default::default()
                },
            ])
        });

        mock.expect_count().returning(|| Ok(2));

        let service = create_test_service(mock);

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
                Ok(vec![User {
                    email: "user21@example.com".to_string(),
                    ..Default::default()
                }])
            });

        mock.expect_count().returning(|| Ok(21));

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

        let input = UpdateUserInput {
            display_name: Some("New Name".to_string()),
            avatar_url: None,
        };

        let result = service.update(id, input).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_user_cascade_success() {
        let mut mock_user = MockUserRepository::new();
        let mut mock_session = MockSessionRepository::new();
        let mut mock_password_reset = MockPasswordResetRepository::new();
        let mut mock_linked_identity = MockLinkedIdentityRepository::new();
        let mut mock_login_event = MockLoginEventRepository::new();
        let mut mock_security_alert = MockSecurityAlertRepository::new();
        let mut mock_audit = MockAuditRepository::new();
        let mock_rbac = MockRbacRepository::new();

        let user = User::default();
        let user_clone = user.clone();
        let id = user.id;

        // User lookup
        mock_user
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        // List tenant_user IDs (empty for this test)
        mock_user
            .expect_list_tenant_user_ids()
            .with(eq(id))
            .returning(|_| Ok(vec![]));

        // Delete tenant memberships
        mock_user
            .expect_delete_all_tenant_memberships()
            .with(eq(id))
            .returning(|_| Ok(0));

        // Delete sessions
        mock_session
            .expect_delete_by_user()
            .with(eq(id))
            .returning(|_| Ok(0));

        // Delete password reset tokens
        mock_password_reset
            .expect_delete_by_user()
            .with(eq(id))
            .returning(|_| Ok(()));

        // Delete linked identities
        mock_linked_identity
            .expect_delete_by_user()
            .with(eq(id))
            .returning(|_| Ok(0));

        // Nullify login events
        mock_login_event
            .expect_nullify_user_id()
            .with(eq(id))
            .returning(|_| Ok(0));

        // Nullify security alerts
        mock_security_alert
            .expect_nullify_user_id()
            .with(eq(id))
            .returning(|_| Ok(0));

        // Nullify audit logs
        mock_audit
            .expect_nullify_actor_id()
            .with(eq(id))
            .returning(|_| Ok(0));

        // Delete user record
        mock_user.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let repos = UserRepositoryBundle::new(
            Arc::new(mock_user),
            Arc::new(mock_session),
            Arc::new(mock_password_reset),
            Arc::new(mock_linked_identity),
            Arc::new(mock_login_event),
            Arc::new(mock_security_alert),
            Arc::new(mock_audit),
            Arc::new(mock_rbac),
        );
        let service = UserService::new(repos, None, None);

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

        let service = create_test_service(mock);

        let result = service.delete(id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_user_with_tenant_memberships() {
        let mut mock_user = MockUserRepository::new();
        let mut mock_session = MockSessionRepository::new();
        let mut mock_password_reset = MockPasswordResetRepository::new();
        let mut mock_linked_identity = MockLinkedIdentityRepository::new();
        let mut mock_login_event = MockLoginEventRepository::new();
        let mut mock_security_alert = MockSecurityAlertRepository::new();
        let mut mock_audit = MockAuditRepository::new();
        let mut mock_rbac = MockRbacRepository::new();

        let user = User::default();
        let user_clone = user.clone();
        let id = user.id;
        let tu_id1 = StringUuid::new_v4();
        let tu_id2 = StringUuid::new_v4();

        // User lookup
        mock_user
            .expect_find_by_id()
            .with(eq(id))
            .returning(move |_| Ok(Some(user_clone.clone())));

        // List tenant_user IDs (2 memberships)
        mock_user
            .expect_list_tenant_user_ids()
            .with(eq(id))
            .returning(move |_| Ok(vec![tu_id1, tu_id2]));

        // Delete role assignments for each tenant_user
        mock_rbac
            .expect_delete_user_roles_by_tenant_user()
            .times(2)
            .returning(|_| Ok(0));

        // Delete tenant memberships
        mock_user
            .expect_delete_all_tenant_memberships()
            .with(eq(id))
            .returning(|_| Ok(2));

        // Delete sessions
        mock_session
            .expect_delete_by_user()
            .with(eq(id))
            .returning(|_| Ok(3));

        // Delete password reset tokens
        mock_password_reset
            .expect_delete_by_user()
            .with(eq(id))
            .returning(|_| Ok(()));

        // Delete linked identities
        mock_linked_identity
            .expect_delete_by_user()
            .with(eq(id))
            .returning(|_| Ok(1));

        // Nullify login events
        mock_login_event
            .expect_nullify_user_id()
            .with(eq(id))
            .returning(|_| Ok(5));

        // Nullify security alerts
        mock_security_alert
            .expect_nullify_user_id()
            .with(eq(id))
            .returning(|_| Ok(2));

        // Nullify audit logs
        mock_audit
            .expect_nullify_actor_id()
            .with(eq(id))
            .returning(|_| Ok(10));

        // Delete user record
        mock_user.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let repos = UserRepositoryBundle::new(
            Arc::new(mock_user),
            Arc::new(mock_session),
            Arc::new(mock_password_reset),
            Arc::new(mock_linked_identity),
            Arc::new(mock_login_event),
            Arc::new(mock_security_alert),
            Arc::new(mock_audit),
            Arc::new(mock_rbac),
        );
        let service = UserService::new(repos, None, None);

        let result = service.delete(id).await;
        assert!(result.is_ok());
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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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

        let service = create_test_service(mock);

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
    async fn test_add_to_tenant_duplicate_returns_conflict() {
        let mut mock = MockUserRepository::new();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        mock.expect_add_to_tenant().returning(|_| {
            Err(AppError::Database(sqlx::Error::Database(Box::new(
                TestDbError("Duplicate entry 'xxx' for key 'tenant_users.uk_tenant_user'".into()),
            ))))
        });

        let service = create_test_service(mock);

        let input = AddUserToTenantInput {
            user_id,
            tenant_id,
            role_in_tenant: "member".to_string(),
        };

        let result = service.add_to_tenant(input).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
        if let Err(AppError::Conflict(msg)) = result {
            assert!(msg.contains("already a member"));
        }
    }

    #[tokio::test]
    async fn test_remove_from_tenant_success() {
        let mut mock_user = MockUserRepository::new();
        let mut mock_rbac = MockRbacRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();
        let tenant_user_id = StringUuid::new_v4();

        // Find tenant_user_id
        mock_rbac
            .expect_find_tenant_user_id()
            .with(eq(user_id), eq(tenant_id))
            .returning(move |_, _| Ok(Some(tenant_user_id)));

        // Delete user_tenant_roles
        mock_rbac
            .expect_delete_user_roles_by_tenant_user()
            .with(eq(tenant_user_id))
            .returning(|_| Ok(2));

        // Delete tenant_users record
        mock_user
            .expect_remove_from_tenant()
            .with(eq(user_id), eq(tenant_id))
            .returning(|_, _| Ok(()));

        let repos = UserRepositoryBundle::new(
            Arc::new(mock_user),
            Arc::new(MockSessionRepository::new()),
            Arc::new(MockPasswordResetRepository::new()),
            Arc::new(MockLinkedIdentityRepository::new()),
            Arc::new(MockLoginEventRepository::new()),
            Arc::new(MockSecurityAlertRepository::new()),
            Arc::new(MockAuditRepository::new()),
            Arc::new(mock_rbac),
        );
        let service = UserService::new(repos, None, None);

        let result = service.remove_from_tenant(user_id, tenant_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_from_tenant_no_roles() {
        let mut mock_user = MockUserRepository::new();
        let mut mock_rbac = MockRbacRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();

        // User not found in tenant (no tenant_user record)
        mock_rbac
            .expect_find_tenant_user_id()
            .with(eq(user_id), eq(tenant_id))
            .returning(|_, _| Ok(None));

        // Should still delete tenant_users record (idempotent)
        mock_user
            .expect_remove_from_tenant()
            .with(eq(user_id), eq(tenant_id))
            .returning(|_, _| Ok(()));

        let repos = UserRepositoryBundle::new(
            Arc::new(mock_user),
            Arc::new(MockSessionRepository::new()),
            Arc::new(MockPasswordResetRepository::new()),
            Arc::new(MockLinkedIdentityRepository::new()),
            Arc::new(MockLoginEventRepository::new()),
            Arc::new(MockSecurityAlertRepository::new()),
            Arc::new(MockAuditRepository::new()),
            Arc::new(mock_rbac),
        );
        let service = UserService::new(repos, None, None);

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
                    User {
                        email: "user1@example.com".to_string(),
                        ..Default::default()
                    },
                    User {
                        email: "user2@example.com".to_string(),
                        ..Default::default()
                    },
                ])
            });

        let service = create_test_service(mock);

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
                Ok(vec![TenantUser {
                    id: StringUuid::new_v4(),
                    tenant_id: StringUuid::new_v4(),
                    user_id: uid,
                    role_in_tenant: "member".to_string(),
                    joined_at: chrono::Utc::now(),
                }])
            });

        let service = create_test_service(mock);

        let result = service.get_user_tenants(user_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }
}
