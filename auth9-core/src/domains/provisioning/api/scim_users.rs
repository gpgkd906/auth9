//! SCIM User CRUD API handlers

use crate::domain::scim::{ScimError, ScimRequestContext, ScimUser};
use crate::domain::StringUuid;
use crate::domains::provisioning::api::ScimJson;
use crate::domains::provisioning::context::ProvisioningContext;
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ListParams {
    pub filter: Option<String>,
    #[serde(rename = "startIndex", default = "default_start")]
    pub start_index: i64,
    pub count: Option<i64>,
}

fn default_start() -> i64 {
    1
}

/// GET /Users - List users with optional filter
pub async fn list_users<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let count = params.count.unwrap_or(100).min(200);
    match state
        .scim_service()
        .list_users(&ctx, params.filter.as_deref(), params.start_index, count)
        .await
    {
        Ok(response) => ScimJson(response).into_response(),
        Err(e) => {
            let err = ScimError::internal(e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR, ScimJson(err)).into_response()
        }
    }
}

/// POST /Users - Create user
pub async fn create_user<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    axum::Json(scim_user): axum::Json<ScimUser>,
) -> impl IntoResponse {
    match state.scim_service().create_user(&ctx, scim_user).await {
        Ok(user) => (StatusCode::CREATED, ScimJson(user)).into_response(),
        Err(e) => {
            let (status, err) = match &e {
                crate::error::AppError::Conflict(_) => {
                    (StatusCode::CONFLICT, ScimError::conflict(e.to_string()))
                }
                crate::error::AppError::BadRequest(_) | crate::error::AppError::Validation(_) => {
                    (StatusCode::BAD_REQUEST, ScimError::bad_request(e.to_string()))
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

/// GET /Users/{id} - Get user
pub async fn get_user<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match StringUuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid user ID: {}", id))),
            )
                .into_response()
        }
    };

    match state.scim_service().get_user(user_id, &ctx).await {
        Ok(user) => ScimJson(user).into_response(),
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

/// PUT /Users/{id} - Replace user
pub async fn replace_user<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
    axum::Json(scim_user): axum::Json<ScimUser>,
) -> impl IntoResponse {
    let user_id = match StringUuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid user ID: {}", id))),
            )
                .into_response()
        }
    };

    match state
        .scim_service()
        .replace_user(user_id, &ctx, scim_user)
        .await
    {
        Ok(user) => ScimJson(user).into_response(),
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

/// PATCH /Users/{id} - Patch user
pub async fn patch_user<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
    axum::Json(patch): axum::Json<crate::domain::scim::ScimPatchOp>,
) -> impl IntoResponse {
    let user_id = match StringUuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid user ID: {}", id))),
            )
                .into_response()
        }
    };

    match state
        .scim_service()
        .patch_user(user_id, &ctx, patch)
        .await
    {
        Ok(user) => ScimJson(user).into_response(),
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

/// DELETE /Users/{id} - Delete (deactivate) user
pub async fn delete_user<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match StringUuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                ScimJson(ScimError::bad_request(format!("Invalid user ID: {}", id))),
            )
                .into_response()
        }
    };

    match state.scim_service().delete_user(user_id, &ctx).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
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
