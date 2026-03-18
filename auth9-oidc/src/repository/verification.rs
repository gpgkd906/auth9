use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::error::{CredentialError, Result};
use crate::models::verification::UserVerificationStatus;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait VerificationRepository: Send + Sync {
    async fn get_or_create(&self, user_id: &str) -> Result<UserVerificationStatus>;
    async fn set_email_verified(&self, user_id: &str, verified: bool) -> Result<()>;
}

pub struct VerificationRepositoryImpl {
    pool: MySqlPool,
}

impl VerificationRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
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
            return Err(CredentialError::NotFound(format!(
                "verification status for user '{}' not found",
                user_id
            )));
        }
        Ok(())
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
}
