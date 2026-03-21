use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::error::{CredentialError, Result};
use crate::models::credential::{CreateCredentialInput, Credential, CredentialType};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait CredentialRepository: Send + Sync {
    async fn create(&self, input: &CreateCredentialInput) -> Result<Credential>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Credential>>;
    async fn find_by_user_and_type(
        &self,
        user_id: &str,
        credential_type: CredentialType,
    ) -> Result<Vec<Credential>>;
    async fn update_data(&self, id: &str, data: &serde_json::Value) -> Result<()>;
    async fn deactivate(&self, id: &str) -> Result<()>;
    async fn activate(&self, id: &str) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn delete_all_by_user(&self, user_id: &str) -> Result<u64>;
    async fn delete_by_user_and_type(
        &self,
        user_id: &str,
        credential_type: CredentialType,
    ) -> Result<u64>;
}

pub struct CredentialRepositoryImpl {
    pool: MySqlPool,
}

impl CredentialRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    fn row_to_credential(&self, row: &sqlx::mysql::MySqlRow) -> Result<Credential> {
        use sqlx::Row;
        let type_str: String = row.try_get("credential_type")?;
        let credential_type = CredentialType::from_str_value(&type_str).ok_or_else(|| {
            CredentialError::Internal(anyhow::anyhow!(
                "unknown credential type: {}",
                type_str
            ))
        })?;
        let is_active: i8 = row.try_get("is_active")?;

