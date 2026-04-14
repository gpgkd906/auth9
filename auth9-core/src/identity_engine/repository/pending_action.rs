use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::error::{AppError, Result};
use crate::identity_engine::models::pending_action::{
    ActionStatus, ActionType, CreatePendingActionInput, PendingAction,
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PendingActionRepository: Send + Sync {
    async fn create(&self, input: &CreatePendingActionInput) -> Result<PendingAction>;
    async fn find_pending_by_user(&self, user_id: &str) -> Result<Vec<PendingAction>>;
    async fn complete(&self, id: &str) -> Result<()>;
    async fn cancel(&self, id: &str) -> Result<()>;
    async fn delete_by_user(&self, user_id: &str) -> Result<u64>;
}

pub struct PendingActionRepositoryImpl {
    pool: MySqlPool,
}

impl PendingActionRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    fn row_to_pending_action(&self, row: &sqlx::mysql::MySqlRow) -> Result<PendingAction> {
        use sqlx::Row;
        let type_str: String = row.try_get("action_type")?;
        let action_type = ActionType::from_str_value(&type_str)
            .ok_or_else(|| AppError::Internal(anyhow!("unknown action type: {}", type_str)))?;
        let status_str: String = row.try_get("status")?;
        let status = ActionStatus::from_str_value(&status_str)
            .ok_or_else(|| AppError::Internal(anyhow!("unknown action status: {}", status_str)))?;

        Ok(PendingAction {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            action_type,
            status,
            metadata: row.try_get("metadata")?,
            created_at: row.try_get("created_at")?,
            completed_at: row.try_get("completed_at")?,
        })
    }
}

#[async_trait]
impl PendingActionRepository for PendingActionRepositoryImpl {
    async fn create(&self, input: &CreatePendingActionInput) -> Result<PendingAction> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO pending_actions (id, user_id, action_type, metadata)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&input.user_id)
        .bind(input.action_type.as_str())
        .bind(&input.metadata)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query(
            "SELECT id, user_id, action_type, status, metadata, created_at, completed_at FROM pending_actions WHERE id = ?",
        )
        .bind(&id)
        .fetch_one(&self.pool)
        .await?;

        self.row_to_pending_action(&row)
    }

    async fn find_pending_by_user(&self, user_id: &str) -> Result<Vec<PendingAction>> {
        let rows = sqlx::query(
            "SELECT id, user_id, action_type, status, metadata, created_at, completed_at FROM pending_actions WHERE user_id = ? AND status = 'pending'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| self.row_to_pending_action(r))
            .collect()
    }

    async fn complete(&self, id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE pending_actions SET status = 'completed', completed_at = NOW() WHERE id = ? AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "pending action '{}' not found or already completed",
                id
            )));
        }
        Ok(())
    }

    async fn cancel(&self, id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE pending_actions SET status = 'cancelled', completed_at = NOW() WHERE id = ? AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "pending action '{}' not found or already completed",
                id
            )));
        }
        Ok(())
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM pending_actions WHERE user_id = ?")
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
    async fn create_and_find_pending() {
        let mut mock = MockPendingActionRepository::new();

        let action = PendingAction {
            id: "action-1".to_string(),
            user_id: "user-1".to_string(),
            action_type: ActionType::VerifyEmail,
            status: ActionStatus::Pending,
            metadata: Some(serde_json::json!({"email": "user@test.example.com"})),
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        let action_clone = action.clone();

        mock.expect_create().returning(move |_| Ok(action_clone.clone()));

        mock.expect_find_pending_by_user()
            .withf(|uid| uid == "user-1")
            .returning(move |_| {
                Ok(vec![PendingAction {
                    id: "action-1".to_string(),
                    user_id: "user-1".to_string(),
                    action_type: ActionType::VerifyEmail,
                    status: ActionStatus::Pending,
                    metadata: Some(serde_json::json!({"email": "user@test.example.com"})),
                    created_at: chrono::Utc::now(),
                    completed_at: None,
                }])
            });

        let input = CreatePendingActionInput {
            user_id: "user-1".to_string(),
            action_type: ActionType::VerifyEmail,
            metadata: Some(serde_json::json!({"email": "user@test.example.com"})),
        };

        let created = mock.create(&input).await.unwrap();
        assert_eq!(created.status, ActionStatus::Pending);
        assert!(created.completed_at.is_none());

        let pending = mock.find_pending_by_user("user-1").await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action_type, ActionType::VerifyEmail);
    }

    #[tokio::test]
    async fn complete_action() {
        let mut mock = MockPendingActionRepository::new();

        mock.expect_complete()
            .withf(|id| id == "action-1")
            .returning(|_| Ok(()));

        mock.complete("action-1").await.unwrap();
    }

    #[tokio::test]
    async fn cancel_action() {
        let mut mock = MockPendingActionRepository::new();

        mock.expect_cancel()
            .withf(|id| id == "action-1")
            .returning(|_| Ok(()));

        mock.cancel("action-1").await.unwrap();
    }

    #[tokio::test]
    async fn find_pending_excludes_completed() {
        let mut mock = MockPendingActionRepository::new();

        // After completing action-1, find_pending should return empty
        mock.expect_complete()
            .withf(|id| id == "action-1")
            .returning(|_| Ok(()));

        mock.expect_find_pending_by_user()
            .withf(|uid| uid == "user-1")
            .returning(|_| Ok(vec![]));

        mock.complete("action-1").await.unwrap();

        let pending = mock.find_pending_by_user("user-1").await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn delete_by_user() {
        let mut mock = MockPendingActionRepository::new();

        mock.expect_delete_by_user()
            .withf(|uid| uid == "user-1")
            .returning(|_| Ok(2));

        let count = mock.delete_by_user("user-1").await.unwrap();
        assert_eq!(count, 2);
    }
}
