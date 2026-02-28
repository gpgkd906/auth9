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
use utoipa::ToSchema;

/// Create a new action
#[utoipa::path(
    post,
    path = "/api/v1/services/{service_id}/actions",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn create_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
    Json(input): Json<CreateActionInput>,
) -> Result<Json<SuccessResponse<Action>>, AppError> {
    // Resolve service's tenant for policy enforcement
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let action = action_service.create(tenant_id, service_id, input).await?;

    Ok(Json(SuccessResponse::new(action)))
}

/// List all actions for a service
#[utoipa::path(
    get,
    path = "/api/v1/services/{service_id}/actions",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn list_actions<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
    Query(params): Query<ListActionsQuery>,
) -> Result<Json<SuccessResponse<Vec<Action>>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();

    let actions = if let Some(trigger_id) = params.trigger_id {
        action_service
            .list_by_trigger(service_id, &trigger_id)
            .await?
    } else {
        action_service.list(service_id).await?
    };

    Ok(Json(SuccessResponse::new(actions)))
}

/// Get a single action by ID
#[utoipa::path(
    get,
    path = "/api/v1/services/{service_id}/actions/{action_id}",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID"),
        ("action_id" = String, Path, description = "Action ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((service_id, action_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<Action>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let action = action_service.get(action_id, service_id).await?;

    Ok(Json(SuccessResponse::new(action)))
}

/// Update an action
#[utoipa::path(
    patch,
    path = "/api/v1/services/{service_id}/actions/{action_id}",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID"),
        ("action_id" = String, Path, description = "Action ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((service_id, action_id)): Path<(StringUuid, StringUuid)>,
    Json(input): Json<UpdateActionInput>,
) -> Result<Json<SuccessResponse<Action>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let action = action_service.update(action_id, service_id, input).await?;

    Ok(Json(SuccessResponse::new(action)))
}

/// Delete an action
#[utoipa::path(
    delete,
    path = "/api/v1/services/{service_id}/actions/{action_id}",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID"),
        ("action_id" = String, Path, description = "Action ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn delete_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((service_id, action_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<MessageResponse>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    action_service.delete(action_id, service_id).await?;

    Ok(Json(MessageResponse::new("Action deleted successfully.")))
}

/// Batch upsert actions (AI Agent friendly)
#[utoipa::path(
    post,
    path = "/api/v1/services/{service_id}/actions/batch",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn batch_upsert_actions<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
    Json(req): Json<BatchUpsertRequest>,
) -> Result<Json<SuccessResponse<BatchUpsertResponse>>, AppError> {
    if req.actions.len() > MAX_BATCH_SIZE {
        return Err(AppError::BadRequest(format!(
            "Batch size {} exceeds maximum of {}",
            req.actions.len(),
            MAX_BATCH_SIZE
        )));
    }

    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let response = action_service.batch_upsert(tenant_id, service_id, req.actions).await?;

    Ok(Json(SuccessResponse::new(response)))
}

/// Test an action with mock context
#[utoipa::path(
    post,
    path = "/api/v1/services/{service_id}/actions/{action_id}/test",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID"),
        ("action_id" = String, Path, description = "Action ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn test_action<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((service_id, action_id)): Path<(StringUuid, StringUuid)>,
    Json(req): Json<TestActionRequest>,
) -> Result<Json<SuccessResponse<TestActionResponse>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let response = action_service
        .test(action_id, service_id, req.context)
        .await?;

    Ok(Json(SuccessResponse::new(response)))
}

/// Get a single execution log by ID
#[utoipa::path(
    get,
    path = "/api/v1/services/{service_id}/actions/logs/{log_id}",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID"),
        ("log_id" = String, Path, description = "Log ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_action_log<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((service_id, log_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<ActionExecution>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let execution = action_service.get_execution(log_id, service_id).await?;

    Ok(Json(SuccessResponse::new(execution)))
}

/// Query action execution logs
#[utoipa::path(
    get,
    path = "/api/v1/services/{service_id}/actions/logs",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn query_action_logs<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
    Query(params): Query<LogQueryParams>,
) -> Result<Json<PaginatedResponse<ActionExecution>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

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
    let (logs, total) = action_service.query_logs(service_id, filter).await?;

    Ok(Json(PaginatedResponse::new(logs, page, per_page, total)))
}

/// Get action statistics
#[utoipa::path(
    get,
    path = "/api/v1/services/{service_id}/actions/{action_id}/stats",
    tag = "Integration",
    params(
        ("service_id" = String, Path, description = "Service ID"),
        ("action_id" = String, Path, description = "Action ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_action_stats<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path((service_id, action_id)): Path<(StringUuid, StringUuid)>,
) -> Result<Json<SuccessResponse<ActionStats>>, AppError> {
    let tenant_id = resolve_service_tenant(&state, service_id).await?;
    enforce(
        state.config(),
        &auth,
        &PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        },
    )?;
    ensure_service_scope(&state, &auth, service_id).await?;

    let action_service = state.action_service();
    let stats = action_service.get_stats(action_id, service_id).await?;

    Ok(Json(SuccessResponse::new(stats)))
}

/// Get all available triggers (unchanged)
#[utoipa::path(
    get,
    path = "/api/v1/actions/triggers",
    tag = "Integration",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_triggers<S: HasServices>(
    State(_state): State<S>,
    _auth: AuthUser,
) -> Result<Json<SuccessResponse<Vec<ActionTrigger>>>, AppError> {
    let triggers = ActionTrigger::all();
    Ok(Json(SuccessResponse::new(triggers)))
}

// ============================================================
// Helper functions
// ============================================================

/// Resolve service_id to its tenant_id for policy enforcement
async fn resolve_service_tenant<S: HasServices>(
    state: &S,
    service_id: StringUuid,
) -> Result<StringUuid, AppError> {
    let service = state.client_service().get(*service_id).await?;
    service
        .tenant_id
        .ok_or_else(|| AppError::NotFound("Service has no associated tenant".to_string()))
}

/// Verify the caller's token is scoped to the target service.
/// TenantAccess tokens carry an `aud` (OAuth client_id) that maps to a
/// specific service; requests targeting a different service are rejected.
async fn ensure_service_scope<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    target_service_id: StringUuid,
) -> Result<(), AppError> {
    if auth.token_type != crate::middleware::auth::TokenType::TenantAccess {
        return Ok(());
    }
    if state.config().is_platform_admin_email(&auth.email) {
        return Ok(());
    }
    if let Some(ref aud) = auth.aud {
        let token_service = state.client_service().get_by_client_id(aud).await?;
        if token_service.id != target_service_id {
            return Err(AppError::Forbidden(
                "Token is not scoped to the target service".to_string(),
            ));
        }
    }
    Ok(())
}

// ============================================================
// Request/Response Types
// ============================================================

#[derive(Debug, Deserialize)]
pub struct ListActionsQuery {
    pub trigger_id: Option<String>,
}

/// Maximum number of actions allowed in a single batch request
const MAX_BATCH_SIZE: usize = 100;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchUpsertRequest {
    pub actions: Vec<UpsertActionInput>,
}

#[derive(Debug, Deserialize, ToSchema)]
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
