//! Security alert repository

use crate::domain::{AlertSeverity, CreateSecurityAlertInput, SecurityAlert, SecurityAlertType, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SecurityAlertRepository: Send + Sync {
    async fn create(&self, input: &CreateSecurityAlertInput) -> Result<SecurityAlert>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<SecurityAlert>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>>;
    async fn list_unresolved(&self, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>>;
    async fn list_by_user(&self, user_id: StringUuid, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>>;
    async fn list_by_severity(&self, severity: AlertSeverity, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>>;
    async fn count(&self) -> Result<i64>;
    async fn count_unresolved(&self) -> Result<i64>;
    async fn resolve(&self, id: StringUuid, resolved_by: StringUuid) -> Result<SecurityAlert>;
    async fn delete_old(&self, days: i64) -> Result<u64>;

    /// Nullify user_id for security alerts (preserve audit trail when user is deleted)
    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64>;

    /// Delete all security alerts for a tenant (when tenant is deleted)
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
}

pub struct SecurityAlertRepositoryImpl {
    pool: MySqlPool,
}

impl SecurityAlertRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SecurityAlertRepository for SecurityAlertRepositoryImpl {
    async fn create(&self, input: &CreateSecurityAlertInput) -> Result<SecurityAlert> {
        let id = StringUuid::new_v4();
        let details_json = input
            .details
            .as_ref()
            .map(|d| serde_json::to_string(d))
            .transpose()
            .map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO security_alerts (id, user_id, tenant_id, alert_type, severity,
                                         details, created_at)
            VALUES (?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(input.tenant_id)
        .bind(&input.alert_type)
        .bind(&input.severity)
        .bind(&details_json)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create security alert")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<SecurityAlert>> {
        let alert = sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT id, user_id, tenant_id, alert_type, severity, details,
                   resolved_at, resolved_by, created_at
            FROM security_alerts
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(alert)
    }

    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>> {
        let alerts = sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT id, user_id, tenant_id, alert_type, severity, details,
                   resolved_at, resolved_by, created_at
            FROM security_alerts
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(alerts)
    }

    async fn list_unresolved(&self, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>> {
        let alerts = sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT id, user_id, tenant_id, alert_type, severity, details,
                   resolved_at, resolved_by, created_at
            FROM security_alerts
            WHERE resolved_at IS NULL
            ORDER BY
                CASE severity
                    WHEN 'critical' THEN 1
                    WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3
                    WHEN 'low' THEN 4
                END,
                created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(alerts)
    }

    async fn list_by_user(&self, user_id: StringUuid, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>> {
        let alerts = sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT id, user_id, tenant_id, alert_type, severity, details,
                   resolved_at, resolved_by, created_at
            FROM security_alerts
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

        Ok(alerts)
    }

    async fn list_by_severity(&self, severity: AlertSeverity, offset: i64, limit: i64) -> Result<Vec<SecurityAlert>> {
        let alerts = sqlx::query_as::<_, SecurityAlert>(
            r#"
            SELECT id, user_id, tenant_id, alert_type, severity, details,
                   resolved_at, resolved_by, created_at
            FROM security_alerts
            WHERE severity = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(&severity)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(alerts)
    }

    async fn count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM security_alerts")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    async fn count_unresolved(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_alerts WHERE resolved_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn resolve(&self, id: StringUuid, resolved_by: StringUuid) -> Result<SecurityAlert> {
        let result = sqlx::query(
            r#"
            UPDATE security_alerts
            SET resolved_at = NOW(), resolved_by = ?
            WHERE id = ? AND resolved_at IS NULL
            "#,
        )
        .bind(resolved_by)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Security alert not found or already resolved".to_string(),
            ));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to resolve security alert")))
    }

    async fn delete_old(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM security_alerts
            WHERE resolved_at IS NOT NULL
              AND created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("UPDATE security_alerts SET user_id = NULL WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM security_alerts WHERE tenant_id = ?")
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
    async fn test_mock_security_alert_repository() {
        let mut mock = MockSecurityAlertRepository::new();

        mock.expect_count_unresolved()
            .returning(|| Ok(5));

        let count = mock.count_unresolved().await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_mock_create() {
        let mut mock = MockSecurityAlertRepository::new();

        mock.expect_create()
            .returning(|input| {
                Ok(SecurityAlert {
                    user_id: input.user_id,
                    tenant_id: input.tenant_id,
                    alert_type: input.alert_type.clone(),
                    severity: input.severity.clone(),
                    details: input.details.clone(),
                    ..Default::default()
                })
            });

        let input = CreateSecurityAlertInput {
            user_id: Some(StringUuid::new_v4()),
            tenant_id: None,
            alert_type: SecurityAlertType::BruteForce,
            severity: AlertSeverity::High,
            details: Some(serde_json::json!({"ip": "192.168.1.1", "attempts": 10})),
        };

        let alert = mock.create(&input).await.unwrap();
        assert_eq!(alert.alert_type, SecurityAlertType::BruteForce);
        assert_eq!(alert.severity, AlertSeverity::High);
    }

    #[tokio::test]
    async fn test_mock_list_unresolved() {
        let mut mock = MockSecurityAlertRepository::new();

        mock.expect_list_unresolved()
            .with(eq(0), eq(10))
            .returning(|_, _| {
                Ok(vec![
                    SecurityAlert {
                        alert_type: SecurityAlertType::BruteForce,
                        severity: AlertSeverity::Critical,
                        ..Default::default()
                    },
                    SecurityAlert {
                        alert_type: SecurityAlertType::NewDevice,
                        severity: AlertSeverity::Medium,
                        ..Default::default()
                    },
                ])
            });

        let alerts = mock.list_unresolved(0, 10).await.unwrap();
        assert_eq!(alerts.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_resolve() {
        let mut mock = MockSecurityAlertRepository::new();
        let id = StringUuid::new_v4();
        let resolved_by = StringUuid::new_v4();

        mock.expect_resolve()
            .with(eq(id), eq(resolved_by))
            .returning(|id, resolved_by| {
                Ok(SecurityAlert {
                    id,
                    resolved_by: Some(resolved_by),
                    resolved_at: Some(chrono::Utc::now()),
                    ..Default::default()
                })
            });

        let alert = mock.resolve(id, resolved_by).await.unwrap();
        assert!(alert.resolved_at.is_some());
        assert_eq!(alert.resolved_by, Some(resolved_by));
    }

    #[tokio::test]
    async fn test_mock_list_by_severity() {
        let mut mock = MockSecurityAlertRepository::new();

        mock.expect_list_by_severity()
            .with(eq(AlertSeverity::Critical), eq(0), eq(10))
            .returning(|severity, _, _| {
                Ok(vec![SecurityAlert {
                    severity,
                    ..Default::default()
                }])
            });

        let alerts = mock.list_by_severity(AlertSeverity::Critical, 0, 10).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }
}
