use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::error::{AppError, Result};
use crate::identity_engine::models::verification::{
    CreateVerificationTokenInput, EmailVerificationToken, UserVerificationStatus,
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait VerificationRepository: Send + Sync {
    async fn get_or_create(&self, user_id: &str) -> Result<UserVerificationStatus>;
    async fn set_email_verified(&self, user_id: &str, verified: bool) -> Result<()>;
    async fn create_token(
        &self,
        input: &CreateVerificationTokenInput,
    ) -> Result<EmailVerificationToken>;
    async fn find_valid_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<EmailVerificationToken>>;
    async fn mark_token_used(&self, id: &str) -> Result<()>;
    async fn delete_expired_tokens(&self) -> Result<u64>;
    async fn invalidate_user_tokens(&self, user_id: &str) -> Result<u64>;
}

pub struct VerificationRepositoryImpl {
    pool: MySqlPool,
}

impl VerificationRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    fn row_to_token(&self, row: &sqlx::mysql::MySqlRow) -> Result<EmailVerificationToken> {
        use sqlx::Row;
        Ok(EmailVerificationToken {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            token_hash: row.try_get("token_hash")?,
            expires_at: row.try_get("expires_at")?,
            used_at: row.try_get("used_at")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

#[async_trait]
impl VerificationRepository for VerificationRepositoryImpl {
    async fn get_or_create(&self, user_id: &str) -> Result<UserVerificationStatus> {
        // Upsert: insert if not exists, then read back.
        sqlx::query(
            r#"
            INSERT IGNORE INTO user_verification_status (user_id, email_verified)
            VALUES (?, 0)
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(
            "SELECT user_id, email_verified, email_verified_at, updated_at FROM user_verification_status WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        use sqlx::Row;
        let email_verified_raw: i8 = row.try_get("email_verified")?;
        Ok(UserVerificationStatus {
            user_id: row.try_get("user_id")?,
            email_verified: email_verified_raw != 0,
            email_verified_at: row.try_get("email_verified_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    async fn set_email_verified(&self, user_id: &str, verified: bool) -> Result<()> {
        let email_verified_at = if verified {
            "NOW()"
        } else {
            "NULL"
        };

        let query = format!(
            "UPDATE user_verification_status SET email_verified = ?, email_verified_at = {} WHERE user_id = ?",
            email_verified_at
        );

        let result = sqlx::query(&query)
            .bind(verified as i8)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "verification status for user '{}' not found",
                user_id
            )));
        }
        Ok(())
    }

    async fn create_token(
        &self,
        input: &CreateVerificationTokenInput,
    ) -> Result<EmailVerificationToken> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.user_id)
        .bind(&input.token_hash)
        .bind(input.expires_at)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(
            "SELECT id, user_id, token_hash, expires_at, used_at, created_at FROM email_verification_tokens WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_token(&row)
    }

    async fn find_valid_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<EmailVerificationToken>> {
        let row = sqlx::query(
            "SELECT id, user_id, token_hash, expires_at, used_at, created_at FROM email_verification_tokens WHERE token_hash = ? AND used_at IS NULL AND expires_at > NOW()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_token(&r)?)),
            None => Ok(None),
        }
    }

    async fn mark_token_used(&self, id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE email_verification_tokens SET used_at = NOW() WHERE id = ? AND used_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "verification token '{}' not found or already used",
                id
            )));
        }
        Ok(())
    }

    async fn delete_expired_tokens(&self) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM email_verification_tokens WHERE expires_at < NOW() OR used_at IS NOT NULL",
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn invalidate_user_tokens(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE email_verification_tokens SET used_at = NOW() WHERE user_id = ? AND used_at IS NULL",
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

    #[tokio::test]
    async fn get_or_create_new_user() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_get_or_create()
            .withf(|uid| uid == "user-1")
            .returning(|uid| {
                Ok(UserVerificationStatus {
                    user_id: uid.to_string(),
                    email_verified: false,
                    email_verified_at: None,
                    updated_at: chrono::Utc::now(),
                })
            });

        let status = mock.get_or_create("user-1").await.unwrap();
        assert_eq!(status.user_id, "user-1");
        assert!(!status.email_verified);
        assert!(status.email_verified_at.is_none());
    }

    #[tokio::test]
    async fn set_email_verified_true() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_set_email_verified()
            .withf(|uid, verified| uid == "user-1" && *verified)
            .returning(|_, _| Ok(()));

        mock.expect_get_or_create()
            .withf(|uid| uid == "user-1")
            .returning(|uid| {
                Ok(UserVerificationStatus {
                    user_id: uid.to_string(),
                    email_verified: true,
                    email_verified_at: Some(chrono::Utc::now()),
                    updated_at: chrono::Utc::now(),
                })
            });

        mock.set_email_verified("user-1", true).await.unwrap();
        let status = mock.get_or_create("user-1").await.unwrap();
        assert!(status.email_verified);
        assert!(status.email_verified_at.is_some());
    }

    #[tokio::test]
    async fn set_email_verified_false() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_set_email_verified()
            .withf(|uid, verified| uid == "user-1" && !*verified)
            .returning(|_, _| Ok(()));

        mock.set_email_verified("user-1", false).await.unwrap();
    }

    #[tokio::test]
    async fn create_and_find_token() {
        let mut mock = MockVerificationRepository::new();
        let now = chrono::Utc::now();

        mock.expect_create_token().returning(move |input| {
            Ok(EmailVerificationToken {
                id: "tok-1".to_string(),
                user_id: input.user_id.clone(),
                token_hash: input.token_hash.clone(),
                expires_at: input.expires_at,
                used_at: None,
                created_at: now,
            })
        });

        mock.expect_find_valid_token()
            .withf(|hash| hash == "sha256hash")
            .returning(move |_| {
                Ok(Some(EmailVerificationToken {
                    id: "tok-1".to_string(),
                    user_id: "user-1".to_string(),
                    token_hash: "sha256hash".to_string(),
                    expires_at: now + chrono::Duration::hours(24),
                    used_at: None,
                    created_at: now,
                }))
            });

        let input = CreateVerificationTokenInput {
            user_id: "user-1".to_string(),
            token_hash: "sha256hash".to_string(),
            expires_at: now + chrono::Duration::hours(24),
        };

        let created = mock.create_token(&input).await.unwrap();
        assert_eq!(created.user_id, "user-1");
        assert!(created.used_at.is_none());

        let found = mock.find_valid_token("sha256hash").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "tok-1");
    }

    #[tokio::test]
    async fn mark_token_used() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_mark_token_used()
            .withf(|id| id == "tok-1")
            .returning(|_| Ok(()));

        mock.mark_token_used("tok-1").await.unwrap();
    }

    #[tokio::test]
    async fn find_valid_token_returns_none_for_unknown() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_find_valid_token()
            .withf(|hash| hash == "nonexistent")
            .returning(|_| Ok(None));

        let result = mock.find_valid_token("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn invalidate_user_tokens() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_invalidate_user_tokens()
            .withf(|uid| uid == "user-1")
            .returning(|_| Ok(3));

        let count = mock.invalidate_user_tokens("user-1").await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn delete_expired_tokens() {
        let mut mock = MockVerificationRepository::new();

        mock.expect_delete_expired_tokens()
            .returning(|| Ok(5));

        let count = mock.delete_expired_tokens().await.unwrap();
        assert_eq!(count, 5);
    }
}
