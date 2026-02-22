//! ABAC policy management APIs.

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::{AbacMode, AbacPolicyDocument, AbacSimulationInput, StringUuid};
use crate::domains::authorization::service::abac::AbacPolicyService;
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::policy::{enforce_with_state, PolicyAction, PolicyInput, ResourceScope};
use crate::repository::abac::AbacRepositoryImpl;
use crate::state::{HasDbPool, HasServices};
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAbacPolicyInput {
    #[serde(default)]
    pub change_note: Option<String>,
    pub policy: AbacPolicyDocument,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAbacPolicyInput {
    #[serde(default)]
    pub change_note: Option<String>,
    pub policy: AbacPolicyDocument,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishAbacPolicyInput {
    #[serde(default)]
    pub mode: Option<AbacMode>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RollbackAbacPolicyInput {
    #[serde(default)]
    pub mode: Option<AbacMode>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimulateAbacPolicyInput {
    #[serde(default)]
    pub policy: Option<AbacPolicyDocument>,
    pub simulation: AbacSimulationInput,
}

async fn ensure_abac_permission<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    tenant_id: Uuid,
    action: PolicyAction,
) -> Result<()> {
    enforce_with_state(
        state,
        auth,
        &PolicyInput {
            action,
            scope: ResourceScope::Tenant(StringUuid::from(tenant_id)),
        },
    )
    .await
}

fn abac_service<S: HasDbPool>(state: &S) -> AbacPolicyService<AbacRepositoryImpl> {
    AbacPolicyService::new(Arc::new(AbacRepositoryImpl::new(state.db_pool().clone())))
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/abac/policies",
    tag = "Authorization",
    params(
        ("tenant_id" = String, Path, description = "Tenant ID (UUID)")
    ),
    responses(
        (status = 200, description = "ABAC policy set and versions")
    )
)]
pub async fn list_policies<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    ensure_abac_permission(&state, &auth, tenant_id, PolicyAction::AbacRead).await?;
    let tenant_id = StringUuid::from(tenant_id);
    let payload = abac_service(&state).list_policies(tenant_id).await?;
    Ok(Json(SuccessResponse::new(json!({
        "policy_set": payload.policy_set,
        "versions": payload.versions
    }))))
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/abac/policies",
    tag = "Authorization",
    request_body = CreateAbacPolicyInput,
    params(
        ("tenant_id" = String, Path, description = "Tenant ID (UUID)")
    ),
    responses(
        (status = 200, description = "ABAC draft policy created")
    )
)]
pub async fn create_policy<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<CreateAbacPolicyInput>,
) -> Result<impl IntoResponse> {
    ensure_abac_permission(&state, &auth, tenant_id, PolicyAction::AbacWrite).await?;
    input.policy.validate()?;
    let out = abac_service(&state)
        .create_policy(
            StringUuid::from(tenant_id),
            input.policy,
            input.change_note,
            StringUuid::from(auth.user_id),
        )
        .await?;
    Ok(Json(SuccessResponse::new(json!({
        "id": out.id.to_string(),
        "policy_set_id": out.policy_set_id.to_string(),
        "version_no": out.version_no,
        "status": out.status
    }))))
}

#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/abac/policies/{version_id}",
    tag = "Authorization",
    request_body = UpdateAbacPolicyInput,
    params(
        ("tenant_id" = String, Path, description = "Tenant ID (UUID)"),
        ("version_id" = String, Path, description = "Policy version ID (UUID)")
    ),
    responses(
        (status = 200, description = "ABAC draft policy updated")
    )
)]
pub async fn update_policy<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, version_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateAbacPolicyInput>,
) -> Result<impl IntoResponse> {
    ensure_abac_permission(&state, &auth, tenant_id, PolicyAction::AbacWrite).await?;
    input.policy.validate()?;
    abac_service(&state)
        .update_policy(
            StringUuid::from(tenant_id),
            StringUuid::from(version_id),
            input.policy,
            input.change_note,
        )
        .await?;
    Ok(Json(MessageResponse::new("ABAC draft policy updated")))
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/abac/policies/{version_id}/publish",
    tag = "Authorization",
    request_body = PublishAbacPolicyInput,
    params(
        ("tenant_id" = String, Path, description = "Tenant ID (UUID)"),
        ("version_id" = String, Path, description = "Policy version ID (UUID)")
    ),
    responses(
        (status = 200, description = "ABAC policy published")
    )
)]
pub async fn publish_policy<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, version_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<PublishAbacPolicyInput>,
) -> Result<impl IntoResponse> {
    ensure_abac_permission(&state, &auth, tenant_id, PolicyAction::AbacPublish).await?;
    abac_service(&state)
        .publish_policy(
            StringUuid::from(tenant_id),
            StringUuid::from(version_id),
            input.mode.unwrap_or(AbacMode::Enforce),
        )
        .await?;
    Ok(Json(MessageResponse::new("ABAC policy published")))
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/abac/policies/{version_id}/rollback",
    tag = "Authorization",
    request_body = RollbackAbacPolicyInput,
    params(
        ("tenant_id" = String, Path, description = "Tenant ID (UUID)"),
        ("version_id" = String, Path, description = "Policy version ID (UUID)")
    ),
    responses(
        (status = 200, description = "ABAC policy rolled back")
    )
)]
pub async fn rollback_policy<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path((tenant_id, version_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<RollbackAbacPolicyInput>,
) -> Result<impl IntoResponse> {
    ensure_abac_permission(&state, &auth, tenant_id, PolicyAction::AbacPublish).await?;
    abac_service(&state)
        .rollback_policy(
            StringUuid::from(tenant_id),
            StringUuid::from(version_id),
            input.mode.unwrap_or(AbacMode::Enforce),
        )
        .await?;
    Ok(Json(MessageResponse::new("ABAC policy rolled back")))
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/abac/simulate",
    tag = "Authorization",
    request_body = SimulateAbacPolicyInput,
    params(
        ("tenant_id" = String, Path, description = "Tenant ID (UUID)")
    ),
    responses(
        (status = 200, description = "ABAC policy simulation result")
    )
)]
pub async fn simulate_policy<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<SimulateAbacPolicyInput>,
) -> Result<impl IntoResponse> {
    ensure_abac_permission(&state, &auth, tenant_id, PolicyAction::AbacSimulate).await?;
    if let Some(ref policy) = input.policy {
        policy.validate()?;
    }
    let result = abac_service(&state)
        .simulate_policy(StringUuid::from(tenant_id), input.policy, input.simulation)
        .await?;
    Ok(Json(SuccessResponse::new(result)))
}
