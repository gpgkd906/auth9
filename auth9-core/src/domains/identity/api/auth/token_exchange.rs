//! Tenant token exchange and userinfo endpoints.

use super::helpers::{extract_client_ip, extract_identity_claims_from_headers};
use super::types::{TenantTokenExchangeRequest, TokenResponse};
use crate::error::{AppError, Result};
use crate::http_support::{write_audit_log_generic, SuccessResponse};
use crate::jwt::claims::sanitize_action_claims;
use crate::models::action::{
    ActionContext, ActionContextRequest, ActionContextTenant, ActionContextUser,
};
use crate::models::common::StringUuid;
use crate::state::HasServices;
use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;

#[utoipa::path(
    post,
    path = "/api/v1/auth/tenant-token",
    tag = "Identity",
    responses(
        (status = 200, description = "Tenant access token")
    )
)]
pub async fn tenant_token<S: HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(params): Json<TenantTokenExchangeRequest>,
) -> Result<Response> {
    let tenant_ref = params.tenant_id.trim();
    let service_id = params.service_id.trim();
    if tenant_ref.is_empty() {
        return Err(AppError::BadRequest("Missing tenant_id".to_string()));
    }
    if service_id.is_empty() {
        return Err(AppError::BadRequest("Missing service_id".to_string()));
    }

    let identity_claims = extract_identity_claims_from_headers(&state, &headers)?;
    let user_id = identity_claims
        .sub
        .parse::<StringUuid>()
        .map_err(|_| AppError::Unauthorized("Invalid user ID in identity token".to_string()))?;

    let tenant_id = match tenant_ref.parse::<StringUuid>() {
        Ok(id) => id,
        Err(_) => state.tenant_service().get_by_slug(tenant_ref).await?.id,
    };

    // Verify tenant is active before allowing token exchange
    let tenant = state.tenant_service().get(tenant_id).await?;
    if tenant.status != crate::models::tenant::TenantStatus::Active {
        return Err(AppError::Forbidden(format!(
            "Tenant is not active (status: '{}')",
            tenant.status
        )));
    }

    state
        .rbac_service()
        .ensure_tenant_membership(user_id, tenant_id)
        .await?;

    let service = state
        .client_service()
        .get_by_client_id(service_id)
        .await
        .map_err(|e| match e {
            AppError::NotFound(_) => {
                AppError::Forbidden("Service does not belong to the requested tenant".to_string())
            }
            other => other,
        })?;
    if let Some(service_tenant_id) = service.tenant_id {
        if service_tenant_id != tenant_id {
            return Err(AppError::Forbidden(
                "Service does not belong to the requested tenant".to_string(),
            ));
        }
    }

    let user_roles = state
        .rbac_service()
        .get_user_roles_for_service(user_id, tenant_id, service.id)
        .await?;

    // Execute post-login actions for the target tenant and inject sanitized claims
    let custom_claims = {
        let user = state.user_service().get(user_id).await?;
        let ip_address = extract_client_ip(&headers);
        let user_agent = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let context = ActionContext {
            user: ActionContextUser {
                id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name.clone(),
                mfa_enabled: user.mfa_enabled,
            },
            tenant: ActionContextTenant {
                id: tenant_id.to_string(),
                slug: String::new(),
                name: String::new(),
            },
            service: None,
            request: ActionContextRequest {
                ip: ip_address,
                user_agent,
                timestamp: Utc::now(),
            },
            claims: None,
        };
        let modified = state
            .action_service()
            .execute_trigger_by_tenant(tenant_id, "post-login", context)
            .await?;
        modified.claims.and_then(sanitize_action_claims)
    };

    let jwt_manager = state.jwt_manager();
    let access_token = jwt_manager.create_tenant_access_token_with_claims(
        *user_id,
        &identity_claims.email,
        *tenant_id,
        service_id,
        user_roles.roles,
        user_roles.permissions,
        identity_claims.sid.clone(),
        custom_claims,
    )?;
    let refresh_token = jwt_manager.create_refresh_token(*user_id, *tenant_id, service_id)?;

    // Write audit log for tenant token exchange
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "token_exchange.rest.succeeded",
        "token_exchange",
        Some(*tenant_id),
        None,
        Some(serde_json::json!({
            "user_id": user_id.to_string(),
            "tenant_id": tenant_id.to_string(),
            "service_id": service_id,
        })),
    )
    .await;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
        refresh_token: Some(refresh_token),
        id_token: None,
    })
    .into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/userinfo",
    tag = "Identity",
    responses(
        (status = 200, description = "User info")
    )
)]
/// UserInfo endpoint
///
/// Accepts Identity tokens, Tenant Access tokens, and Service Client tokens
/// via the standard AuthUser middleware chain.
pub async fn userinfo<S: HasServices>(auth: crate::middleware::auth::AuthUser) -> Result<Response> {
    Ok(Json(auth).into_response())
}

// Suppress unused import warning -- SuccessResponse is used by related modules
// but we keep the import for consistency with the original file.
#[allow(unused_imports)]
use SuccessResponse as _SuccessResponse;
