//! SCIM Bearer Token authentication middleware
//!
//! Validates SCIM tokens and injects ScimRequestContext into request extensions.

use crate::domains::provisioning::context::ProvisioningContext;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// SCIM authentication middleware.
/// Extracts Bearer token from Authorization header, validates it via ScimTokenService,
/// and injects ScimRequestContext into request extensions.
pub async fn scim_auth_middleware<S: ProvisioningContext>(
    State(state): State<S>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract Bearer token
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token = match auth_header {
        Some(ref header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                    "status": "401",
                    "detail": "Missing or invalid Authorization header"
                })),
            )
                .into_response();
        }
    };

    // Determine base URL from request
    let base_url = {
        let scheme = if request
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https")
        {
            "https"
        } else {
            "http"
        };
        let host = request
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        format!("{}://{}/api/v1/scim/v2", scheme, host)
    };

    // Validate token
    match state
        .scim_token_service()
        .validate_token(token, &base_url)
        .await
    {
        Ok(ctx) => {
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "schemas": ["urn:ietf:params:scim:api:messages:2.0:Error"],
                "status": "401",
                "detail": "Invalid or expired SCIM token"
            })),
        )
            .into_response(),
    }
}
