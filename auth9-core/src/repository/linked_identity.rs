//! Linked identity repository

use crate::domain::{CreateLinkedIdentityInput, LinkedIdentity, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LinkedIdentityRepository: Send + Sync {
    async fn create(&self, input: &CreateLinkedIdentityInput) -> Result<LinkedIdentity>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<LinkedIdentity>>;
    async fn find_by_provider(
        &self,
        provider_alias: &str,
        external_user_id: &str,
    ) -> Result<Option<LinkedIdentity>>;
    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<LinkedIdentity>>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64>;
}

pub struct LinkedIdentityRepositoryImpl {
    pool: MySqlPool,
}

impl LinkedIdentityRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LinkedIdentityRepository for LinkedIdentityRepositoryImpl {
    async fn create(&self, input: &CreateLinkedIdentityInput) -> Result<LinkedIdentity> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO linked_identities (id, user_id, provider_type, provider_alias,
                                           external_user_id, external_email, linked_at)
            VALUES (?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(&input.provider_type)
        .bind(&input.provider_alias)
        .bind(&input.external_user_id)
        .bind(&input.external_email)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create linked identity")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<LinkedIdentity>> {
        let identity = sqlx::query_as::<_, LinkedIdentity>(
            r#"
            SELECT id, user_id, provider_type, provider_alias, external_user_id,
                   external_email, linked_at
            FROM linked_identities
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(identity)
    }

    async fn find_by_provider(
        &self,
        provider_alias: &str,
        external_user_id: &str,
    ) -> Result<Option<LinkedIdentity>> {
        let identity = sqlx::query_as::<_, LinkedIdentity>(
            r#"
            SELECT id, user_id, provider_type, provider_alias, external_user_id,
                   external_email, linked_at
            FROM linked_identities
            WHERE provider_alias = ? AND external_user_id = ?
            "#,
        )
        .bind(provider_alias)
        .bind(external_user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(identity)
    }

    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<LinkedIdentity>> {
        let identities = sqlx::query_as::<_, LinkedIdentity>(
            r#"
            SELECT id, user_id, provider_type, provider_alias, external_user_id,
                   external_email, linked_at
            FROM linked_identities
            WHERE user_id = ?
            ORDER BY linked_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(identities)
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM linked_identities
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Linked identity not found".to_string()));
        }

        Ok(())
    }

    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM linked_identities
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_linked_identity_repository() {
        let mut mock = MockLinkedIdentityRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_list_by_user().with(eq(user_id)).returning(|_| {
            Ok(vec![
                LinkedIdentity {
                    provider_type: "google".to_string(),
                    provider_alias: "google".to_string(),
                    ..Default::default()
                },
                LinkedIdentity {
                    provider_type: "github".to_string(),
                    provider_alias: "github".to_string(),
                    ..Default::default()
                },
            ])
        });

        let identities = mock.list_by_user(user_id).await.unwrap();
        assert_eq!(identities.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_find_by_provider() {
        let mut mock = MockLinkedIdentityRepository::new();

        mock.expect_find_by_provider()
            .with(eq("google"), eq("12345"))
            .returning(|provider_alias, external_user_id| {
                Ok(Some(LinkedIdentity {
                    provider_alias: provider_alias.to_string(),
                    external_user_id: external_user_id.to_string(),
                    ..Default::default()
                }))
            });

        let result = mock.find_by_provider("google", "12345").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().external_user_id, "12345");
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockLinkedIdentityRepository::new();

        mock.expect_create().returning(|input| {
            Ok(LinkedIdentity {
                user_id: input.user_id,
                provider_type: input.provider_type.clone(),
                provider_alias: input.provider_alias.clone(),
                external_user_id: input.external_user_id.clone(),
                external_email: input.external_email.clone(),
                ..Default::default()
            })
        });

        let input = CreateLinkedIdentityInput {
            user_id: StringUuid::new_v4(),
            provider_type: "google".to_string(),
            provider_alias: "google".to_string(),
            external_user_id: "12345".to_string(),
            external_email: Some("user@gmail.com".to_string()),
        };

        let result = mock.create(&input).await.unwrap();
        assert_eq!(result.provider_type, "google");
        assert_eq!(result.external_email, Some("user@gmail.com".to_string()));
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let mut mock = MockLinkedIdentityRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_delete().with(eq(id)).returning(|_| Ok(()));

        let result = mock.delete(id).await;
        assert!(result.is_ok());
    }
}
