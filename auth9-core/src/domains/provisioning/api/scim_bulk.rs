//! SCIM Bulk operations handler

use crate::domain::scim::{ScimBulkRequest, ScimError, ScimRequestContext};
use crate::domains::provisioning::api::ScimJson;
use crate::domains::provisioning::context::ProvisioningContext;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

/// POST /Bulk
pub async fn bulk_operations<S: ProvisioningContext>(
    State(state): State<S>,
    Extension(ctx): Extension<ScimRequestContext>,
    axum::Json(request): axum::Json<ScimBulkRequest>,
) -> impl IntoResponse {
    match state.scim_service().process_bulk(&ctx, request).await {
        Ok(response) => ScimJson(response).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ScimJson(ScimError::internal(e.to_string())),
        )
            .into_response(),
    }
}