        Ok(Credential {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            credential_type,
            credential_data: row.try_get("credential_data")?,
            user_label: row.try_get("user_label")?,
            is_active: is_active != 0,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[async_trait]
impl CredentialRepository for CredentialRepositoryImpl {
    async fn create(&self, input: &CreateCredentialInput) -> Result<Credential> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO credentials (id, user_id, credential_type, credential_data, user_label)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.user_id)
        .bind(input.credential_type.as_str())
        .bind(&input.credential_data)
        .bind(&input.user_label)
        .execute(&self.pool)
        .await?;

        self.find_by_id(&id)
            .await?
            .ok_or_else(|| CredentialError::Internal(anyhow::anyhow!("failed to read back credential after insert")))
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Credential>> {
        let row = sqlx::query(
            "SELECT id, user_id, credential_type, credential_data, user_label, is_active, created_at, updated_at FROM credentials WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_credential(&r)?)),
            None => Ok(None),
        }
    }

    async fn find_by_user_and_type(
        &self,
        user_id: &str,
        credential_type: CredentialType,
    ) -> Result<Vec<Credential>> {
        let rows = sqlx::query(
            "SELECT id, user_id, credential_type, credential_data, user_label, is_active, created_at, updated_at FROM credentials WHERE user_id = ? AND credential_type = ?",
        )
        .bind(user_id)
        .bind(credential_type.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(|r| self.row_to_credential(r)).collect()
    }

    async fn update_data(&self, id: &str, data: &serde_json::Value) -> Result<()> {
        let result = sqlx::query("UPDATE credentials SET credential_data = ? WHERE id = ?")
            .bind(data)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CredentialError::NotFound(format!(
                "credential '{}' not found",
                id
            )));
        }
        Ok(())
    }

    async fn deactivate(&self, id: &str) -> Result<()> {
        let result = sqlx::query("UPDATE credentials SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CredentialError::NotFound(format!(
                "credential '{}' not found",
                id
            )));
        }
        Ok(())
    }

    async fn activate(&self, id: &str) -> Result<()> {
        let result = sqlx::query("UPDATE credentials SET is_active = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CredentialError::NotFound(format!(
                "credential '{}' not found",
                id
            )));
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM credentials WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CredentialError::NotFound(format!(
                "credential '{}' not found",
                id
            )));
        }
        Ok(())
    }

    async fn delete_all_by_user(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM credentials WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_user_and_type(
        &self,
        user_id: &str,
        credential_type: CredentialType,
    ) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM credentials WHERE user_id = ? AND credential_type = ?")
                .bind(user_id)
                .bind(credential_type.as_str())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::credential::PasswordCredentialData;

    #[tokio::test]
    async fn create_and_find_password_credential() {
        let mut mock = MockCredentialRepository::new();

        let expected = Credential {
            id: "cred-1".to_string(),
            user_id: "user-1".to_string(),
            credential_type: CredentialType::Password,
            credential_data: serde_json::json!({
                "hash": "argon2id$hash",
                "algorithm": "argon2id",
                "temporary": false
            }),
            user_label: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let expected_clone = expected.clone();

        mock.expect_create().returning(move |_| Ok(expected_clone.clone()));

        let input = CreateCredentialInput {
            user_id: "user-1".to_string(),
            credential_type: CredentialType::Password,
            credential_data: serde_json::json!({
                "hash": "argon2id$hash",
                "algorithm": "argon2id",
                "temporary": false
            }),
            user_label: None,
        };

        let result = mock.create(&input).await.unwrap();
        assert_eq!(result.id, "cred-1");
        assert_eq!(result.credential_type, CredentialType::Password);
        assert!(result.is_active);

        let pwd: PasswordCredentialData =
            serde_json::from_value(result.credential_data).unwrap();
        assert_eq!(pwd.algorithm, "argon2id");
        assert!(!pwd.temporary);
    }

    #[tokio::test]
    async fn find_by_user_and_type_filters_correctly() {
        let mut mock = MockCredentialRepository::new();

        let password_cred = Credential {
            id: "cred-1".to_string(),
            user_id: "user-1".to_string(),
            credential_type: CredentialType::Password,
            credential_data: serde_json::json!({"hash": "h", "algorithm": "argon2id", "temporary": false}),
            user_label: None,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let pc = password_cred.clone();

        mock.expect_find_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::Password)
            .returning(move |_, _| Ok(vec![pc.clone()]));

        mock.expect_find_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::Totp)
            .returning(|_, _| Ok(vec![]));

        let passwords = mock
            .find_by_user_and_type("user-1", CredentialType::Password)
            .await
            .unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].credential_type, CredentialType::Password);

        let totps = mock
            .find_by_user_and_type("user-1", CredentialType::Totp)
            .await
            .unwrap();
        assert!(totps.is_empty());
    }

    #[tokio::test]
    async fn update_credential_data() {
        let mut mock = MockCredentialRepository::new();

        mock.expect_update_data()
            .withf(|id, _| id == "cred-1")
            .returning(|_, _| Ok(()));

        let new_data = serde_json::json!({
            "hash": "argon2id$new_hash",
            "algorithm": "argon2id",
            "temporary": false
        });
        mock.update_data("cred-1", &new_data).await.unwrap();
    }

    #[tokio::test]
    async fn deactivate_and_activate() {
        let mut mock = MockCredentialRepository::new();

        mock.expect_deactivate()
            .withf(|id| id == "cred-1")
            .returning(|_| Ok(()));

        mock.expect_activate()
            .withf(|id| id == "cred-1")
            .returning(|_| Ok(()));

        mock.expect_find_by_id()
            .withf(|id| id == "cred-1")
            .times(1)
            .returning(|_| {
                Ok(Some(Credential {
                    id: "cred-1".to_string(),
                    user_id: "user-1".to_string(),
                    credential_type: CredentialType::Password,
                    credential_data: serde_json::json!({}),
                    user_label: None,
                    is_active: false,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        mock.deactivate("cred-1").await.unwrap();

        let found = mock.find_by_id("cred-1").await.unwrap().unwrap();
        assert!(!found.is_active);

        mock.activate("cred-1").await.unwrap();
    }

    #[tokio::test]
    async fn delete_credential() {
        let mut mock = MockCredentialRepository::new();

        mock.expect_delete()
            .withf(|id| id == "cred-1")
            .returning(|_| Ok(()));

        mock.expect_find_by_id()
            .withf(|id| id == "cred-1")
            .returning(|_| Ok(None));

        mock.delete("cred-1").await.unwrap();
        let found = mock.find_by_id("cred-1").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn delete_all_by_user() {
        let mut mock = MockCredentialRepository::new();

        mock.expect_delete_all_by_user()
            .withf(|uid| uid == "user-1")
            .returning(|_| Ok(3));

        let count = mock.delete_all_by_user("user-1").await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn delete_by_user_and_type() {
        let mut mock = MockCredentialRepository::new();

        mock.expect_delete_by_user_and_type()
            .withf(|uid, ct| uid == "user-1" && *ct == CredentialType::RecoveryCode)
            .returning(|_, _| Ok(8));

        let count = mock
            .delete_by_user_and_type("user-1", CredentialType::RecoveryCode)
            .await
            .unwrap();
        assert_eq!(count, 8);
    }
}
