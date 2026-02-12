//! Action management service
//!
//! Provides CRUD operations and management functions for Auth9 Actions

use crate::domain::{
    Action, ActionContext, ActionExecution, ActionStats, ActionTrigger, BatchError,
    BatchUpsertResponse, CreateActionInput, LogQueryFilter, StringUuid, TestActionResponse,
    UpdateActionInput, UpsertActionInput,
};
use crate::error::{AppError, Result};
use crate::repository::ActionRepository;
use crate::service::ActionEngine;
use metrics::{counter, histogram};
use std::sync::Arc;
use std::time::Instant;
use validator::Validate;

pub struct ActionService<R: ActionRepository> {
    action_repo: Arc<R>,
    action_engine: Option<Arc<ActionEngine<R>>>,
}

impl<R: ActionRepository + 'static> ActionService<R> {
    /// Create a new ActionService
    pub fn new(action_repo: Arc<R>, action_engine: Option<Arc<ActionEngine<R>>>) -> Self {
        Self {
            action_repo,
            action_engine,
        }
    }

    /// Create a new action
    pub async fn create(
        &self,
        tenant_id: StringUuid,
        input: CreateActionInput,
    ) -> Result<Action> {
        let start = Instant::now();

        // Validate input
        input.validate()?;

        // Validate trigger_id
        ActionTrigger::from_str(&input.trigger_id)?;

        // Validate script by attempting to compile it
        self.validate_script(&input.script)?;

        // Check for duplicate name within tenant and trigger
        let existing = self
            .action_repo
            .list_by_trigger(tenant_id, &input.trigger_id, false)
            .await?;

        if existing.iter().any(|a| a.name == input.name) {
            return Err(AppError::Conflict(format!(
                "Action with name '{}' already exists for trigger '{}'",
                input.name, input.trigger_id
            )));
        }

        // Create action
        let result = self.action_repo.create(tenant_id, &input).await;

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        counter!("auth9_action_operations_total", "operation" => "create", "result" => status).increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "create").record(duration);

        result
    }

    /// Get action by ID
    pub async fn get(&self, id: StringUuid, tenant_id: StringUuid) -> Result<Action> {
        let action = self
            .action_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Action not found".to_string()))?;

        // Verify tenant ownership
        if action.tenant_id != tenant_id {
            return Err(AppError::Forbidden(
                "Action does not belong to this tenant".to_string(),
            ));
        }

        Ok(action)
    }

    /// List all actions for a tenant
    pub async fn list(&self, tenant_id: StringUuid) -> Result<Vec<Action>> {
        let start = Instant::now();
        let result = self.action_repo.list_by_tenant(tenant_id).await;
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        counter!("auth9_action_operations_total", "operation" => "list", "result" => status).increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "list").record(duration);
        result
    }

    /// List actions by trigger
    pub async fn list_by_trigger(
        &self,
        tenant_id: StringUuid,
        trigger_id: &str,
    ) -> Result<Vec<Action>> {
        // Validate trigger_id
        ActionTrigger::from_str(trigger_id)?;

        self.action_repo
            .list_by_trigger(tenant_id, trigger_id, false)
            .await
    }

    /// Update an action
    pub async fn update(
        &self,
        id: StringUuid,
        tenant_id: StringUuid,
        input: UpdateActionInput,
    ) -> Result<Action> {
        let start = Instant::now();

        // Validate input
        input.validate()?;

        // Verify action exists and belongs to tenant
        let _action = self.get(id, tenant_id).await?;

        // If script is being updated, validate it
        if let Some(ref script) = input.script {
            self.validate_script(script)?;
        }

        // Update action
        let result = self.action_repo.update(id, &input).await;

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        counter!("auth9_action_operations_total", "operation" => "update", "result" => status).increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "update").record(duration);

        result
    }

    /// Delete an action
    pub async fn delete(&self, id: StringUuid, tenant_id: StringUuid) -> Result<()> {
        let start = Instant::now();

        // Verify action exists and belongs to tenant
        let _action = self.get(id, tenant_id).await?;

        let result = self.action_repo.delete(id).await;

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        let status = if result.is_ok() { "success" } else { "error" };
        counter!("auth9_action_operations_total", "operation" => "delete", "result" => status).increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "delete").record(duration);

        result
    }

    /// Batch upsert actions (create or update)
    pub async fn batch_upsert(
        &self,
        tenant_id: StringUuid,
        inputs: Vec<UpsertActionInput>,
    ) -> Result<BatchUpsertResponse> {
        let mut created = Vec::new();
        let mut updated = Vec::new();
        let mut errors = Vec::new();

        for (index, input) in inputs.into_iter().enumerate() {
            // Validate input
            if let Err(e) = input.validate() {
                errors.push(BatchError {
                    input_index: index,
                    name: input.name.clone(),
                    error: e.to_string(),
                });
                continue;
            }

            // Validate trigger_id
            if let Err(e) = ActionTrigger::from_str(&input.trigger_id) {
                errors.push(BatchError {
                    input_index: index,
                    name: input.name.clone(),
                    error: e.to_string(),
                });
                continue;
            }

            // Validate script
            if let Err(e) = self.validate_script(&input.script) {
                errors.push(BatchError {
                    input_index: index,
                    name: input.name.clone(),
                    error: e.to_string(),
                });
                continue;
            }

            // Upsert
            match input.id {
                Some(id) => {
                    // Update existing action
                    let update_input = UpdateActionInput {
                        name: Some(input.name.clone()),
                        description: input.description.clone(),
                        script: Some(input.script),
                        enabled: Some(input.enabled),
                        execution_order: Some(input.execution_order),
                        timeout_ms: Some(input.timeout_ms),
                    };

                    match self.update(id, tenant_id, update_input).await {
                        Ok(action) => updated.push(action),
                        Err(e) => errors.push(BatchError {
                            input_index: index,
                            name: input.name,
                            error: e.to_string(),
                        }),
                    }
                }
                None => {
                    // Create new action
                    let create_input = CreateActionInput {
                        name: input.name.clone(),
                        description: input.description,
                        trigger_id: input.trigger_id,
                        script: input.script,
                        enabled: input.enabled,
                        execution_order: input.execution_order,
                        timeout_ms: input.timeout_ms,
                    };

                    match self.create(tenant_id, create_input).await {
                        Ok(action) => created.push(action),
                        Err(e) => errors.push(BatchError {
                            input_index: index,
                            name: input.name,
                            error: e.to_string(),
                        }),
                    }
                }
            }
        }

        Ok(BatchUpsertResponse {
            created,
            updated,
            errors,
        })
    }

    /// Test an action with mock context
    pub async fn test(
        &self,
        id: StringUuid,
        tenant_id: StringUuid,
        context: ActionContext,
    ) -> Result<TestActionResponse> {
        // Verify action exists and belongs to tenant
        let action = self.get(id, tenant_id).await?;

        // Check if action engine is available
        let Some(ref action_engine) = self.action_engine else {
            return Ok(TestActionResponse {
                success: false,
                duration_ms: 0,
                modified_context: None,
                error_message: Some("Action engine not available (test mode)".to_string()),
                console_logs: Vec::new(),
            });
        };

        // Test execution
        match action_engine.test_action(&action, context).await {
            Ok((modified_context, duration_ms, console_logs)) => Ok(TestActionResponse {
                success: true,
                duration_ms,
                modified_context: Some(modified_context),
                error_message: None,
                console_logs,
            }),
            Err(e) => Ok(TestActionResponse {
                success: false,
                duration_ms: 0,
                modified_context: None,
                error_message: Some(e.to_string()),
                console_logs: Vec::new(),
            }),
        }
    }

    /// Query execution logs
    pub async fn query_logs(
        &self,
        tenant_id: StringUuid,
        filter: LogQueryFilter,
    ) -> Result<Vec<ActionExecution>> {
        // If action_id is specified, verify it belongs to tenant
        if let Some(action_id) = filter.action_id {
            let _action = self.get(action_id, tenant_id).await?;
        }

        self.action_repo.query_logs(&filter).await
    }

    /// Get action statistics
    pub async fn get_stats(&self, id: StringUuid, tenant_id: StringUuid) -> Result<ActionStats> {
        // Verify action exists and belongs to tenant
        let _action = self.get(id, tenant_id).await?;

        let stats = self
            .action_repo
            .get_stats(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Action stats not found".to_string()))?;

        Ok(ActionStats {
            execution_count: stats.0,
            error_count: stats.1,
            avg_duration_ms: stats.2,
            last_24h_count: stats.3,
        })
    }

    /// Validate script by attempting to compile it
    fn validate_script(&self, script: &str) -> Result<()> {
        // Basic validation: check script is not empty
        if script.trim().is_empty() {
            return Err(AppError::Validation(
                "Script cannot be empty".to_string(),
            ));
        }

        // TODO: Add more sophisticated validation
        // For now, we'll let the ActionEngine handle compilation errors at runtime

        Ok(())
    }

    /// Execute actions for a specific trigger (delegates to ActionEngine)
    ///
    /// Returns the (potentially modified) ActionContext after all actions have run.
    /// If no ActionEngine is configured, returns the context unchanged.
    pub async fn execute_trigger(
        &self,
        tenant_id: StringUuid,
        trigger_id: &str,
        context: ActionContext,
    ) -> Result<ActionContext> {
        match &self.action_engine {
            Some(engine) => engine.execute_trigger(tenant_id, trigger_id, context).await,
            None => Ok(context),
        }
    }

    /// Get all available triggers
    pub fn get_triggers(&self) -> Vec<ActionTrigger> {
        ActionTrigger::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::action::MockActionRepository;

    #[test]
    fn test_action_service_creation() {
        let mock_repo = Arc::new(MockActionRepository::new());
        // Use None for tests to avoid slow V8 initialization
        let _service = ActionService::new(mock_repo, None);
    }
}
