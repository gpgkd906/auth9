//! SCIM Token repository

use crate::domain::{ScimToken, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ScimTokenRepository: Send + Sync {
    async fn create(&self, token: &ScimToken) -> Result<ScimToken>;
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ScimToken>>;
    async fn list_by_connector(&self, connector_id: StringUuid) -> Result<Vec<ScimToken>>;
    async fn update_last_used(&self, id: StringUuid) -> Result<()>;
    async fn revoke(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_connector(&self, connector_id: StringUuid) -> Result<u64>;
}

pub struct ScimTokenRepositoryImpl {
    pool: MySqlPool,
}

impl ScimTokenRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScimTokenRepository for ScimTokenRepositoryImpl {
    async fn create(&self, token: &ScimToken) -> Result<ScimToken> {
        sqlx::query(
            r#"
            INSERT INTO scim_tokens (id, tenant_id, connector_id, token_hash, token_prefix, description, expires_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(token.id)
        .bind(token.tenant_id)
        .bind(token.connector_id)
        .bind(&token.token_hash)
        .bind(&token.token_prefix)
        .bind(&token.description)
        .bind(token.expires_at)
        .execute(&self.pool)
        .await?;

        self.find_by_hash(&token.token_hash)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create SCIM token")))
    }

    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ScimToken>> {
        let token = sqlx::query_as::<_, ScimToken>(
            r#"
            SELECT id, tenant_id, connector_id, token_hash, token_prefix, description,
                   expires_at, last_used_at, revoked_at, created_at, updated_at
            FROM scim_tokens
            WHERE token_hash = ?
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    async fn list_by_connector(&self, connector_id: StringUuid) -> Result<Vec<ScimToken>> {
        let tokens = sqlx::query_as::<_, ScimToken>(
            r#"
            SELECT id, tenant_id, connector_id, token_hash, token_prefix, description,
                   expires_at, last_used_at, revoked_at, created_at, updated_at
            FROM scim_tokens
            WHERE connector_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(connector_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(tokens)
    }

    async fn update_last_used(&self, id: StringUuid) -> Result<()> {
        sqlx::query("UPDATE scim_tokens SET last_used_at = NOW() WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn revoke(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query(
            "UPDATE scim_tokens SET revoked_at = NOW(), updated_at = NOW() WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("SCIM token {} not found", id)));
        }
        Ok(())
    }

    async fn delete_by_connector(&self, connector_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM scim_tokens WHERE connector_id = ?")
            .bind(connector_id)
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
    async fn test_mock_find_by_hash_not_found() {
        let mut mock = MockScimTokenRepository::new();
        mock.expect_find_by_hash()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let result = mock.find_by_hash("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_revoke() {
        let mut mock = MockScimTokenRepository::new();
        let id = StringUuid::new_v4();
        mock.expect_revoke().with(eq(id)).returning(|_| Ok(()));

        assert!(mock.revoke(id).await.is_ok());
    }
}
