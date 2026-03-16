//! SAML Application (IdP outbound) REST API handlers

use crate::error::Result;
use crate::http_support::{MessageResponse, SuccessResponse};
use crate::middleware::auth::AuthUser;
use crate::models::common::StringUuid;
use crate::models::saml_application::{
    CreateSamlApplicationInput, SamlApplicationResponse, UpdateSamlApplicationInput,
};
use crate::policy::{self, PolicyAction, PolicyInput, ResourceScope};
use crate::state::HasServices;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/saml-apps",
    tag = "SAML Applications",
    responses(
        (status = 200, description = "List SAML applications")
    )
)]
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    _headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<SuccessResponse<Vec<SamlApplicationResponse>>>> {
    ensure_tenant_access(&state, &auth, tenant_id).await?;
    let apps = state
        .saml_application_service()
        .list(StringUuid::from(tenant_id))
        .await?;
    Ok(Json(SuccessResponse::new(apps)))
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants/{tenant_id}/saml-apps",
    tag = "SAML Applications",
    responses(
        (status = 200, description = "Created SAML application")
    )
)]
pub async fn create<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    _headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<CreateSamlApplicationInput>,
) -> Result<Json<SuccessResponse<SamlApplicationResponse>>> {
    ensure_tenant_access(&state, &auth, tenant_id).await?;
    let app = state
        .saml_application_service()
        .create(StringUuid::from(tenant_id), input)
        .await?;
    Ok(Json(SuccessResponse::new(app)))
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/saml-apps/{app_id}",
    tag = "SAML Applications",
    responses(
        (status = 200, description = "SAML application details")
    )
)]
pub async fn get<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    _headers: HeaderMap,
    Path((tenant_id, app_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SuccessResponse<SamlApplicationResponse>>> {
    ensure_tenant_access(&state, &auth, tenant_id).await?;
    let app = state
        .saml_application_service()
        .get(StringUuid::from(tenant_id), StringUuid::from(app_id))
        .await?;
    Ok(Json(SuccessResponse::new(app)))
}

#[utoipa::path(
    put,
    path = "/api/v1/tenants/{tenant_id}/saml-apps/{app_id}",
    tag = "SAML Applications",
    responses(
        (status = 200, description = "Updated SAML application")
    )
)]
pub async fn update<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    _headers: HeaderMap,
    Path((tenant_id, app_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateSamlApplicationInput>,
) -> Result<Json<SuccessResponse<SamlApplicationResponse>>> {
    ensure_tenant_access(&state, &auth, tenant_id).await?;
    let app = state
        .saml_application_service()
        .update(StringUuid::from(tenant_id), StringUuid::from(app_id), input)
        .await?;
    Ok(Json(SuccessResponse::new(app)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{tenant_id}/saml-apps/{app_id}",
    tag = "SAML Applications",
    responses(
        (status = 200, description = "Deleted")
    )
)]
pub async fn delete<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    _headers: HeaderMap,
    Path((tenant_id, app_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MessageResponse>> {
    ensure_tenant_access(&state, &auth, tenant_id).await?;
    state
        .saml_application_service()
        .delete(StringUuid::from(tenant_id), StringUuid::from(app_id))
        .await?;
    Ok(Json(MessageResponse::new(
        "SAML application deleted successfully.",
    )))
}

/// Get IdP Metadata XML (public endpoint — no auth required)
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{tenant_id}/saml-apps/{app_id}/metadata",
    tag = "SAML Applications",
    responses(
        (status = 200, description = "IdP Metadata XML", content_type = "application/xml")
    )
)]
pub async fn get_metadata<S: HasServices>(
    State(state): State<S>,
    Path((tenant_id, app_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    let xml = state
        .saml_application_service()
        .get_idp_metadata(StringUuid::from(tenant_id), StringUuid::from(app_id))
        .await?;
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )],
        xml,
    ))
}

async fn ensure_tenant_access<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    tenant_id: Uuid,
) -> Result<()> {
    policy::enforce_with_state(
        state,
        auth,
        &PolicyInput {
            action: PolicyAction::TenantSsoWrite,
            scope: ResourceScope::Tenant(StringUuid::from(tenant_id)),
        },
    )
    .await
}
