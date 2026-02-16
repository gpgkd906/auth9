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
    pub async fn create(&self, tenant_id: StringUuid, input: CreateActionInput) -> Result<Action> {
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
        counter!("auth9_action_operations_total", "operation" => "create", "result" => status)
            .increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "create")
            .record(duration);

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
        counter!("auth9_action_operations_total", "operation" => "list", "result" => status)
            .increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "list")
            .record(duration);
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
        counter!("auth9_action_operations_total", "operation" => "update", "result" => status)
            .increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "update")
            .record(duration);

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
        counter!("auth9_action_operations_total", "operation" => "delete", "result" => status)
            .increment(1);
        histogram!("auth9_action_operation_duration_seconds", "operation" => "delete")
            .record(duration);

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
                        strict_mode: Some(input.strict_mode),
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
                        strict_mode: input.strict_mode,
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

    /// Get a single execution log by ID
    pub async fn get_execution(
        &self,
        execution_id: StringUuid,
        tenant_id: StringUuid,
    ) -> Result<ActionExecution> {
        let execution = self
            .action_repo
            .find_execution_by_id(execution_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Execution log not found".to_string()))?;

        // Verify tenant ownership
        if execution.tenant_id != tenant_id {
            return Err(AppError::Forbidden(
                "Execution log does not belong to this tenant".to_string(),
            ));
        }

        Ok(execution)
    }

    /// Query execution logs with total count for pagination
    pub async fn query_logs(
        &self,
        tenant_id: StringUuid,
        filter: LogQueryFilter,
    ) -> Result<(Vec<ActionExecution>, i64)> {
        // If action_id is specified, verify it belongs to tenant
        if let Some(action_id) = filter.action_id {
            let _action = self.get(action_id, tenant_id).await?;
        }

        let logs = self.action_repo.query_logs(&filter).await?;
        let total = self.action_repo.count_logs(&filter).await?;

        Ok((logs, total))
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
            return Err(AppError::Validation("Script cannot be empty".to_string()));
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
    use crate::domain::{
        Action, ActionContext, ActionContextRequest, ActionContextTenant, ActionContextUser,
        ActionExecution, CreateActionInput, LogQueryFilter, StringUuid, UpdateActionInput,
        UpsertActionInput,
    };
    use crate::repository::action::MockActionRepository;
    use chrono::Utc;
    use mockall::predicate::*;

    /// Helper: build a test Action with the given tenant_id
    fn make_test_action(tenant_id: StringUuid) -> Action {
        Action {
            id: StringUuid::new_v4(),
            tenant_id,
            name: "Test Action".to_string(),
            trigger_id: "post-login".to_string(),
            script: "export default async function(ctx) { return ctx; }".to_string(),
            ..Default::default()
        }
    }

    /// Helper: build a valid CreateActionInput
    fn make_create_input() -> CreateActionInput {
        CreateActionInput {
            name: "Test Action".to_string(),
            description: Some("A test action".to_string()),
            trigger_id: "post-login".to_string(),
            script: "export default async function(ctx) { return ctx; }".to_string(),
            enabled: true,
            strict_mode: false,
            execution_order: 0,
            timeout_ms: 3000,
        }
    }

    /// Helper: build an ActionContext for testing
    fn make_test_context() -> ActionContext {
        ActionContext {
            user: ActionContextUser {
                id: StringUuid::new_v4().to_string(),
                email: "test@example.com".to_string(),
                display_name: Some("Test User".to_string()),
                mfa_enabled: false,
            },
            tenant: ActionContextTenant {
                id: StringUuid::new_v4().to_string(),
                slug: "test-tenant".to_string(),
                name: "Test Tenant".to_string(),
            },
            request: ActionContextRequest {
                ip: Some("127.0.0.1".to_string()),
                user_agent: Some("test-agent".to_string()),
                timestamp: Utc::now(),
            },
            claims: None,
        }
    }

    // ---------------------------------------------------------------
    // 1. create - success case
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_create_success() {
        let tenant_id = StringUuid::new_v4();
        let input = make_create_input();
        let expected_action = make_test_action(tenant_id);

        let mut mock = MockActionRepository::new();

        // list_by_trigger should return empty (no duplicate)
        mock.expect_list_by_trigger()
            .withf(move |tid, trig, enabled| *tid == tenant_id && trig == "post-login" && !enabled)
            .returning(|_, _, _| Ok(vec![]));

        // create should succeed
        let action_clone = expected_action.clone();
        mock.expect_create()
            .withf(move |tid, _| *tid == tenant_id)
            .returning(move |_, _| Ok(action_clone.clone()));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.create(tenant_id, input).await;

        assert!(result.is_ok());
        let action = result.unwrap();
        assert_eq!(action.tenant_id, tenant_id);
    }

    // ---------------------------------------------------------------
    // 2. create - duplicate name
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_create_duplicate_name() {
        let tenant_id = StringUuid::new_v4();
        let input = make_create_input();
        let existing = make_test_action(tenant_id);

        let mut mock = MockActionRepository::new();

        // list_by_trigger returns an action with the same name
        mock.expect_list_by_trigger()
            .returning(move |_, _, _| Ok(vec![existing.clone()]));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.create(tenant_id, input).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Conflict(_)),
            "Expected Conflict error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 3. create - invalid trigger_id
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_create_invalid_trigger() {
        let tenant_id = StringUuid::new_v4();
        let mut input = make_create_input();
        input.trigger_id = "invalid-trigger".to_string();

        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let result = service.create(tenant_id, input).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::BadRequest(_)),
            "Expected BadRequest error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 4. create - empty script
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_create_empty_script() {
        let tenant_id = StringUuid::new_v4();
        let mut input = make_create_input();
        input.script = "   ".to_string(); // whitespace-only

        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let result = service.create(tenant_id, input).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Validation(_)),
            "Expected Validation error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 5. get - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_success() {
        let tenant_id = StringUuid::new_v4();
        let action = make_test_action(tenant_id);
        let action_id = action.id;

        let mut mock = MockActionRepository::new();
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get(action_id, tenant_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, action_id);
    }

    // ---------------------------------------------------------------
    // 6. get - not found
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_not_found() {
        let tenant_id = StringUuid::new_v4();
        let action_id = StringUuid::new_v4();

        let mut mock = MockActionRepository::new();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(|_| Ok(None));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get(action_id, tenant_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "Expected NotFound error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 7. get - wrong tenant returns Forbidden
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_wrong_tenant() {
        let owner_tenant = StringUuid::new_v4();
        let other_tenant = StringUuid::new_v4();
        let action = make_test_action(owner_tenant);
        let action_id = action.id;

        let mut mock = MockActionRepository::new();
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get(action_id, other_tenant).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Forbidden(_)),
            "Expected Forbidden error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 8. list - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_list_success() {
        let tenant_id = StringUuid::new_v4();
        let actions = vec![make_test_action(tenant_id), make_test_action(tenant_id)];

        let mut mock = MockActionRepository::new();
        let actions_clone = actions.clone();
        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(move |_| Ok(actions_clone.clone()));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.list(tenant_id).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    // ---------------------------------------------------------------
    // 9. list_by_trigger - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_list_by_trigger_success() {
        let tenant_id = StringUuid::new_v4();
        let actions = vec![make_test_action(tenant_id)];

        let mut mock = MockActionRepository::new();
        let actions_clone = actions.clone();
        mock.expect_list_by_trigger()
            .withf(move |tid, trig, enabled| *tid == tenant_id && trig == "post-login" && !enabled)
            .returning(move |_, _, _| Ok(actions_clone.clone()));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.list_by_trigger(tenant_id, "post-login").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    // ---------------------------------------------------------------
    // 10. list_by_trigger - invalid trigger
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_list_by_trigger_invalid_trigger() {
        let tenant_id = StringUuid::new_v4();

        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let result = service.list_by_trigger(tenant_id, "bogus-trigger").await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::BadRequest(_)),
            "Expected BadRequest error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 11. update - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_update_success() {
        let tenant_id = StringUuid::new_v4();
        let action = make_test_action(tenant_id);
        let action_id = action.id;

        let mut mock = MockActionRepository::new();

        // get() calls find_by_id
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        // update returns updated action
        let mut updated = action.clone();
        updated.name = "Updated Name".to_string();
        let updated_clone = updated.clone();
        mock.expect_update()
            .with(eq(action_id), always())
            .returning(move |_, _| Ok(updated_clone.clone()));

        let service = ActionService::new(Arc::new(mock), None);
        let input = UpdateActionInput {
            name: Some("Updated Name".to_string()),
            ..Default::default()
        };
        let result = service.update(action_id, tenant_id, input).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Updated Name");
    }

    // ---------------------------------------------------------------
    // 12. update - not found
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_update_not_found() {
        let tenant_id = StringUuid::new_v4();
        let action_id = StringUuid::new_v4();

        let mut mock = MockActionRepository::new();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(|_| Ok(None));

        let service = ActionService::new(Arc::new(mock), None);
        let input = UpdateActionInput {
            name: Some("New Name".to_string()),
            ..Default::default()
        };
        let result = service.update(action_id, tenant_id, input).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "Expected NotFound error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 13. update - empty name validation failure
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_update_empty_name_validation() {
        let tenant_id = StringUuid::new_v4();
        let action_id = StringUuid::new_v4();

        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let input = UpdateActionInput {
            name: Some("".to_string()), // empty name fails validation (min = 1)
            ..Default::default()
        };
        let result = service.update(action_id, tenant_id, input).await;

        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // 14. delete - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_delete_success() {
        let tenant_id = StringUuid::new_v4();
        let action = make_test_action(tenant_id);
        let action_id = action.id;

        let mut mock = MockActionRepository::new();

        // get() calls find_by_id
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        mock.expect_delete()
            .with(eq(action_id))
            .returning(|_| Ok(()));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.delete(action_id, tenant_id).await;

        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // 15. delete - not found
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_delete_not_found() {
        let tenant_id = StringUuid::new_v4();
        let action_id = StringUuid::new_v4();

        let mut mock = MockActionRepository::new();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(|_| Ok(None));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.delete(action_id, tenant_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "Expected NotFound error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 16. test - without engine returns error message
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_test_action_without_engine() {
        let tenant_id = StringUuid::new_v4();
        let action = make_test_action(tenant_id);
        let action_id = action.id;
        let context = make_test_context();

        let mut mock = MockActionRepository::new();
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.test(action_id, tenant_id, context).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.success);
        assert!(response.error_message.is_some());
        assert!(response.error_message.unwrap().contains("not available"));
        assert!(response.modified_context.is_none());
    }

    // ---------------------------------------------------------------
    // get_execution - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_execution_success() {
        let tenant_id = StringUuid::new_v4();
        let execution_id = StringUuid::new_v4();
        let now = Utc::now();
        let execution = ActionExecution {
            id: execution_id,
            action_id: StringUuid::new_v4(),
            tenant_id,
            trigger_id: "post-login".to_string(),
            user_id: None,
            success: false,
            duration_ms: 123,
            error_message: Some("Test error".to_string()),
            executed_at: now,
        };

        let mut mock = MockActionRepository::new();
        let execution_clone = execution.clone();
        mock.expect_find_execution_by_id()
            .with(eq(execution_id))
            .returning(move |_| Ok(Some(execution_clone.clone())));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get_execution(execution_id, tenant_id).await;

        assert!(result.is_ok());
        let exec = result.unwrap();
        assert_eq!(exec.id, execution_id);
        assert!(!exec.success);
        assert_eq!(exec.error_message.as_deref(), Some("Test error"));
    }

    // ---------------------------------------------------------------
    // get_execution - not found
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_execution_not_found() {
        let tenant_id = StringUuid::new_v4();
        let execution_id = StringUuid::new_v4();

        let mut mock = MockActionRepository::new();
        mock.expect_find_execution_by_id()
            .with(eq(execution_id))
            .returning(|_| Ok(None));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get_execution(execution_id, tenant_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "Expected NotFound error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // get_execution - wrong tenant
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_execution_wrong_tenant() {
        let owner_tenant = StringUuid::new_v4();
        let other_tenant = StringUuid::new_v4();
        let execution_id = StringUuid::new_v4();
        let now = Utc::now();
        let execution = ActionExecution {
            id: execution_id,
            action_id: StringUuid::new_v4(),
            tenant_id: owner_tenant,
            trigger_id: "post-login".to_string(),
            user_id: None,
            success: true,
            duration_ms: 10,
            error_message: None,
            executed_at: now,
        };

        let mut mock = MockActionRepository::new();
        let execution_clone = execution.clone();
        mock.expect_find_execution_by_id()
            .with(eq(execution_id))
            .returning(move |_| Ok(Some(execution_clone.clone())));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get_execution(execution_id, other_tenant).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Forbidden(_)),
            "Expected Forbidden error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 17. query_logs - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_query_logs_success() {
        let tenant_id = StringUuid::new_v4();
        let now = Utc::now();
        let executions = vec![ActionExecution {
            id: StringUuid::new_v4(),
            action_id: StringUuid::new_v4(),
            tenant_id,
            trigger_id: "post-login".to_string(),
            user_id: None,
            success: true,
            duration_ms: 42,
            error_message: None,
            executed_at: now,
        }];

        let mut mock = MockActionRepository::new();
        let executions_clone = executions.clone();
        mock.expect_query_logs()
            .returning(move |_| Ok(executions_clone.clone()));
        mock.expect_count_logs().returning(|_| Ok(1));

        let service = ActionService::new(Arc::new(mock), None);
        let filter = LogQueryFilter::default();
        let result = service.query_logs(tenant_id, filter).await;

        assert!(result.is_ok());
        let (logs, total) = result.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(total, 1);
    }

    // ---------------------------------------------------------------
    // 18. get_stats - success
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_stats_success() {
        let tenant_id = StringUuid::new_v4();
        let action = make_test_action(tenant_id);
        let action_id = action.id;

        let mut mock = MockActionRepository::new();

        // get() calls find_by_id
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        // get_stats returns a tuple
        mock.expect_get_stats()
            .with(eq(action_id))
            .returning(|_| Ok(Some((100, 5, 23.5, 42))));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get_stats(action_id, tenant_id).await;

        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.execution_count, 100);
        assert_eq!(stats.error_count, 5);
        assert!((stats.avg_duration_ms - 23.5).abs() < f64::EPSILON);
        assert_eq!(stats.last_24h_count, 42);
    }

    // ---------------------------------------------------------------
    // 19. get_stats - not found (action exists but stats missing)
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_stats_not_found() {
        let tenant_id = StringUuid::new_v4();
        let action = make_test_action(tenant_id);
        let action_id = action.id;

        let mut mock = MockActionRepository::new();

        // Action exists
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        // Stats not found
        mock.expect_get_stats()
            .with(eq(action_id))
            .returning(|_| Ok(None));

        let service = ActionService::new(Arc::new(mock), None);
        let result = service.get_stats(action_id, tenant_id).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::NotFound(_)),
            "Expected NotFound error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 20. validate_script - empty script fails
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_validate_script_empty_fails() {
        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let result = service.validate_script("");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Validation(_)),
            "Expected Validation error, got: {:?}",
            err
        );
    }

    // ---------------------------------------------------------------
    // 21. validate_script - valid script succeeds
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_validate_script_valid_succeeds() {
        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let result = service.validate_script("export default async function(ctx) { return ctx; }");

        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // 22. execute_trigger - without engine returns context unchanged
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_execute_trigger_without_engine() {
        let tenant_id = StringUuid::new_v4();
        let context = make_test_context();
        let original_user_email = context.user.email.clone();

        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let result = service
            .execute_trigger(tenant_id, "post-login", context)
            .await;

        assert!(result.is_ok());
        let returned_ctx = result.unwrap();
        assert_eq!(returned_ctx.user.email, original_user_email);
    }

    // ---------------------------------------------------------------
    // 23. get_triggers - returns all triggers
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_get_triggers_returns_all() {
        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);
        let triggers = service.get_triggers();

        assert_eq!(triggers.len(), 6);
        assert!(triggers.contains(&ActionTrigger::PostLogin));
        assert!(triggers.contains(&ActionTrigger::PreUserRegistration));
        assert!(triggers.contains(&ActionTrigger::PostUserRegistration));
        assert!(triggers.contains(&ActionTrigger::PostChangePassword));
        assert!(triggers.contains(&ActionTrigger::PostEmailVerification));
        assert!(triggers.contains(&ActionTrigger::PreTokenRefresh));
    }

    // ---------------------------------------------------------------
    // batch_upsert - create and update mixed
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_batch_upsert_create_and_update() {
        let tenant_id = StringUuid::new_v4();
        let existing_action = make_test_action(tenant_id);
        let existing_id = existing_action.id;

        let mut mock = MockActionRepository::new();

        // For the create path: list_by_trigger returns empty (no dup)
        mock.expect_list_by_trigger()
            .returning(|_, _, _| Ok(vec![]));

        // For the create path
        mock.expect_create().returning(move |tid, input| {
            Ok(Action {
                id: StringUuid::new_v4(),
                tenant_id: tid,
                name: input.name.clone(),
                trigger_id: input.trigger_id.clone(),
                script: input.script.clone(),
                ..Default::default()
            })
        });

        // For the update path: find_by_id returns the existing action
        let action_clone = existing_action.clone();
        mock.expect_find_by_id().returning(move |id| {
            if id == existing_id {
                Ok(Some(action_clone.clone()))
            } else {
                Ok(None)
            }
        });

        // For the update path
        let tenant_for_update = tenant_id;
        mock.expect_update().returning(move |id, input| {
            Ok(Action {
                id,
                tenant_id: tenant_for_update,
                name: input.name.clone().unwrap_or_default(),
                ..Default::default()
            })
        });

        let service = ActionService::new(Arc::new(mock), None);

        let inputs = vec![
            // New action (no id)
            UpsertActionInput {
                id: None,
                name: "New Action".to_string(),
                description: None,
                trigger_id: "post-login".to_string(),
                script: "console.log('new')".to_string(),
                enabled: true,
                strict_mode: false,
                execution_order: 0,
                timeout_ms: 3000,
            },
            // Existing action (with id)
            UpsertActionInput {
                id: Some(existing_id),
                name: "Updated Action".to_string(),
                description: None,
                trigger_id: "post-login".to_string(),
                script: "console.log('updated')".to_string(),
                enabled: true,
                strict_mode: false,
                execution_order: 1,
                timeout_ms: 3000,
            },
        ];

        let result = service.batch_upsert(tenant_id, inputs).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.created.len(), 1);
        assert_eq!(response.updated.len(), 1);
        assert!(response.errors.is_empty());
    }

    // ---------------------------------------------------------------
    // batch_upsert - collects validation errors
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_batch_upsert_validation_errors() {
        let tenant_id = StringUuid::new_v4();
        let mock = MockActionRepository::new();
        let service = ActionService::new(Arc::new(mock), None);

        let inputs = vec![
            // Invalid trigger
            UpsertActionInput {
                id: None,
                name: "Bad Trigger".to_string(),
                description: None,
                trigger_id: "invalid-trigger".to_string(),
                script: "console.log('x')".to_string(),
                enabled: true,
                strict_mode: false,
                execution_order: 0,
                timeout_ms: 3000,
            },
            // Empty script
            UpsertActionInput {
                id: None,
                name: "Empty Script".to_string(),
                description: None,
                trigger_id: "post-login".to_string(),
                script: "   ".to_string(),
                enabled: true,
                strict_mode: false,
                execution_order: 0,
                timeout_ms: 3000,
            },
        ];

        let result = service.batch_upsert(tenant_id, inputs).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.created.is_empty());
        assert!(response.updated.is_empty());
        assert_eq!(response.errors.len(), 2);
    }

    // ---------------------------------------------------------------
    // query_logs - with action_id verifies tenant ownership
    // ---------------------------------------------------------------
    #[tokio::test]
    async fn test_query_logs_with_action_id_checks_tenant() {
        let tenant_id = StringUuid::new_v4();
        let other_tenant = StringUuid::new_v4();
        let action = make_test_action(other_tenant); // belongs to other_tenant
        let action_id = action.id;

        let mut mock = MockActionRepository::new();
        let action_clone = action.clone();
        mock.expect_find_by_id()
            .with(eq(action_id))
            .returning(move |_| Ok(Some(action_clone.clone())));

        let service = ActionService::new(Arc::new(mock), None);
        let filter = LogQueryFilter {
            action_id: Some(action_id),
            ..Default::default()
        };

        let result = service.query_logs(tenant_id, filter).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Forbidden(_)),
            "Expected Forbidden error, got: {:?}",
            err
        );
    }
}
