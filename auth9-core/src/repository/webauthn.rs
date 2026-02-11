//! WebAuthn credentials repository

use crate::domain::{CreatePasskeyInput, StoredPasskey};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WebAuthnRepository: Send + Sync {
    async fn create(&self, input: &CreatePasskeyInput) -> Result<StoredPasskey>;
    async fn find_by_credential_id(&self, credential_id: &str) -> Result<Option<StoredPasskey>>;
    async fn list_by_user(&self, user_id: &str) -> Result<Vec<StoredPasskey>>;
    async fn delete(&self, id: &str, user_id: &str) -> Result<()>;
    async fn delete_by_user(&self, user_id: &str) -> Result<u64>;
    async fn update_last_used(&self, id: &str) -> Result<()>;
    async fn update_credential_data(&self, id: &str, data: &serde_json::Value) -> Result<()>;
}

pub struct WebAuthnRepositoryImpl {
    pool: MySqlPool,
}

impl WebAuthnRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WebAuthnRepository for WebAuthnRepositoryImpl {
    async fn create(&self, input: &CreatePasskeyInput) -> Result<StoredPasskey> {
        sqlx::query(
            r#"
            INSERT INTO webauthn_credentials (id, user_id, credential_id, credential_data, user_label, aaguid, created_at)
            VALUES (?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(&input.id)
        .bind(&input.user_id)
        .bind(&input.credential_id)
        .bind(&input.credential_data)
        .bind(&input.user_label)
        .bind(&input.aaguid)
        .execute(&self.pool)
        .await?;

        self.find_by_credential_id(&input.credential_id)
            .await?
            .ok_or_else(|| {
                AppError::Internal(anyhow::anyhow!("Failed to create webauthn credential"))
            })
    }

    async fn find_by_credential_id(&self, credential_id: &str) -> Result<Option<StoredPasskey>> {
        let row = sqlx::query_as::<_, StoredPasskey>(
            r#"
            SELECT id, user_id, credential_id, credential_data, user_label, aaguid, created_at, last_used_at
            FROM webauthn_credentials
            WHERE credential_id = ?
            "#,
        )
        .bind(credential_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn list_by_user(&self, user_id: &str) -> Result<Vec<StoredPasskey>> {
        let rows = sqlx::query_as::<_, StoredPasskey>(
            r#"
            SELECT id, user_id, credential_id, credential_data, user_label, aaguid, created_at, last_used_at
            FROM webauthn_credentials
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn delete(&self, id: &str, user_id: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            DELETE FROM webauthn_credentials
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "WebAuthn credential not found".to_string(),
            ));
        }

        Ok(())
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM webauthn_credentials WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn update_last_used(&self, id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webauthn_credentials
            SET last_used_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_credential_data(&self, id: &str, data: &serde_json::Value) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE webauthn_credentials
            SET credential_data = ?
            WHERE id = ?
            "#,
        )
        .bind(data)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_create().returning(|input| {
            Ok(StoredPasskey {
                id: input.id.clone(),
                user_id: input.user_id.clone(),
                credential_id: input.credential_id.clone(),
                credential_data: input.credential_data.clone(),
                user_label: input.user_label.clone(),
                aaguid: input.aaguid.clone(),
                created_at: chrono::Utc::now(),
                last_used_at: None,
            })
        });

        let input = CreatePasskeyInput {
            id: "pk-1".to_string(),
            user_id: "user-1".to_string(),
            credential_id: "cred-1".to_string(),
            credential_data: serde_json::json!({"key": "data"}),
            user_label: Some("My Key".to_string()),
            aaguid: None,
        };

        let result = mock.create(&input).await.unwrap();
        assert_eq!(result.id, "pk-1");
        assert_eq!(result.user_id, "user-1");
    }

    #[tokio::test]
    async fn test_mock_find_by_credential_id() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_find_by_credential_id()
            .with(eq("cred-123"))
            .returning(|cred_id| {
                Ok(Some(StoredPasskey {
                    id: "pk-1".to_string(),
                    user_id: "user-1".to_string(),
                    credential_id: cred_id.to_string(),
                    credential_data: serde_json::json!({}),
                    user_label: None,
                    aaguid: None,
                    created_at: chrono::Utc::now(),
                    last_used_at: None,
                }))
            });

        let result = mock.find_by_credential_id("cred-123").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().credential_id, "cred-123");
    }

    #[tokio::test]
    async fn test_mock_find_by_credential_id_not_found() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_find_by_credential_id()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let result = mock.find_by_credential_id("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_list_by_user() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_list_by_user()
            .with(eq("user-1"))
            .returning(|_| {
                Ok(vec![
                    StoredPasskey {
                        id: "pk-1".to_string(),
                        user_id: "user-1".to_string(),
                        credential_id: "cred-1".to_string(),
                        credential_data: serde_json::json!({}),
                        user_label: Some("Key 1".to_string()),
                        aaguid: None,
                        created_at: chrono::Utc::now(),
                        last_used_at: None,
                    },
                    StoredPasskey {
                        id: "pk-2".to_string(),
                        user_id: "user-1".to_string(),
                        credential_id: "cred-2".to_string(),
                        credential_data: serde_json::json!({}),
                        user_label: Some("Key 2".to_string()),
                        aaguid: None,
                        created_at: chrono::Utc::now(),
                        last_used_at: None,
                    },
                ])
            });

        let result = mock.list_by_user("user-1").await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_delete()
            .with(eq("pk-1"), eq("user-1"))
            .returning(|_, _| Ok(()));

        let result = mock.delete("pk-1", "user-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_delete_by_user() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_delete_by_user()
            .with(eq("user-1"))
            .returning(|_| Ok(3));

        let count = mock.delete_by_user("user-1").await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_mock_update_last_used() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_update_last_used()
            .with(eq("pk-1"))
            .returning(|_| Ok(()));

        let result = mock.update_last_used("pk-1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_update_credential_data() {
        let mut mock = MockWebAuthnRepository::new();

        mock.expect_update_credential_data()
            .returning(|_, _| Ok(()));

        let data = serde_json::json!({"counter": 5});
        let result = mock.update_credential_data("pk-1", &data).await;
        assert!(result.is_ok());
    }
}
