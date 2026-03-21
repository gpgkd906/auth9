//! impl LoginEventRepository for LoginEventRepositoryImpl

use super::{LoginEventRepository, LoginEventRepositoryImpl};
use crate::error::Result;
use crate::models::analytics::{CreateLoginEventInput, DailyTrendPoint, LoginEvent, LoginStats};
use crate::models::common::StringUuid;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[async_trait]
impl LoginEventRepository for LoginEventRepositoryImpl {
    async fn create(&self, input: &CreateLoginEventInput) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO login_events (user_id, email, tenant_id, event_type, ip_address,
                                      user_agent, device_type, location, session_id,
                                      failure_reason, provider_alias, provider_type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW())
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
        .bind(&input.provider_alias)
        .bind(&input.provider_type)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LoginEvent>> {
        let event = sqlx::query_as::<_, LoginEvent>(
            r#"
            SELECT id, user_id, email, tenant_id, event_type, ip_address, user_agent,
                   device_type, location, session_id, failure_reason, provider_alias,
                   provider_type, created_at
            FROM login_events
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<LoginEvent>> {
        let events = sqlx::query_as::<_, LoginEvent>(
            r#"
            SELECT id, user_id, email, tenant_id, event_type, ip_address, user_agent,
                   device_type, location, session_id, failure_reason, provider_alias,
                   provider_type, created_at
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
                   device_type, location, session_id, failure_reason, provider_alias,
                   provider_type, created_at
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
                   device_type, location, session_id, failure_reason, provider_alias,
                   provider_type, created_at
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

    async fn list_by_email(&self, email: &str, offset: i64, limit: i64) -> Result<Vec<LoginEvent>> {
        let events = sqlx::query_as::<_, LoginEvent>(
            r#"
            SELECT id, user_id, email, tenant_id, event_type, ip_address, user_agent,
                   device_type, location, session_id, failure_reason, provider_alias,
                   provider_type, created_at
            FROM login_events
            WHERE email = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(email)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    async fn count_by_email(&self, email: &str) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM login_events WHERE email = ?")
            .bind(email)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn get_stats(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<LoginStats> {
        let tenant_filter = if tenant_id.is_some() {
            " AND tenant_id = ?"
        } else {
            ""
        };

        let counts_sql = format!(
            r#"
            SELECT
                COUNT(*) as total,
                CAST(COALESCE(SUM(CASE WHEN event_type IN ('success', 'social', 'federation_success') THEN 1 ELSE 0 END), 0) AS SIGNED) as successful,
                CAST(COALESCE(SUM(CASE WHEN event_type IN ('failed_password', 'failed_mfa', 'locked', 'federation_failed') THEN 1 ELSE 0 END), 0) AS SIGNED) as failed,
                COUNT(DISTINCT user_id) as unique_users
            FROM login_events
            WHERE created_at BETWEEN ? AND ?{}
            "#,
            tenant_filter
        );
        let mut q = sqlx::query_as::<_, (i64, i64, i64, i64)>(&counts_sql)
            .bind(start)
            .bind(end);
        if let Some(ref tid) = tenant_id {
            q = q.bind(tid.to_string());
        }
        let counts = q.fetch_one(&self.pool).await?;

        let event_type_sql = format!(
            r#"
            SELECT event_type, COUNT(*) as count
            FROM login_events
            WHERE created_at BETWEEN ? AND ?{}
            GROUP BY event_type
            "#,
            tenant_filter
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&event_type_sql)
            .bind(start)
            .bind(end);
        if let Some(ref tid) = tenant_id {
            q = q.bind(tid.to_string());
        }
        let event_type_counts = q.fetch_all(&self.pool).await?;

        let device_type_sql = format!(
            r#"
            SELECT device_type, COUNT(*) as count
            FROM login_events
            WHERE created_at BETWEEN ? AND ?{}
            GROUP BY device_type
            "#,
            tenant_filter
        );
        let mut q = sqlx::query_as::<_, (Option<String>, i64)>(&device_type_sql)
            .bind(start)
            .bind(end);
        if let Some(ref tid) = tenant_id {
            q = q.bind(tid.to_string());
        }
        let device_type_counts = q.fetch_all(&self.pool).await?;

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

    async fn count_failed_by_user(&self, email: &str, since: DateTime<Utc>) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM login_events
            WHERE email = ?
              AND event_type IN ('failed_password', 'failed_mfa')
              AND created_at >= ?
            "#,
        )
        .bind(email)
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

    async fn count_federation_failed_by_provider(
        &self,
        provider_alias: &str,
        since: DateTime<Utc>,
    ) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM login_events
            WHERE provider_alias = ?
              AND event_type = 'federation_failed'
              AND created_at >= ?
            "#,
        )
        .bind(provider_alias)
        .bind(since)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn get_daily_trend(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DailyTrendPoint>> {
        let tenant_filter = if tenant_id.is_some() {
            " AND tenant_id = ?"
        } else {
            ""
        };
        let sql = format!(
            r#"
            SELECT
                DATE_FORMAT(created_at, '%Y-%m-%d') as date,
                COUNT(*) as total,
                CAST(COALESCE(SUM(CASE WHEN event_type IN ('success', 'social', 'federation_success') THEN 1 ELSE 0 END), 0) AS SIGNED) as successful,
                CAST(COALESCE(SUM(CASE WHEN event_type IN ('failed_password', 'failed_mfa', 'locked', 'federation_failed') THEN 1 ELSE 0 END), 0) AS SIGNED) as failed
            FROM login_events
            WHERE created_at BETWEEN ? AND ?{}
            GROUP BY DATE_FORMAT(created_at, '%Y-%m-%d')
            ORDER BY date
            "#,
            tenant_filter
        );
        let mut q = sqlx::query_as::<_, (String, i64, i64, i64)>(&sql)
            .bind(start)
            .bind(end);
        if let Some(ref tid) = tenant_id {
            q = q.bind(tid.to_string());
        }
        let rows = q.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|(date, total, successful, failed)| DailyTrendPoint {
                date,
                total,
                successful,
                failed,
            })
            .collect())
    }
}
