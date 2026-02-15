//! Action management API handlers
//!
//! Provides REST API endpoints for managing Auth9 Actions

use crate::api::{MessageResponse, PaginatedResponse, SuccessResponse};
use crate::domain::{
    Action, ActionContext, ActionExecution, ActionStats, ActionTrigger, BatchUpsertResponse,
    CreateActionInput, LogQueryFilter, StringUuid, TestActionResponse, UpdateActionInput,
    UpsertActionInput,
};
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce, PolicyAction, PolicyInput, ResourceScope};
use crate::state::HasServices;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// Create a new action
pub async fn create_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
    Json(input): Json<CreateActionInput>,
) -> Result<Json<SuccessResponse<Action>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionWrite,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_service = state.action_service();
    let action = action_service.create(tenant_id, input).await?;

    Ok(Json(SuccessResponse::new(action)))
}

/// List all actions for a tenant
pub async fn list_actions<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
    Query(params): Query<ListActionsQuery>,
) -> Result<Json<SuccessResponse<Vec<Action>>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionRead,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_service = state.action_service();

    let actions = if let Some(trigger_id) = params.trigger_id {
        action_service
            .list_by_trigger(tenant_id, &trigger_id)
            .await?
    } else {
        action_service.list(tenant_id).await?
    };

    Ok(Json(SuccessResponse::new(actions)))
}

/// Get a single action by ID
pub async fn get_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, action_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<Action>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionRead,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_service = state.action_service();
    let action = action_service.get(action_id, tenant_id).await?;

    // Verify the action belongs to the tenant
    if action.tenant_id != tenant_id {
        return Err(AppError::NotFound("Action not found".to_string()));
    }

    Ok(Json(SuccessResponse::new(action)))
}

/// Update an action
pub async fn update_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, action_id)): Path<(StringUuid, StringUuid)>,
    Json(input): Json<UpdateActionInput>,
) -> Result<Json<SuccessResponse<Action>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionWrite,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    // Verify the action belongs to the tenant
    let action_service = state.action_service();
    let existing = action_service.get(action_id, tenant_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Action not found".to_string()));
    }

    let action = action_service.update(action_id, tenant_id, input).await?;

    Ok(Json(SuccessResponse::new(action)))
}

/// Delete an action
pub async fn delete_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, action_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<MessageResponse>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionWrite,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    // Verify the action belongs to the tenant
    let action_service = state.action_service();
    let existing = action_service.get(action_id, tenant_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Action not found".to_string()));
    }

    action_service.delete(action_id, tenant_id).await?;

    Ok(Json(MessageResponse::new("Action deleted successfully.")))
}

/// Batch upsert actions (AI Agent friendly)
pub async fn batch_upsert_actions<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
    Json(req): Json<BatchUpsertRequest>,
) -> Result<Json<SuccessResponse<BatchUpsertResponse>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionWrite,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_service = state.action_service();
    let response = action_service.batch_upsert(tenant_id, req.actions).await?;

    Ok(Json(SuccessResponse::new(response)))
}

/// Test an action with mock context
pub async fn test_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, action_id)): Path<(StringUuid, StringUuid)>,
    Json(req): Json<TestActionRequest>,
) -> Result<Json<SuccessResponse<TestActionResponse>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionWrite,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_service = state.action_service();
    let response = action_service.test(action_id, tenant_id, req.context).await?;

    Ok(Json(SuccessResponse::new(response)))
}

/// Get a single execution log by ID
pub async fn get_action_log<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, log_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<ActionExecution>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionRead,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_service = state.action_service();
    let execution = action_service.get_execution(log_id, tenant_id).await?;

    Ok(Json(SuccessResponse::new(execution)))
}

/// Query action execution logs
pub async fn query_action_logs<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<StringUuid>,
    Query(params): Query<LogQueryParams>,
) -> Result<Json<PaginatedResponse<ActionExecution>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionRead,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    let action_id = if let Some(ref id_str) = params.action_id {
        Some(
            StringUuid::parse_str(id_str)
                .map_err(|_| AppError::BadRequest("Invalid action ID".to_string()))?,
        )
    } else {
        None
    };

    let user_id = if let Some(ref id_str) = params.user_id {
        Some(
            StringUuid::parse_str(id_str)
                .map_err(|_| AppError::BadRequest("Invalid user ID".to_string()))?,
        )
    } else {
        None
    };

    let per_page = params.limit.unwrap_or(50) as i64;
    let offset = params.offset.unwrap_or(0) as i64;
    let page = offset / per_page + 1;

    let filter = LogQueryFilter {
        action_id,
        user_id,
        success: params.success,
        from: params.from,
        to: params.to,
        limit: params.limit,
        offset: params.offset,
    };

    let action_service = state.action_service();
    let (logs, total) = action_service.query_logs(tenant_id, filter).await?;

    Ok(Json(PaginatedResponse::new(logs, page, per_page, total)))
}

/// Get action statistics
pub async fn get_action_stats<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, action_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<ActionStats>>, AppError> {
    enforce(state.config(), &auth, &PolicyInput {
        action: PolicyAction::ActionRead,
        scope: ResourceScope::Tenant(tenant_id),
    })?;

    // Verify the action belongs to the tenant
    let action_service = state.action_service();
    let existing = action_service.get(action_id, tenant_id).await?;
    if existing.tenant_id != tenant_id {
        return Err(AppError::NotFound("Action not found".to_string()));
    }

    let stats = action_service.get_stats(action_id, tenant_id).await?;

    Ok(Json(SuccessResponse::new(stats)))
}

/// Get all available triggers
pub async fn get_triggers<S: HasServices>(
    State(_state): State<S>,
    _auth: AuthUser,
) -> Result<Json<SuccessResponse<Vec<ActionTrigger>>>, AppError> {
    let triggers = ActionTrigger::all();
    Ok(Json(SuccessResponse::new(triggers)))
}

// ============================================================
// Request/Response Types
// ============================================================

#[derive(Debug, Deserialize)]
pub struct ListActionsQuery {
    pub trigger_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchUpsertRequest {
    pub actions: Vec<UpsertActionInput>,
}

#[derive(Debug, Deserialize)]
pub struct TestActionRequest {
    pub context: ActionContext,
}

#[derive(Debug, Deserialize)]
pub struct LogQueryParams {
    pub action_id: Option<String>,
    pub user_id: Option<String>,
    pub success: Option<bool>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

