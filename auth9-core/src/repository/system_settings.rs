//! System settings repository

use crate::domain::{SystemSettingRow, UpsertSystemSettingInput};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SystemSettingsRepository: Send + Sync {
    /// Get a setting by category and key
    async fn get(&self, category: &str, key: &str) -> Result<Option<SystemSettingRow>>;

    /// List all settings in a category
    async fn list_by_category(&self, category: &str) -> Result<Vec<SystemSettingRow>>;

    /// Upsert a setting (insert or update)
    async fn upsert(&self, input: &UpsertSystemSettingInput) -> Result<SystemSettingRow>;

    /// Delete a setting
    async fn delete(&self, category: &str, key: &str) -> Result<()>;
}

pub struct SystemSettingsRepositoryImpl {
    pool: MySqlPool,
}

impl SystemSettingsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SystemSettingsRepository for SystemSettingsRepositoryImpl {
    async fn get(&self, category: &str, key: &str) -> Result<Option<SystemSettingRow>> {
        let row = sqlx::query_as::<_, SystemSettingRow>(
            r#"
            SELECT id, category, setting_key, value, encrypted, description, created_at, updated_at
            FROM system_settings
            WHERE category = ? AND setting_key = ?
            "#,
        )
        .bind(category)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn list_by_category(&self, category: &str) -> Result<Vec<SystemSettingRow>> {
        let rows = sqlx::query_as::<_, SystemSettingRow>(
            r#"
            SELECT id, category, setting_key, value, encrypted, description, created_at, updated_at
            FROM system_settings
            WHERE category = ?
            ORDER BY setting_key
            "#,
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn upsert(&self, input: &UpsertSystemSettingInput) -> Result<SystemSettingRow> {
        let value_json = serde_json::to_string(&input.value)
            .map_err(|e| crate::error::AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO system_settings (category, setting_key, value, encrypted, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, NOW(), NOW())
            ON DUPLICATE KEY UPDATE
                value = VALUES(value),
                encrypted = VALUES(encrypted),
                description = VALUES(description),
                updated_at = NOW()
            "#,
        )
        .bind(&input.category)
        .bind(&input.setting_key)
        .bind(&value_json)
        .bind(input.encrypted)
        .bind(&input.description)
        .execute(&self.pool)
        .await?;

        // Fetch the updated/inserted row
        self.get(&input.category, &input.setting_key)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::Internal(anyhow::anyhow!("Failed to upsert setting"))
            })
    }

    async fn delete(&self, category: &str, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM system_settings WHERE category = ? AND setting_key = ?")
            .bind(category)
            .bind(key)
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
    async fn test_mock_get_setting() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("email"), eq("provider"))
            .returning(|_, _| {
                Ok(Some(SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({"type": "smtp"}),
                    encrypted: false,
                    description: Some("Email provider config".to_string()),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        let result = mock.get("email", "provider").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "email");
    }

    #[tokio::test]
    async fn test_mock_get_setting_not_found() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_get()
            .with(eq("nonexistent"), eq("key"))
            .returning(|_, _| Ok(None));

        let result = mock.get("nonexistent", "key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_list_by_category() {
        let mut mock = MockSystemSettingsRepository::new();

        mock.expect_list_by_category()
            .with(eq("email"))
            .returning(|_| {
                Ok(vec![SystemSettingRow {
                    id: 1,
                    category: "email".to_string(),
                    setting_key: "provider".to_string(),
                    value: serde_json::json!({"type": "smtp"}),
                    encrypted: false,
                    description: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }])
            });

        let result = mock.list_by_category("email").await.unwrap();
        assert_eq!(result.len(), 1);
    }
}
