//! Audit log repository

use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySqlPool, Row};
use sqlx::mysql::MySqlRow;
use uuid::Uuid;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub actor_id: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl<'r> FromRow<'r, MySqlRow> for AuditLog {
    fn from_row(row: &'r MySqlRow) -> sqlx::Result<Self> {
        let id: i64 = row.try_get("id")?;
        let actor_id: Option<String> = row.try_get("actor_id")?;
        let action: String = row.try_get("action")?;
        let resource_type: String = row.try_get("resource_type")?;
        let resource_id: Option<String> = row.try_get("resource_id")?;
        
        // Handle JSON fields that might be NULL
        // We read them as Option<String> or Option<serde_json::Value> explicitly
        // If the column is JSON type in MySQL, sqlx treats it as Value if valid.
        // But the issue was UnexpectedNullError when it was NULL and mapped to Option<Value>.
        // Let's try reading as Option<serde_json::Value> directly but without the macro magic first?
        // Actually, the macro magic IS what caused the issue (likely strictness).
        // A safer way is to read as Option<sqlx::types::Json<serde_json::Value>> and unwrap
        // OR read as Option<serde_json::Value> manually.
        
        let old_value_wrapper: Option<sqlx::types::Json<serde_json::Value>> = row.try_get("old_value")?;
        let old_value = old_value_wrapper.map(|w| w.0);

        let new_value_wrapper: Option<sqlx::types::Json<serde_json::Value>> = row.try_get("new_value")?;
        let new_value = new_value_wrapper.map(|w| w.0);

        let ip_address: Option<String> = row.try_get("ip_address")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;

        Ok(AuditLog {
            id,
            actor_id,
            action,
            resource_type,
            resource_id,
            old_value,
            new_value,
            ip_address,
            created_at,
        })
    }
}

/// Input for creating an audit log entry
#[derive(Debug, Clone)]
pub struct CreateAuditLogInput {
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
}

/// Audit log query parameters
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuditLogQuery {
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub action: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn create(&self, input: &CreateAuditLogInput) -> Result<()>;
    async fn find(&self, query: &AuditLogQuery) -> Result<Vec<AuditLog>>;
    async fn count(&self, query: &AuditLogQuery) -> Result<i64>;
}

pub struct AuditRepositoryImpl {
    pool: MySqlPool,
}

impl AuditRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepository for AuditRepositoryImpl {
    async fn create(&self, input: &CreateAuditLogInput) -> Result<()> {
        let old_value = input
            .old_value
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());
        let new_value = input
            .new_value
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default());

        let actor_id = input.actor_id.map(|id| id.to_string());
        let resource_id = input.resource_id.map(|id| id.to_string());

        sqlx::query(
            r#"
            INSERT INTO audit_logs (actor_id, action, resource_type, resource_id, old_value, new_value, ip_address, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(actor_id)
        .bind(&input.action)
        .bind(&input.resource_type)
        .bind(resource_id)
        .bind(old_value)
        .bind(new_value)
        .bind(&input.ip_address)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find(&self, query: &AuditLogQuery) -> Result<Vec<AuditLog>> {
        let mut sql = String::from(
            "SELECT id, actor_id, action, resource_type, resource_id, old_value, new_value, ip_address, created_at FROM audit_logs WHERE 1=1",
        );

        if query.actor_id.is_some() {
            sql.push_str(" AND actor_id = ?");
        }
        if query.resource_type.is_some() {
            sql.push_str(" AND resource_type = ?");
        }
        if query.resource_id.is_some() {
            sql.push_str(" AND resource_id = ?");
        }
        if query.action.is_some() {
            sql.push_str(" AND action = ?");
        }
        if query.from_date.is_some() {
            sql.push_str(" AND created_at >= ?");
        }
        if query.to_date.is_some() {
            sql.push_str(" AND created_at <= ?");
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut query_builder = sqlx::query_as::<_, AuditLog>(&sql);

        if let Some(actor_id) = query.actor_id {
            query_builder = query_builder.bind(actor_id.to_string());
        }
        if let Some(ref resource_type) = query.resource_type {
            query_builder = query_builder.bind(resource_type);
        }
        if let Some(resource_id) = query.resource_id {
            query_builder = query_builder.bind(resource_id.to_string());
        }
        if let Some(ref action) = query.action {
            query_builder = query_builder.bind(action);
        }
        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);
        query_builder = query_builder.bind(limit).bind(offset);

        let logs = query_builder.fetch_all(&self.pool).await?;
        Ok(logs)
    }

    async fn count(&self, query: &AuditLogQuery) -> Result<i64> {
        let mut sql = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");

        if query.actor_id.is_some() {
            sql.push_str(" AND actor_id = ?");
        }
        if query.resource_type.is_some() {
            sql.push_str(" AND resource_type = ?");
        }
        if query.resource_id.is_some() {
            sql.push_str(" AND resource_id = ?");
        }
        if query.action.is_some() {
            sql.push_str(" AND action = ?");
        }
        if query.from_date.is_some() {
            sql.push_str(" AND created_at >= ?");
        }
        if query.to_date.is_some() {
            sql.push_str(" AND created_at <= ?");
        }

        let mut query_builder = sqlx::query_as::<_, (i64,)>(&sql);

        if let Some(actor_id) = query.actor_id {
            query_builder = query_builder.bind(actor_id.to_string());
        }
        if let Some(ref resource_type) = query.resource_type {
            query_builder = query_builder.bind(resource_type);
        }
        if let Some(resource_id) = query.resource_id {
            query_builder = query_builder.bind(resource_id.to_string());
        }
        if let Some(ref action) = query.action {
            query_builder = query_builder.bind(action);
        }
        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let (count,) = query_builder.fetch_one(&self.pool).await?;
        Ok(count)
    }
}
