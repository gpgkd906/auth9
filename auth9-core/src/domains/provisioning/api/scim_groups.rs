//! SCIM Group CRUD API handlers

use crate::domain::scim::{ScimError, ScimGroup, ScimPatchOp, ScimRequestContext};
use crate::domain::StringUuid;
use crate::domains::provisioning::api::ScimJson;
use crate::domains::provisioning::context::ProvisioningContext;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListGroupParams {
    #[serde(rename = "startIndex", default = "default_start")]
    pub start_index: i64,
    pub count: Option<i64>,
}

fn default_start() -> i64 {
    1
}

/// GET /Groups
pub async fn list_groups<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Query(params): Query<ListGroupParams>,
) -> impl IntoResponse {
    let count = params.count.unwrap_or(100).min(200);
    match state
        .scim_service()
        .list_groups(&ctx, params.start_index, count)
        .await
    {
        Ok(response) => ScimJson(response).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ScimJson(ScimError::internal(e.to_string())),
        )
            .into_response(),
    }
}

/// POST /Groups
pub async fn create_group<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    axum::Json(group): axum::Json<ScimGroup>,
) -> impl IntoResponse {
    match state.scim_service().create_group(&ctx, group).await {
        Ok(group) => (StatusCode::CREATED, ScimJson(group)).into_response(),
        Err(e) => {
            let (status, err) = match &e {
                crate::error::AppError::Conflict(_) => {
                    (StatusCode::CONFLICT, ScimError::conflict(e.to_string()))
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ScimError::internal(e.to_string()),
                ),
            };
            (status, ScimJson(err)).into_response()
        }
    }
}

/// GET /Groups/{id}
pub async fn get_group<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let group_id = match StringUuid::parse_str(&id) {
        Ok(gid) => gid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid group ID: {}", id))),
            )
                .into_response()
        }
    };

    match state.scim_service().get_group(group_id, &ctx).await {
        Ok(group) => ScimJson(group).into_response(),
        Err(e) => {
            let (status, err) = match &e {
                crate::error::AppError::NotFound(_) => {
                    (StatusCode::NOT_FOUND, ScimError::not_found(e.to_string()))
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ScimError::internal(e.to_string()),
                ),
            };
            (status, ScimJson(err)).into_response()
        }
    }
}

/// PUT /Groups/{id}
pub async fn replace_group<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
    axum::Json(group): axum::Json<ScimGroup>,
) -> impl IntoResponse {
    // For groups, replace = patch the display name
    let group_id = match StringUuid::parse_str(&id) {
        Ok(gid) => gid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid group ID: {}", id))),
            )
                .into_response()
        }
    };

    // Convert PUT to a patch operation on the display name
    let patch = ScimPatchOp {
        schemas: vec![ScimPatchOp::SCHEMA.to_string()],
        operations: vec![crate::domain::scim::ScimPatchOperation {
            op: "replace".to_string(),
            path: Some("displayName".to_string()),
            value: Some(serde_json::Value::String(group.display_name)),
        }],
    };

    match state
        .scim_service()
        .patch_group(group_id, &ctx, patch)
        .await
    {
        Ok(group) => ScimJson(group).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ScimJson(ScimError::internal(e.to_string())),
        )
            .into_response(),
    }
}

/// PATCH /Groups/{id}
pub async fn patch_group<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
    axum::Json(patch): axum::Json<ScimPatchOp>,
) -> impl IntoResponse {
    let group_id = match StringUuid::parse_str(&id) {
        Ok(gid) => gid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid group ID: {}", id))),
            )
                .into_response()
        }
    };

    match state
        .scim_service()
        .patch_group(group_id, &ctx, patch)
        .await
    {
        Ok(group) => ScimJson(group).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ScimJson(ScimError::internal(e.to_string())),
        )
            .into_response(),
    }
}

/// DELETE /Groups/{id}
pub async fn delete_group<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let group_id = match StringUuid::parse_str(&id) {
        Ok(gid) => gid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid group ID: {}", id))),
            )
                .into_response()
        }
    };

    match state.scim_service().delete_group(group_id, &ctx).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ScimJson(ScimError::internal(e.to_string())),
        )
            .into_response(),
    }
}
