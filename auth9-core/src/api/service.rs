//! Service/Client API handlers

use crate::api::{
    write_audit_log, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::domain::{CreateServiceInput, UpdateServiceInput};
use crate::error::Result;
use crate::keycloak::KeycloakOidcClient;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ListServicesQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,
    pub tenant_id: Option<Uuid>,
}

/// List services
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListServicesQuery>,
) -> Result<impl IntoResponse> {
    let (services, total) = state
        .client_service
        .list(
            query.tenant_id,
            query.pagination.page,
            query.pagination.per_page,
        )
        .await?;

    Ok(Json(PaginatedResponse::new(
        services,
        query.pagination.page,
        query.pagination.per_page,
        total,
    )))
}

/// Get service by ID
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse> {
    let service = state.client_service.get(id).await?;
    Ok(Json(SuccessResponse::new(service)))
}

/// Create service
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateServiceInput>,
) -> Result<impl IntoResponse> {
    let logout_uris = input.logout_uris.clone().unwrap_or_default();
    let attributes = if logout_uris.is_empty() {
        None
    } else {
        let mut attrs = HashMap::new();
        attrs.insert(
            "post.logout.redirect.uris".to_string(),
            logout_uris.join(" "),
        );
        Some(attrs)
    };
    let keycloak_client = KeycloakOidcClient {
        id: None,
        client_id: input.client_id.clone(),
        name: Some(input.name.clone()),
        enabled: true,
        protocol: "openid-connect".to_string(),
        base_url: input.base_url.clone(),
        root_url: input.base_url.clone(),
        admin_url: input.base_url.clone(),
        redirect_uris: input.redirect_uris.clone(),
        web_origins: input
            .base_url
            .as_ref()
            .map(|url| vec![url.clone()])
            .unwrap_or_default(),
        attributes,
        public_client: false,
        secret: None,
    };

    let client_uuid = state
        .keycloak_client
        .create_oidc_client(&keycloak_client)
        .await?;
    let client_secret = state
        .keycloak_client
        .get_client_secret(&client_uuid)
        .await?;

    let service_with_secret = state
        .client_service
        .create_with_secret(input, client_secret)
        .await?;

    let _ = write_audit_log(
        &state,
        &headers,
        "service.create",
        "service",
        Some(service_with_secret.service.id.0),
        None,
        serde_json::to_value(&service_with_secret.service).ok(),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::new(service_with_secret)),
    ))
}

/// Update service
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateServiceInput>,
) -> Result<impl IntoResponse> {
    let before = state.client_service.get(id).await?;
    let updated_name = input.name.clone().unwrap_or_else(|| before.name.clone());
    let updated_base_url = input.base_url.clone().or(before.base_url.clone());
    let updated_redirect_uris = input
        .redirect_uris
        .clone()
        .unwrap_or_else(|| before.redirect_uris.clone());
    let updated_logout_uris = input
        .logout_uris
        .clone()
        .unwrap_or_else(|| before.logout_uris.clone());
    let updated_status = input.status.clone().unwrap_or(before.status.clone());
    let attributes = if updated_logout_uris.is_empty() {
        None
    } else {
        let mut attrs = HashMap::new();
        attrs.insert(
            "post.logout.redirect.uris".to_string(),
            updated_logout_uris.join(" "),
        );
        Some(attrs)
    };

    let keycloak_client = KeycloakOidcClient {
        id: None,
        client_id: before.client_id.clone(),
        name: Some(updated_name.clone()),
        enabled: updated_status == crate::domain::ServiceStatus::Active,
        protocol: "openid-connect".to_string(),
        base_url: updated_base_url.clone(),
        root_url: updated_base_url.clone(),
        admin_url: updated_base_url.clone(),
        redirect_uris: updated_redirect_uris.clone(),
        web_origins: updated_base_url
            .as_ref()
            .map(|url| vec![url.clone()])
            .unwrap_or_default(),
        attributes,
        public_client: false,
        secret: None,
    };
    let client_uuid = state
        .keycloak_client
        .get_client_uuid_by_client_id(&before.client_id)
        .await?;
    state
        .keycloak_client
        .update_oidc_client(&client_uuid, &keycloak_client)
        .await?;

    let service = state.client_service.update(id, input).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "service.update",
        "service",
        Some(service.id.0),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&service).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(service)))
}

/// Regenerate client secret
#[derive(Serialize)]
pub struct SecretResponse {
    pub client_secret: String,
}

pub async fn regenerate_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service.get(id).await?;
    let client_uuid = state
        .keycloak_client
        .get_client_uuid_by_client_id(&service.client_id)
        .await?;
    let secret = state
        .keycloak_client
        .regenerate_client_secret(&client_uuid)
        .await?;
    state
        .client_service
        .regenerate_secret_with_value(id, secret.clone())
        .await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "service.regenerate_secret",
        "service",
        Some(id),
        None,
        None,
    )
    .await;
    Ok(Json(SuccessResponse::new(SecretResponse {
        client_secret: secret,
    })))
}

/// Delete service
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let before = state.client_service.get(id).await?;
    if let Ok(client_uuid) = state
        .keycloak_client
        .get_client_uuid_by_client_id(&before.client_id)
        .await
    {
        let _ = state.keycloak_client.delete_oidc_client(&client_uuid).await;
    }
    state.client_service.delete(id).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "service.delete",
        "service",
        Some(id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Service deleted successfully")))
}
