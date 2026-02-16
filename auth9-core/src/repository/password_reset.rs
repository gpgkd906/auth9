//! Password reset token repository

use crate::domain::{CreatePasswordResetTokenInput, PasswordResetToken, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PasswordResetRepository: Send + Sync {
    async fn create(&self, input: &CreatePasswordResetTokenInput) -> Result<PasswordResetToken>;
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<PasswordResetToken>>;
    async fn find_valid_by_user(&self, user_id: StringUuid) -> Result<Option<PasswordResetToken>>;
    async fn mark_used(&self, id: StringUuid) -> Result<()>;
    /// Atomically claim a token by hash: marks it as used and returns it only if it was
    /// previously unused and not expired. Returns None if already claimed or expired.
    async fn claim_by_token_hash(&self, token_hash: &str) -> Result<Option<PasswordResetToken>>;
    async fn delete_expired(&self) -> Result<u64>;
    async fn delete_by_user(&self, user_id: StringUuid) -> Result<()>;
    async fn replace_for_user(
        &self,
        input: &CreatePasswordResetTokenInput,
    ) -> Result<PasswordResetToken>;
}

pub struct PasswordResetRepositoryImpl {
    pool: MySqlPool,
}

impl PasswordResetRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PasswordResetRepository for PasswordResetRepositoryImpl {
    async fn create(&self, input: &CreatePasswordResetTokenInput) -> Result<PasswordResetToken> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, created_at)
            VALUES (?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(&input.token_hash)
        .bind(input.expires_at)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id).await?.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed to create password reset token"))
        })
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<PasswordResetToken>> {
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, used_at, created_at
            FROM password_reset_tokens
            WHERE token_hash = ? AND used_at IS NULL AND expires_at > NOW()
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    async fn find_valid_by_user(&self, user_id: StringUuid) -> Result<Option<PasswordResetToken>> {
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, used_at, created_at
            FROM password_reset_tokens
            WHERE user_id = ? AND used_at IS NULL AND expires_at > NOW()
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    async fn mark_used(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Password reset token not found".to_string(),
            ));
        }

        Ok(())
    }

    async fn claim_by_token_hash(&self, token_hash: &str) -> Result<Option<PasswordResetToken>> {
        // Atomically mark the token as used only if it hasn't been used yet and is not expired.
        // This prevents TOCTOU race conditions: only one concurrent request can claim a token.
        let result = sqlx::query(
            r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE token_hash = ? AND used_at IS NULL AND expires_at > NOW()
            "#,
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        // Fetch the now-claimed token
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, used_at, created_at
            FROM password_reset_tokens
            WHERE token_hash = ?
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    async fn delete_expired(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM password_reset_tokens
            WHERE expires_at < NOW() OR used_at IS NOT NULL
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_user(&self, user_id: StringUuid) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM password_reset_tokens
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn replace_for_user(
        &self,
        input: &CreatePasswordResetTokenInput,
    ) -> Result<PasswordResetToken> {
        let id = StringUuid::new_v4();

        // Step 1: Delete all existing tokens for this user (non-transactional).
        // Under concurrency, multiple requests may execute this concurrently,
        // but each DELETE is individually atomic and visible to subsequent reads.
        sqlx::query(
            r#"
            DELETE FROM password_reset_tokens
            WHERE user_id = ?
            "#,
        )
        .bind(input.user_id)
        .execute(&self.pool)
        .await?;

        // Step 2: Insert the new token.
        sqlx::query(
            r#"
            INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, created_at)
            VALUES (?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(&input.token_hash)
        .bind(input.expires_at)
        .execute(&self.pool)
        .await?;

        // Step 3: Post-insert cleanup — keep only the latest token for this user.
        // This handles the race where concurrent requests each insert a token
        // between each other's DELETE and INSERT. After this cleanup, exactly
        // one valid token remains (the one with the most recent created_at).
        sqlx::query(
            r#"
            DELETE FROM password_reset_tokens
            WHERE user_id = ? AND id != (
                SELECT latest_id FROM (
                    SELECT id AS latest_id FROM password_reset_tokens
                    WHERE user_id = ?
                    ORDER BY created_at DESC
                    LIMIT 1
                ) AS t
            )
            "#,
        )
        .bind(input.user_id)
        .bind(input.user_id)
        .execute(&self.pool)
        .await?;

        // Read back whichever token survived the cleanup.
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, used_at, created_at
            FROM password_reset_tokens
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(input.user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }
}

impl PasswordResetRepositoryImpl {
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<PasswordResetToken>> {
        let token = sqlx::query_as::<_, PasswordResetToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, used_at, created_at
            FROM password_reset_tokens
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_password_reset_repository() {
        let mut mock = MockPasswordResetRepository::new();

        let token = PasswordResetToken::default();
        let token_clone = token.clone();

        mock.expect_find_by_token_hash()
            .with(eq("test-hash"))
            .returning(move |_| Ok(Some(token_clone.clone())));

        let result = mock.find_by_token_hash("test-hash").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_mock_find_valid_by_user() {
        let mut mock = MockPasswordResetRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_find_valid_by_user()
            .with(eq(user_id))
            .returning(|_| Ok(None));

        let result = mock.find_valid_by_user(user_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_mark_used() {
        let mut mock = MockPasswordResetRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_mark_used().with(eq(id)).returning(|_| Ok(()));

        let result = mock.mark_used(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_delete_expired() {
        let mut mock = MockPasswordResetRepository::new();

        mock.expect_delete_expired().returning(|| Ok(5));

        let count = mock.delete_expired().await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockPasswordResetRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            Ok(PasswordResetToken {
                user_id: input.user_id,
                token_hash: input.token_hash.clone(),
                expires_at: input.expires_at,
                ..Default::default()
            })
        });

        let input = CreatePasswordResetTokenInput {
            user_id,
            token_hash: "hash123".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let result = mock.create(&input).await.unwrap();
        assert_eq!(result.user_id, user_id);
        assert_eq!(result.token_hash, "hash123");
    }

    #[tokio::test]
    async fn test_mock_claim_by_token_hash_success() {
        let mut mock = MockPasswordResetRepository::new();

        let token = PasswordResetToken::default();
        let token_clone = token.clone();

        mock.expect_claim_by_token_hash()
            .with(eq("test-hash"))
            .returning(move |_| Ok(Some(token_clone.clone())));

        let result = mock.claim_by_token_hash("test-hash").await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_mock_claim_by_token_hash_already_used() {
        let mut mock = MockPasswordResetRepository::new();

        mock.expect_claim_by_token_hash()
            .with(eq("used-hash"))
            .returning(|_| Ok(None));

        let result = mock.claim_by_token_hash("used-hash").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_replace_for_user() {
        let mut mock = MockPasswordResetRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_replace_for_user().returning(|input| {
            Ok(PasswordResetToken {
                user_id: input.user_id,
                token_hash: input.token_hash.clone(),
                expires_at: input.expires_at,
                ..Default::default()
            })
        });

        let input = CreatePasswordResetTokenInput {
            user_id,
            token_hash: "new-hash".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        };

        let result = mock.replace_for_user(&input).await.unwrap();
        assert_eq!(result.user_id, user_id);
        assert_eq!(result.token_hash, "new-hash");
    }
}
