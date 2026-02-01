//! Session repository

use crate::domain::{CreateSessionInput, Session, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, input: &CreateSessionInput) -> Result<Session>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Session>>;
    async fn find_by_keycloak_session(&self, keycloak_session_id: &str) -> Result<Option<Session>>;
    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>>;
    async fn list_active_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>>;
    async fn update_last_active(&self, id: StringUuid) -> Result<()>;
    async fn revoke(&self, id: StringUuid) -> Result<()>;
    async fn revoke_all_by_user(&self, user_id: StringUuid) -> Result<u64>;
    async fn revoke_all_except(&self, user_id: StringUuid, except_id: StringUuid) -> Result<u64>;
    async fn delete_old(&self, days: i64) -> Result<u64>;

    /// Delete all sessions for a user (for cascade delete)
    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64>;
}

pub struct SessionRepositoryImpl {
    pool: MySqlPool,
}

impl SessionRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for SessionRepositoryImpl {
    async fn create(&self, input: &CreateSessionInput) -> Result<Session> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, keycloak_session_id, device_type, device_name,
                                  ip_address, location, user_agent, last_active_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(&input.keycloak_session_id)
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
            SELECT id, user_id, keycloak_session_id, device_type, device_name,
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

    async fn find_by_keycloak_session(&self, keycloak_session_id: &str) -> Result<Option<Session>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, keycloak_session_id, device_type, device_name,
                   ip_address, location, user_agent, last_active_at, created_at, revoked_at
            FROM sessions
            WHERE keycloak_session_id = ?
            "#,
        )
        .bind(keycloak_session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, keycloak_session_id, device_type, device_name,
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
            SELECT id, user_id, keycloak_session_id, device_type, device_name,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_session_repository() {
        let mut mock = MockSessionRepository::new();

        let session = Session::default();
        let session_clone = session.clone();

        mock.expect_find_by_id()
            .with(eq(session.id))
            .returning(move |_| Ok(Some(session_clone.clone())));

        let result = mock.find_by_id(session.id).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_mock_list_active_by_user() {
        let mut mock = MockSessionRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_list_active_by_user()
            .with(eq(user_id))
            .returning(|_| Ok(vec![Session::default(), Session::default()]));

        let sessions = mock.list_active_by_user(user_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_revoke() {
        let mut mock = MockSessionRepository::new();
        let id = StringUuid::new_v4();

        mock.expect_revoke().with(eq(id)).returning(|_| Ok(()));

        let result = mock.revoke(id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_revoke_all_except() {
        let mut mock = MockSessionRepository::new();
        let user_id = StringUuid::new_v4();
        let except_id = StringUuid::new_v4();

        mock.expect_revoke_all_except()
            .with(eq(user_id), eq(except_id))
            .returning(|_, _| Ok(3));

        let count = mock.revoke_all_except(user_id, except_id).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockSessionRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            Ok(Session {
                user_id: input.user_id,
                device_type: input.device_type.clone(),
                ..Default::default()
            })
        });

        let input = CreateSessionInput {
            user_id,
            keycloak_session_id: Some("kc-session-123".to_string()),
            device_type: Some("desktop".to_string()),
            device_name: Some("Chrome".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            location: None,
            user_agent: None,
        };

        let session = mock.create(&input).await.unwrap();
        assert_eq!(session.user_id, user_id);
        assert_eq!(session.device_type, Some("desktop".to_string()));
    }
}
