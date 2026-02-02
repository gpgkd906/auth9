//! Login event repository

#[allow(unused_imports)]
use crate::domain::{CreateLoginEventInput, LoginEvent, LoginEventType, LoginStats, StringUuid};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use std::collections::HashMap;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LoginEventRepository: Send + Sync {
    async fn create(&self, input: &CreateLoginEventInput) -> Result<i64>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<LoginEvent>>;
    async fn list_by_user(
        &self,
        user_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>>;
    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>>;
    async fn count(&self) -> Result<i64>;
    async fn count_by_user(&self, user_id: StringUuid) -> Result<i64>;
    async fn count_by_tenant(&self, tenant_id: StringUuid) -> Result<i64>;
    async fn get_stats(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<LoginStats>;
    async fn count_failed_by_ip(&self, ip_address: &str, since: DateTime<Utc>) -> Result<i64>;
    async fn count_failed_by_ip_multi_user(
        &self,
        ip_address: &str,
        since: DateTime<Utc>,
    ) -> Result<i64>;
    async fn delete_old(&self, days: i64) -> Result<u64>;

    /// Nullify user_id for login events (preserve audit trail when user is deleted)
    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64>;

    /// Delete all login events for a tenant (when tenant is deleted)
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
}

pub struct LoginEventRepositoryImpl {
    pool: MySqlPool,
}

impl LoginEventRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoginEventRepository for LoginEventRepositoryImpl {
    async fn create(&self, input: &CreateLoginEventInput) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO login_events (user_id, email, tenant_id, event_type, ip_address,
                                      user_agent, device_type, location, session_id,
                                      failure_reason, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(input.user_id)
        .bind(&input.email)
        .bind(input.tenant_id)
        .bind(&input.event_type)
        .bind(&input.ip_address)
        .bind(&input.user_agent)
        .bind(&input.device_type)
        .bind(&input.location)
        .bind(input.session_id)
        .bind(&input.failure_reason)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<LoginEvent>> {
        let events = sqlx::query_as::<_, LoginEvent>(
            r#"
            SELECT id, user_id, email, tenant_id, event_type, ip_address, user_agent,
                   device_type, location, session_id, failure_reason, created_at
            FROM login_events
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    async fn list_by_user(
        &self,
        user_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>> {
        let events = sqlx::query_as::<_, LoginEvent>(
            r#"
            SELECT id, user_id, email, tenant_id, event_type, ip_address, user_agent,
                   device_type, location, session_id, failure_reason, created_at
            FROM login_events
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>> {
        let events = sqlx::query_as::<_, LoginEvent>(
            r#"
            SELECT id, user_id, email, tenant_id, event_type, ip_address, user_agent,
                   device_type, location, session_id, failure_reason, created_at
            FROM login_events
            WHERE tenant_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    async fn count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM login_events")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn count_by_user(&self, user_id: StringUuid) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM login_events WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn count_by_tenant(&self, tenant_id: StringUuid) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM login_events WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn get_stats(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<LoginStats> {
        // Get basic counts
        // Note: SUM() returns DECIMAL in MySQL/TiDB, must CAST to SIGNED for i64 compatibility
        let counts: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                CAST(COALESCE(SUM(CASE WHEN event_type = 'success' OR event_type = 'social' THEN 1 ELSE 0 END), 0) AS SIGNED) as successful,
                CAST(COALESCE(SUM(CASE WHEN event_type IN ('failed_password', 'failed_mfa', 'locked') THEN 1 ELSE 0 END), 0) AS SIGNED) as failed,
                COUNT(DISTINCT user_id) as unique_users
            FROM login_events
            WHERE created_at BETWEEN ? AND ?
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        // Get counts by event type
        let event_type_counts: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT event_type, COUNT(*) as count
            FROM login_events
            WHERE created_at BETWEEN ? AND ?
            GROUP BY event_type
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        // Get counts by device type
        let device_type_counts: Vec<(Option<String>, i64)> = sqlx::query_as(
            r#"
            SELECT device_type, COUNT(*) as count
            FROM login_events
            WHERE created_at BETWEEN ? AND ?
            GROUP BY device_type
            "#,
        )
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        let by_event_type: HashMap<String, i64> = event_type_counts.into_iter().collect();
        let by_device_type: HashMap<String, i64> = device_type_counts
            .into_iter()
            .map(|(dt, count)| (dt.unwrap_or_else(|| "unknown".to_string()), count))
            .collect();

        Ok(LoginStats {
            total_logins: counts.0,
            successful_logins: counts.1,
            failed_logins: counts.2,
            unique_users: counts.3,
            by_event_type,
            by_device_type,
            period_start: start,
            period_end: end,
        })
    }

    async fn count_failed_by_ip(&self, ip_address: &str, since: DateTime<Utc>) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM login_events
            WHERE ip_address = ?
              AND event_type IN ('failed_password', 'failed_mfa')
              AND created_at >= ?
            "#,
        )
        .bind(ip_address)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn count_failed_by_ip_multi_user(
        &self,
        ip_address: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        // Count distinct users with failed attempts from this IP
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT COALESCE(user_id, email))
            FROM login_events
            WHERE ip_address = ?
              AND event_type IN ('failed_password', 'failed_mfa')
              AND created_at >= ?
            "#,
        )
        .bind(ip_address)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn delete_old(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM login_events
            WHERE created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("UPDATE login_events SET user_id = NULL WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM login_events WHERE tenant_id = ?")
            .bind(tenant_id)
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
    async fn test_mock_login_event_repository() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_count().returning(|| Ok(100));

        let count = mock.count().await.unwrap();
        assert_eq!(count, 100);
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_create().returning(|_| Ok(1));

        let input = CreateLoginEventInput {
            user_id: Some(StringUuid::new_v4()),
            email: Some("test@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::Success,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: None,
            device_type: Some("desktop".to_string()),
            location: None,
            session_id: None,
            failure_reason: None,
        };

        let id = mock.create(&input).await.unwrap();
        assert_eq!(id, 1);
    }

    #[tokio::test]
    async fn test_mock_get_stats() {
        let mut mock = MockLoginEventRepository::new();
        let start = Utc::now() - chrono::Duration::days(7);
        let end = Utc::now();

        mock.expect_get_stats().returning(|start, end| {
            Ok(LoginStats {
                total_logins: 100,
                successful_logins: 80,
                failed_logins: 20,
                unique_users: 50,
                by_event_type: HashMap::new(),
                by_device_type: HashMap::new(),
                period_start: start,
                period_end: end,
            })
        });

        let stats = mock.get_stats(start, end).await.unwrap();
        assert_eq!(stats.total_logins, 100);
        assert_eq!(stats.successful_logins, 80);
    }

    #[tokio::test]
    async fn test_mock_count_failed_by_ip() {
        let mut mock = MockLoginEventRepository::new();
        let since = Utc::now() - chrono::Duration::minutes(10);

        mock.expect_count_failed_by_ip()
            .with(eq("192.168.1.1"), always())
            .returning(|_, _| Ok(5));

        let count = mock.count_failed_by_ip("192.168.1.1", since).await.unwrap();
        assert_eq!(count, 5);
    }
}
