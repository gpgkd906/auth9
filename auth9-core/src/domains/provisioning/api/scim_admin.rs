//! SCIM Admin API handlers (JWT-protected management endpoints)

use crate::domains::provisioning::context::ProvisioningContext;
use crate::error::AppError;
use crate::models::common::StringUuid;
use crate::models::scim::{
    CreateScimTokenInput, CreateScimTokenResponse, ScimGroupRoleMapping, ScimProvisioningLog,
    ScimTokenResponse, UpdateGroupRoleMappingsInput,
};
use crate::repository::scim_group_mapping::ScimGroupRoleMappingRepository;
use crate::repository::scim_log::ScimProvisioningLogRepository;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

/// POST /tenants/{tid}/sso/connectors/{cid}/scim/tokens - Generate a new SCIM token
pub async fn create_token<S: ProvisioningContext>(
    State(state): State<S>,
    Path((tenant_id, connector_id)): Path<(String, String)>,
    Json(input): Json<CreateScimTokenInput>,
) -> Result<impl IntoResponse, AppError> {
    let tid = StringUuid::parse_str(&tenant_id)
        .map_err(|_| AppError::BadRequest("Invalid tenant ID".to_string()))?;
    let cid = StringUuid::parse_str(&connector_id)
        .map_err(|_| AppError::BadRequest("Invalid connector ID".to_string()))?;

    let (raw_token, details) = state
        .scim_token_service()
        .create_token(tid, cid, input.description, input.expires_in_days)
        .await?;

    let response = CreateScimTokenResponse {
        token: raw_token,
        details,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /tenants/{tid}/sso/connectors/{cid}/scim/tokens - List tokens
pub async fn list_tokens<S: ProvisioningContext>(
    State(state): State<S>,
    Path((tenant_id, connector_id)): Path<(String, String)>,
) -> Result<Json<Vec<ScimTokenResponse>>, AppError> {
    let _tid = StringUuid::parse_str(&tenant_id)
        .map_err(|_| AppError::BadRequest("Invalid tenant ID".to_string()))?;
    let cid = StringUuid::parse_str(&connector_id)
        .map_err(|_| AppError::BadRequest("Invalid connector ID".to_string()))?;

    let tokens = state.scim_token_service().list_tokens(cid).await?;
    Ok(Json(tokens))
}

/// DELETE /tenants/{tid}/sso/connectors/{cid}/scim/tokens/{id} - Revoke a token
pub async fn revoke_token<S: ProvisioningContext>(
    State(state): State<S>,
    Path((_tenant_id, _connector_id, token_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    let tid = StringUuid::parse_str(&token_id)
        .map_err(|_| AppError::BadRequest("Invalid token ID".to_string()))?;

    state.scim_token_service().revoke_token(tid).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct LogListParams {
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// GET /tenants/{tid}/sso/connectors/{cid}/scim/logs - View provisioning logs
pub async fn list_logs<S: ProvisioningContext>(
    State(state): State<S>,
    Path((_tenant_id, connector_id)): Path<(String, String)>,
    Query(params): Query<LogListParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let cid = StringUuid::parse_str(&connector_id)
        .map_err(|_| AppError::BadRequest("Invalid connector ID".to_string()))?;

    let logs: Vec<ScimProvisioningLog> = state
        .scim_log_repo()
        .list_by_connector(cid, params.offset, params.limit)
        .await?;
    let total: i64 = state.scim_log_repo().count_by_connector(cid).await?;

    Ok(Json(serde_json::json!({
        "data": logs,
        "total": total,
        "offset": params.offset,
        "limit": params.limit,
    })))
}

/// GET /tenants/{tid}/sso/connectors/{cid}/scim/group-mappings
pub async fn list_group_mappings<S: ProvisioningContext>(
    State(state): State<S>,
    Path((_tenant_id, connector_id)): Path<(String, String)>,
) -> Result<Json<Vec<ScimGroupRoleMapping>>, AppError> {
    let cid = StringUuid::parse_str(&connector_id)
        .map_err(|_| AppError::BadRequest("Invalid connector ID".to_string()))?;

    let mappings: Vec<ScimGroupRoleMapping> = state
        .scim_group_mapping_repo()
        .list_by_connector(cid)
        .await?;

    Ok(Json(mappings))
}

/// PUT /tenants/{tid}/sso/connectors/{cid}/scim/group-mappings
///
/// Updates role mappings for SCIM groups. Only modifies the mapping relationship
/// (role_id, display_name) — never changes the SCIM group identity (id, scim_group_id).
pub async fn update_group_mappings<S: ProvisioningContext>(
    State(state): State<S>,
    Path((tenant_id, connector_id)): Path<(String, String)>,
    Json(input): Json<UpdateGroupRoleMappingsInput>,
) -> Result<Json<Vec<ScimGroupRoleMapping>>, AppError> {
    let tid = StringUuid::parse_str(&tenant_id)
        .map_err(|_| AppError::BadRequest("Invalid tenant ID".to_string()))?;
    let cid = StringUuid::parse_str(&connector_id)
        .map_err(|_| AppError::BadRequest("Invalid connector ID".to_string()))?;

    // Load existing mappings to preserve their IDs
    let existing: Vec<ScimGroupRoleMapping> = state
        .scim_group_mapping_repo()
        .list_by_connector(cid)
        .await?;

    // Build a lookup by scim_group_id for existing mappings
    let existing_by_group: std::collections::HashMap<String, &ScimGroupRoleMapping> = existing
        .iter()
        .map(|m| (m.scim_group_id.clone(), m))
        .collect();

    // Collect scim_group_ids from input to detect removals
    let input_group_ids: std::collections::HashSet<String> = input
        .mappings
        .iter()
        .map(|e| e.scim_group_id.clone())
        .collect();

    // Remove mappings not present in the new input
    for existing_mapping in &existing {
        if !input_group_ids.contains(&existing_mapping.scim_group_id) {
            state
                .scim_group_mapping_repo()
                .delete(existing_mapping.id)
                .await?;
        }
    }

    // Upsert each input mapping, preserving existing IDs
    for entry in &input.mappings {
        let id = existing_by_group
            .get(&entry.scim_group_id)
            .map(|m| m.id)
            .unwrap_or_else(StringUuid::new_v4);

        let mapping = ScimGroupRoleMapping {
            id,
            tenant_id: tid,
            connector_id: cid,
            scim_group_id: entry.scim_group_id.clone(),
            scim_group_display_name: entry.scim_group_display_name.clone(),
            role_id: entry.role_id,
            created_at: existing_by_group
                .get(&entry.scim_group_id)
                .map(|m| m.created_at)
                .unwrap_or_else(chrono::Utc::now),
            updated_at: chrono::Utc::now(),
        };
        state.scim_group_mapping_repo().upsert(&mapping).await?;
    }

    let mappings: Vec<ScimGroupRoleMapping> = state
        .scim_group_mapping_repo()
        .list_by_connector(cid)
        .await?;

    Ok(Json(mappings))
}
