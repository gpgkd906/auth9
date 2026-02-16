//! Branding API handlers

use crate::api::{require_platform_admin_with_db, write_audit_log_generic, SuccessResponse};
use crate::domain::UpdateBrandingRequest;
use crate::error::Result;
use crate::middleware::auth::AuthUser;
use crate::state::{HasBranding, HasServices};
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};

/// Get branding configuration (public endpoint, no authentication required)
///
/// This endpoint is intended for use by Keycloak themes to fetch branding settings.
/// It returns the full branding configuration without any masking.
///
/// GET /api/v1/public/branding
pub async fn get_public_branding<S: HasBranding>(
    State(state): State<S>,
) -> Result<impl IntoResponse> {
    let config = state.branding_service().get_branding().await?;
    Ok(Json(SuccessResponse::new(config)))
}

/// Get branding configuration (authenticated endpoint)
///
/// GET /api/v1/system/branding
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
