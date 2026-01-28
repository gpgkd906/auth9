//! Audit log repository

use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySqlPool};
use uuid::Uuid;

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    #[sqlx(json)]
    pub old_value: Option<serde_json::Value>,
    #[sqlx(json)]
    pub new_value: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
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

        sqlx::query(
            r#"
            INSERT INTO audit_logs (actor_id, action, resource_type, resource_id, old_value, new_value, ip_address, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(input.actor_id)
        .bind(&input.action)
        .bind(&input.resource_type)
        .bind(input.resource_id)
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
            query_builder = query_builder.bind(actor_id);
        }
        if let Some(ref resource_type) = query.resource_type {
            query_builder = query_builder.bind(resource_type);
        }
        if let Some(resource_id) = query.resource_id {
            query_builder = query_builder.bind(resource_id);
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
            query_builder = query_builder.bind(actor_id);
        }
        if let Some(ref resource_type) = query.resource_type {
            query_builder = query_builder.bind(resource_type);
        }
        if let Some(resource_id) = query.resource_id {
            query_builder = query_builder.bind(resource_id);
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
