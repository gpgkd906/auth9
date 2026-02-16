//! Identity Provider API handlers

use crate::api::{MessageResponse, SuccessResponse};
use crate::domain::{
    CreateIdentityProviderInput, IdentityProvider, IdentityProviderTemplate, LinkedIdentityInfo,
    StringUuid, UpdateIdentityProviderInput,
};
use crate::error::AppError;
use crate::state::{HasIdentityProviders, HasServices};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};

/// List all identity providers
pub async fn list_providers<S: HasIdentityProviders>(
    State(state): State<S>,
) -> Result<Json<SuccessResponse<Vec<IdentityProvider>>>, AppError> {
    let providers = state.identity_provider_service().list_providers().await?;
    Ok(Json(SuccessResponse::new(providers)))
}

/// Get an identity provider by alias
pub async fn get_provider<S: HasIdentityProviders>(
    State(state): State<S>,
    Path(alias): Path<String>,
) -> Result<Json<SuccessResponse<IdentityProvider>>, AppError> {
    let provider = state
        .identity_provider_service()
        .get_provider(&alias)
        .await?;
    Ok(Json(SuccessResponse::new(provider)))
}

/// Create a new identity provider
pub async fn create_provider<S: HasIdentityProviders>(
    State(state): State<S>,
    Json(input): Json<CreateIdentityProviderInput>,
) -> Result<Json<SuccessResponse<IdentityProvider>>, AppError> {
    let provider = state
        .identity_provider_service()
        .create_provider(input)
        .await?;
    Ok(Json(SuccessResponse::new(provider)))
}

/// Update an identity provider
pub async fn update_provider<S: HasIdentityProviders>(
    State(state): State<S>,
    Path(alias): Path<String>,
    Json(input): Json<UpdateIdentityProviderInput>,
) -> Result<Json<SuccessResponse<IdentityProvider>>, AppError> {
    let provider = state
        .identity_provider_service()
        .update_provider(&alias, input)
        .await?;
    Ok(Json(SuccessResponse::new(provider)))
}

/// Delete an identity provider
pub async fn delete_provider<S: HasIdentityProviders>(
    State(state): State<S>,
    Path(alias): Path<String>,
) -> Result<Json<MessageResponse>, AppError> {
    state
        .identity_provider_service()
        .delete_provider(&alias)
        .await?;
    Ok(Json(MessageResponse::new(
        "Identity provider deleted successfully.",
    )))
}

/// Get available identity provider templates
pub async fn get_templates<S: HasIdentityProviders>(
    State(state): State<S>,
) -> Result<Json<SuccessResponse<Vec<IdentityProviderTemplate>>>, AppError> {
    let templates = state.identity_provider_service().get_templates();
    Ok(Json(SuccessResponse::new(templates)))
}

/// Get linked identities for the current user
pub async fn list_my_linked_identities<S: HasIdentityProviders + HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Result<Json<SuccessResponse<Vec<LinkedIdentityInfo>>>, AppError> {
    let user_id = extract_user_id(&state, &headers)?;

    let identities = state
        .identity_provider_service()
        .get_user_identities(user_id)
        .await?;

    Ok(Json(SuccessResponse::new(identities)))
}

/// Unlink an identity from the current user
pub async fn unlink_identity<S: HasIdentityProviders + HasServices>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(identity_id): Path<StringUuid>,
) -> Result<Json<MessageResponse>, AppError> {
    let user_id = extract_user_id(&state, &headers)?;

    state
        .identity_provider_service()
        .unlink_identity(user_id, identity_id)
        .await?;

    Ok(Json(MessageResponse::new(
        "Identity unlinked successfully.",
    )))
}

/// Extract user ID from JWT token
fn extract_user_id<S: HasIdentityProviders + HasServices>(
    state: &S,
    headers: &HeaderMap,
) -> Result<StringUuid, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::Unauthorized("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AppError::Unauthorized("Invalid authorization header".to_string()))?;

    let token = auth_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("Invalid authorization header format".to_string()))?;

    let jwt = HasServices::jwt_manager(state);

    if let Ok(claims) = jwt.verify_identity_token(token) {
        return StringUuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
    }

    let allowed = &state.config().jwt_tenant_access_allowed_audiences;
    if !allowed.is_empty() {
        if let Ok(claims) = jwt.verify_tenant_access_token_strict(token, allowed) {
            return StringUuid::parse_str(&claims.sub)
                .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
        }
    } else if !state.config().is_production() {
        #[allow(deprecated)]
        if let Ok(claims) = jwt.verify_tenant_access_token(token, None) {
            return StringUuid::parse_str(&claims.sub)
                .map_err(|_| AppError::Unauthorized("Invalid user ID in token".to_string()));
        }
    }

    Err(AppError::Unauthorized(
        "Invalid or expired token".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_provider_templates() {
        let templates = IdentityProviderTemplate::all();
        assert!(templates.len() >= 5);

        // Verify Google template exists
        let google = templates.iter().find(|t| t.provider_id == "google");
        assert!(google.is_some());
    }
}
