//! impl SessionRepository for SessionRepositoryImpl

use super::{SessionRepository, SessionRepositoryImpl};
use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use crate::models::session::{CreateSessionInput, Session};
use async_trait::async_trait;

#[async_trait]
impl SessionRepository for SessionRepositoryImpl {
    async fn create(&self, input: &CreateSessionInput) -> Result<Session> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, provider_session_id, keycloak_session_id, device_type, device_name,
                                  ip_address, location, user_agent, last_active_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(&input.provider_session_id)
        .bind(&input.provider_session_id)
        .bind(&input.device_type)
        .bind(&input.device_name)
        .bind(&input.ip_address)
        .bind(&input.location)
        .bind(&input.user_agent)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create session")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Session>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, COALESCE(provider_session_id, keycloak_session_id) AS provider_session_id, device_type, device_name,
                   ip_address, location, user_agent, last_active_at, created_at, revoked_at
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    async fn find_by_provider_session_id(
        &self,
        provider_session_id: &str,
    ) -> Result<Option<Session>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, COALESCE(provider_session_id, keycloak_session_id) AS provider_session_id, device_type, device_name,
                   ip_address, location, user_agent, last_active_at, created_at, revoked_at
            FROM sessions
            WHERE provider_session_id = ? OR keycloak_session_id = ?
            "#,
        )
        .bind(provider_session_id)
        .bind(provider_session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, COALESCE(provider_session_id, keycloak_session_id) AS provider_session_id, device_type, device_name,
                   ip_address, location, user_agent, last_active_at, created_at, revoked_at
            FROM sessions
            WHERE user_id = ?
            ORDER BY last_active_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    async fn list_active_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, COALESCE(provider_session_id, keycloak_session_id) AS provider_session_id, device_type, device_name,
                   ip_address, location, user_agent, last_active_at, created_at, revoked_at
            FROM sessions
            WHERE user_id = ? AND revoked_at IS NULL
            ORDER BY last_active_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    async fn update_last_active(&self, id: StringUuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET last_active_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn revoke(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET revoked_at = NOW()
            WHERE id = ? AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Session not found or already revoked".to_string(),
            ));
        }

        Ok(())
    }

    async fn revoke_all_by_user(&self, user_id: StringUuid) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET revoked_at = NOW()
            WHERE user_id = ? AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn revoke_all_except(&self, user_id: StringUuid, except_id: StringUuid) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET revoked_at = NOW()
            WHERE user_id = ? AND id != ? AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(except_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn delete_old(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn count_active_by_user(&self, user_id: StringUuid) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM sessions
            WHERE user_id = ? AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    async fn find_oldest_active_by_user(&self, user_id: StringUuid) -> Result<Option<Session>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, COALESCE(provider_session_id, keycloak_session_id) AS provider_session_id, device_type, device_name,
                   ip_address, location, user_agent, last_active_at, created_at, revoked_at
            FROM sessions
            WHERE user_id = ? AND revoked_at IS NULL
            ORDER BY last_active_at ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }
}
