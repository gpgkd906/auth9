//! Branding API handlers

use crate::api::MessageResponse;
use crate::api::{require_platform_admin_with_db, write_audit_log_generic, SuccessResponse};
use crate::domain::{
    PublicBrandingQuery, StringUuid, UpdateBrandingRequest, UpdateServiceBrandingRequest,
};
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::state::{HasBranding, HasServices};
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};

/// Get branding configuration (public endpoint, no authentication required)
///
/// This endpoint is intended for use by Keycloak themes to fetch branding settings.
/// Accepts optional `client_id` query parameter to return service-specific branding.
/// Falls back to system default if no service-level branding is configured.
///
/// GET /api/v1/public/branding?client_id=xxx
#[utoipa::path(
    get,
    path = "/api/v1/public/branding",
    tag = "Platform",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_public_branding<S: HasBranding>(
    State(state): State<S>,
    Query(query): Query<PublicBrandingQuery>,
) -> Result<impl IntoResponse> {
    let config = if let Some(client_id) = &query.client_id {
        state
            .branding_service()
            .get_branding_by_client_id(client_id)
            .await?
    } else {
        state.branding_service().get_branding().await?
    };
    Ok(Json(SuccessResponse::new(config)))
}

/// Get branding configuration (authenticated endpoint)
///
/// GET /api/v1/system/branding
#[utoipa::path(
    get,
    path = "/api/v1/system/branding",
    tag = "Platform",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_branding<S: HasBranding + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let config = state.branding_service().get_branding().await?;
    Ok(Json(SuccessResponse::new(config)))
}

/// Update branding configuration
///
/// PUT /api/v1/system/branding
#[utoipa::path(
    put,
    path = "/api/v1/system/branding",
    tag = "Platform",
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_branding<S: HasBranding + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(request): Json<UpdateBrandingRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let config = state
        .branding_service()
        .update_branding(request.config)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "system.branding.update",
        "system_setting",
        None,
        None,
        serde_json::to_value(&config).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(config)))
}

/// Get branding for a specific service
///
/// GET /api/v1/services/{service_id}/branding
#[utoipa::path(
    get,
    path = "/api/v1/services/{service_id}/branding",
    tag = "Platform",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn get_service_branding<S: HasBranding + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let config = state
        .branding_service()
        .get_service_branding_only(service_id)
        .await?;
    Ok(Json(SuccessResponse::new(config)))
}

/// Update branding for a specific service
///
/// PUT /api/v1/services/{service_id}/branding
#[utoipa::path(
    put,
    path = "/api/v1/services/{service_id}/branding",
    tag = "Platform",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn update_service_branding<S: HasBranding + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
    headers: HeaderMap,
    Json(request): Json<UpdateServiceBrandingRequest>,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    let result = state
        .branding_service()
        .update_service_branding(service_id, request.config)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.branding.update",
        "service_branding",
        Some(*service_id),
        None,
        serde_json::to_value(&result).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(result)))
}

/// Delete branding for a specific service (revert to system default)
///
/// DELETE /api/v1/services/{service_id}/branding
#[utoipa::path(
    delete,
    path = "/api/v1/services/{service_id}/branding",
    tag = "Platform",
    params(
        ("service_id" = String, Path, description = "Service ID")
    ),
    responses(
        (status = 200, description = "Success")
    )
)]
pub async fn delete_service_branding<S: HasBranding + HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(service_id): Path<StringUuid>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    require_platform_admin_with_db(&state, &auth).await?;
    state
        .branding_service()
        .delete_service_branding(service_id)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.branding.delete",
        "service_branding",
        Some(*service_id),
        None,
        None,
    )
    .await;

    Ok(Json(MessageResponse::new(
        "Service branding deleted, reverted to system default.",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::BrandingConfig;

    #[test]
    fn test_update_branding_request_deserialization() {
        let json = r##"{"config":{"logo_url":"https://example.com/logo.png","primary_color":"#FF5733","secondary_color":"#33FF57","background_color":"#AABBCC","text_color":"#000000","company_name":"TestCorp"}}"##;

        let request: UpdateBrandingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.config.primary_color, "#FF5733");
        assert_eq!(request.config.company_name, Some("TestCorp".to_string()));
    }

    #[test]
    fn test_update_branding_request_minimal() {
        let json = r##"{"config":{"primary_color":"#FF5733","secondary_color":"#33FF57","background_color":"#AABBCC","text_color":"#000000"}}"##;

        let request: UpdateBrandingRequest = serde_json::from_str(json).unwrap();
        assert!(request.config.logo_url.is_none());
        assert!(request.config.company_name.is_none());
    }

    #[test]
    fn test_branding_config_serialization() {
        let config = BrandingConfig {
            logo_url: Some("https://example.com/logo.png".to_string()),
            primary_color: "#007AFF".to_string(),
            secondary_color: "#5856D6".to_string(),
            background_color: "#F5F5F7".to_string(),
            text_color: "#1D1D1F".to_string(),
            custom_css: Some(".login { color: red; }".to_string()),
            company_name: Some("Auth9".to_string()),
            favicon_url: Some("https://example.com/favicon.ico".to_string()),
            allow_registration: false,
        };

        let response = SuccessResponse::new(config);
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("logo_url"));
        assert!(json.contains("#007AFF"));
        assert!(json.contains("Auth9"));
    }

    #[test]
    fn test_update_branding_request_with_custom_css() {
        let json = r##"{"config":{"primary_color":"#FF5733","secondary_color":"#33FF57","background_color":"#AABBCC","text_color":"#000000","custom_css":".login-form { border-radius: 8px; }"}}"##;

        let request: UpdateBrandingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.config.custom_css,
            Some(".login-form { border-radius: 8px; }".to_string())
        );
    }

    #[test]
    fn test_update_branding_request_with_favicon() {
        let json = r##"{"config":{"primary_color":"#FF5733","secondary_color":"#33FF57","background_color":"#AABBCC","text_color":"#000000","favicon_url":"https://example.com/favicon.ico"}}"##;

        let request: UpdateBrandingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.config.favicon_url,
            Some("https://example.com/favicon.ico".to_string())
        );
    }
}
