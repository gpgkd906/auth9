//! Action repository

use crate::domain::{
    Action, ActionExecution, CreateActionInput, LogQueryFilter, StringUuid, UpdateActionInput,
};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ActionRepository: Send + Sync {
    async fn create(&self, tenant_id: StringUuid, input: &CreateActionInput) -> Result<Action>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Action>>;
    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<Action>>;
    async fn list_by_trigger(
        &self,
        tenant_id: StringUuid,
        trigger_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<Action>>;
    async fn update(&self, id: StringUuid, input: &UpdateActionInput) -> Result<Action>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
    async fn record_execution(
        &self,
        action_id: StringUuid,
        tenant_id: StringUuid,
        trigger_id: String,
        user_id: Option<StringUuid>,
        success: bool,
        duration_ms: i32,
        error: Option<String>,
    ) -> Result<()>;
    async fn update_execution_stats(
        &self,
        id: StringUuid,
        success: bool,
        error: Option<String>,
    ) -> Result<()>;
    async fn find_execution_by_id(&self, id: StringUuid) -> Result<Option<ActionExecution>>;
    async fn query_logs(&self, filter: &LogQueryFilter) -> Result<Vec<ActionExecution>>;
    async fn count_logs(&self, filter: &LogQueryFilter) -> Result<i64>;
    async fn get_stats(&self, action_id: StringUuid) -> Result<Option<(i64, i64, f64, i64)>>;
}

pub struct ActionRepositoryImpl {
    pool: MySqlPool,
}

impl ActionRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ActionRepository for ActionRepositoryImpl {
    async fn create(&self, tenant_id: StringUuid, input: &CreateActionInput) -> Result<Action> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO actions (id, tenant_id, name, description, trigger_id, script,
                                 enabled, strict_mode, execution_order, timeout_ms,
                                 created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.trigger_id)
        .bind(&input.script)
        .bind(input.enabled)
        .bind(input.strict_mode)
        .bind(input.execution_order)
        .bind(input.timeout_ms)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create action")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Action>> {
        let action = sqlx::query_as::<_, Action>(
            r#"
            SELECT id, tenant_id, name, description, trigger_id, script, enabled,
                   strict_mode, execution_order, timeout_ms, last_executed_at, execution_count,
                   error_count, last_error, created_at, updated_at
            FROM actions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(action)
    }

    async fn list_by_tenant(&self, tenant_id: StringUuid) -> Result<Vec<Action>> {
        let actions = sqlx::query_as::<_, Action>(
            r#"
            SELECT id, tenant_id, name, description, trigger_id, script, enabled,
                   strict_mode, execution_order, timeout_ms, last_executed_at, execution_count,
                   error_count, last_error, created_at, updated_at
            FROM actions
            WHERE tenant_id = ?
            ORDER BY trigger_id, execution_order, created_at
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(actions)
    }

    async fn list_by_trigger(
        &self,
        tenant_id: StringUuid,
        trigger_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<Action>> {
        let query = if enabled_only {
            r#"
            SELECT id, tenant_id, name, description, trigger_id, script, enabled,
                   strict_mode, execution_order, timeout_ms, last_executed_at, execution_count,
                   error_count, last_error, created_at, updated_at
            FROM actions
            WHERE tenant_id = ? AND trigger_id = ? AND enabled = TRUE
            ORDER BY execution_order, created_at
            "#
        } else {
            r#"
            SELECT id, tenant_id, name, description, trigger_id, script, enabled,
                   strict_mode, execution_order, timeout_ms, last_executed_at, execution_count,
                   error_count, last_error, created_at, updated_at
            FROM actions
            WHERE tenant_id = ? AND trigger_id = ?
            ORDER BY execution_order, created_at
            "#
        };

        let actions = sqlx::query_as::<_, Action>(query)
            .bind(tenant_id)
            .bind(trigger_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(actions)
    }

    async fn update(&self, id: StringUuid, input: &UpdateActionInput) -> Result<Action> {
        // Build dynamic update query
        let mut updates = Vec::new();
        let mut bindings: Vec<String> = Vec::new();

        if let Some(ref name) = input.name {
            updates.push("name = ?");
            bindings.push(name.clone());
        }
        if input.description.is_some() {
            updates.push("description = ?");
            bindings.push(input.description.clone().unwrap_or_default());
        }
        if let Some(ref script) = input.script {
            updates.push("script = ?");
            bindings.push(script.clone());
        }
        if let Some(enabled) = input.enabled {
            updates.push("enabled = ?");
            bindings.push((enabled as i32).to_string());
        }
        if let Some(strict_mode) = input.strict_mode {
            updates.push("strict_mode = ?");
            bindings.push((strict_mode as i32).to_string());
        }
        if let Some(execution_order) = input.execution_order {
            updates.push("execution_order = ?");
            bindings.push(execution_order.to_string());
        }
        if let Some(timeout_ms) = input.timeout_ms {
            updates.push("timeout_ms = ?");
            bindings.push(timeout_ms.to_string());
        }

        if updates.is_empty() {
            // No updates, just return existing
            return self
                .find_by_id(id)
                .await?
                .ok_or_else(|| AppError::NotFound("Action not found".to_string()));
        }

        updates.push("updated_at = NOW()");

        let query_str = format!("UPDATE actions SET {} WHERE id = ?", updates.join(", "));

        let mut query = sqlx::query(&query_str);
        for binding in bindings {
            query = query.bind(binding);
        }
        query = query.bind(id);

        query.execute(&self.pool).await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Action not found".to_string()))
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        sqlx::query("DELETE FROM actions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM actions WHERE tenant_id = ?")
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    async fn record_execution(
        &self,
        action_id: StringUuid,
        tenant_id: StringUuid,
        trigger_id: String,
        user_id: Option<StringUuid>,
        success: bool,
        duration_ms: i32,
        error: Option<String>,
    ) -> Result<()> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO action_executions (id, action_id, tenant_id, trigger_id, user_id,
                                            success, duration_ms, error_message, executed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, NOW())
            "#,
        )
        .bind(id)
        .bind(action_id)
        .bind(tenant_id)
        .bind(&trigger_id)
        .bind(user_id)
        .bind(success)
        .bind(duration_ms)
        .bind(&error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_execution_stats(
        &self,
        id: StringUuid,
        success: bool,
        error: Option<String>,
    ) -> Result<()> {
        if success {
            sqlx::query(
                r#"
                UPDATE actions
                SET execution_count = execution_count + 1,
                    last_executed_at = NOW()
                WHERE id = ?
                "#,
            )
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE actions
                SET execution_count = execution_count + 1,
                    error_count = error_count + 1,
                    last_executed_at = NOW(),
                    last_error = ?
                WHERE id = ?
                "#,
            )
            .bind(&error)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn find_execution_by_id(&self, id: StringUuid) -> Result<Option<ActionExecution>> {
        let execution = sqlx::query_as::<_, ActionExecution>(
            r#"
            SELECT id, action_id, tenant_id, trigger_id, user_id, success,
                   duration_ms, error_message, executed_at
            FROM action_executions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(execution)
    }

    async fn query_logs(&self, filter: &LogQueryFilter) -> Result<Vec<ActionExecution>> {
        let mut query_str = String::from(
            r#"
            SELECT id, action_id, tenant_id, trigger_id, user_id, success,
                   duration_ms, error_message, executed_at
            FROM action_executions
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();
        if filter.action_id.is_some() {
            conditions.push("action_id = ?");
        }
        if filter.user_id.is_some() {
            conditions.push("user_id = ?");
        }
        if filter.success.is_some() {
            conditions.push("success = ?");
        }
        if filter.from.is_some() {
            conditions.push("executed_at >= ?");
        }
        if filter.to.is_some() {
            conditions.push("executed_at <= ?");
        }

        for condition in conditions {
            query_str.push_str(&format!(" AND {}", condition));
        }

        query_str.push_str(" ORDER BY executed_at DESC");

        if let Some(limit) = filter.limit {
            query_str.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            query_str.push_str(&format!(" OFFSET {}", offset));
        }

        let mut query = sqlx::query_as::<_, ActionExecution>(&query_str);

        if let Some(action_id) = filter.action_id {
            query = query.bind(action_id);
        }
        if let Some(user_id) = filter.user_id {
            query = query.bind(user_id);
        }
        if let Some(success) = filter.success {
            query = query.bind(success);
        }
        if let Some(from) = filter.from {
            query = query.bind(from);
        }
        if let Some(to) = filter.to {
            query = query.bind(to);
        }

        let executions = query.fetch_all(&self.pool).await?;

        Ok(executions)
    }

    async fn count_logs(&self, filter: &LogQueryFilter) -> Result<i64> {
        let mut query_str = String::from(
            r#"
            SELECT COUNT(*) as cnt
            FROM action_executions
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();
        if filter.action_id.is_some() {
            conditions.push("action_id = ?");
        }
        if filter.user_id.is_some() {
            conditions.push("user_id = ?");
        }
        if filter.success.is_some() {
            conditions.push("success = ?");
        }
        if filter.from.is_some() {
            conditions.push("executed_at >= ?");
        }
        if filter.to.is_some() {
            conditions.push("executed_at <= ?");
        }

        for condition in conditions {
            query_str.push_str(&format!(" AND {}", condition));
        }

        let mut query = sqlx::query_scalar::<_, i64>(&query_str);

        if let Some(action_id) = filter.action_id {
            query = query.bind(action_id);
        }
        if let Some(user_id) = filter.user_id {
            query = query.bind(user_id);
        }
        if let Some(success) = filter.success {
            query = query.bind(success);
        }
        if let Some(from) = filter.from {
            query = query.bind(from);
        }
        if let Some(to) = filter.to {
            query = query.bind(to);
        }

        let count = query.fetch_one(&self.pool).await?;

        Ok(count)
    }

    async fn get_stats(&self, action_id: StringUuid) -> Result<Option<(i64, i64, f64, i64)>> {
        let result = sqlx::query_as::<_, (i64, i64, f64, i64)>(
            r#"
            SELECT
                execution_count,
                error_count,
                COALESCE(
                    CAST((SELECT AVG(duration_ms)
                     FROM action_executions
                     WHERE action_id = ?) AS DOUBLE),
                    0
                ) as avg_duration_ms,
                COALESCE(
                    (SELECT COUNT(*)
                     FROM action_executions
                     WHERE action_id = ?
                     AND executed_at >= DATE_SUB(NOW(), INTERVAL 24 HOUR)),
                    0
                ) as last_24h_count
            FROM actions
            WHERE id = ?
            "#,
        )
        .bind(action_id)
        .bind(action_id)
        .bind(action_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_repository_trait_is_mockable() {
        // This test ensures MockActionRepository can be created
        let _mock = MockActionRepository::new();
    }
}
