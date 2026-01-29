//! User API handlers

use crate::api::{
    write_audit_log, MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse,
};
use crate::domain::{AddUserToTenantInput, CreateUserInput, UpdateUserInput};
use crate::error::Result;
use crate::keycloak::{CreateKeycloakUserInput, KeycloakCredential, KeycloakUserUpdate};
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

/// List users
pub async fn list(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let (users, total) = state
        .user_service
        .list(pagination.page, pagination.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        users,
        pagination.page,
        pagination.per_page,
        total,
    )))
}

/// Get user by ID
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<impl IntoResponse> {
    let user = state.user_service.get(id).await?;
    Ok(Json(SuccessResponse::new(user)))
}

/// Create user input (includes optional password for Keycloak)
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    #[serde(flatten)]
    pub user: CreateUserInput,
    pub password: Option<String>,
}

/// Create user
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateUserRequest>,
) -> Result<impl IntoResponse> {
    let credentials = input.password.map(|password| {
        vec![KeycloakCredential {
            credential_type: "password".to_string(),
            value: password,
            temporary: false,
        }]
    });

    let keycloak_id = state
        .keycloak_client
        .create_user(&CreateKeycloakUserInput {
            username: input.user.email.clone(),
            email: input.user.email.clone(),
            first_name: input.user.display_name.clone(),
            last_name: None,
            enabled: true,
            email_verified: false,
            credentials,
        })
        .await?;

    let user = state.user_service.create(&keycloak_id, input.user).await?;

    let _ = write_audit_log(
        &state,
        &headers,
        "user.create",
        "user",
        Some(user.id),
        None,
        serde_json::to_value(&user).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(user))))
}

/// Update user
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateUserInput>,
) -> Result<impl IntoResponse> {
    let before = state.user_service.get(id).await?;
    if input.display_name.is_some() {
        let update = KeycloakUserUpdate {
            username: None,
            email: None,
            first_name: input.display_name.clone(),
            last_name: None,
            enabled: None,
            email_verified: None,
            required_actions: None,
        };
        state
            .keycloak_client
            .update_user(&before.keycloak_id, &update)
            .await?;
    }
    let user = state.user_service.update(id, input).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "user.update",
        "user",
        Some(user.id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&user).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(user)))
}

/// Delete user
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let before = state.user_service.get(id).await?;
    if let Err(err) = state.keycloak_client.delete_user(&before.keycloak_id).await {
        if !matches!(err, crate::error::AppError::NotFound(_)) {
            return Err(err);
        }
    }
    state.user_service.delete(id).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "user.delete",
        "user",
        Some(id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("User deleted successfully")))
}

/// Add user to tenant
#[derive(Debug, Deserialize)]
pub struct AddToTenantRequest {
    pub tenant_id: Uuid,
    pub role_in_tenant: String,
}

pub async fn add_to_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(input): Json<AddToTenantRequest>,
) -> Result<impl IntoResponse> {
    let tenant_user = state
        .user_service
        .add_to_tenant(AddUserToTenantInput {
            user_id,
            tenant_id: input.tenant_id,
            role_in_tenant: input.role_in_tenant,
        })
        .await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "user.add_to_tenant",
        "tenant_user",
        Some(tenant_user.id),
        None,
        serde_json::to_value(&tenant_user).ok(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant_user))))
}

/// Remove user from tenant
pub async fn remove_from_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    state
        .user_service
        .remove_from_tenant(user_id, tenant_id)
        .await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "user.remove_from_tenant",
        "tenant_user",
        None,
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("User removed from tenant")))
}

/// Get user's tenants
pub async fn get_tenants(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let tenants = state.user_service.get_user_tenants(user_id).await?;
    Ok(Json(SuccessResponse::new(tenants)))
}

pub async fn enable_mfa(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = state.user_service.get(id).await?;
    let update = KeycloakUserUpdate {
        username: None,
        email: None,
        first_name: None,
        last_name: None,
        enabled: None,
        email_verified: None,
        required_actions: Some(vec!["CONFIGURE_TOTP".to_string()]),
    };
    state
        .keycloak_client
        .update_user(&user.keycloak_id, &update)
        .await?;
    let updated = state.user_service.set_mfa_enabled(id, true).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "user.mfa.enable",
        "user",
        Some(updated.id),
        None,
        serde_json::to_value(&updated).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(updated)))
}

pub async fn disable_mfa(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = state.user_service.get(id).await?;
    state
        .keycloak_client
        .remove_totp_credentials(&user.keycloak_id)
        .await?;
    let update = KeycloakUserUpdate {
        username: None,
        email: None,
        first_name: None,
        last_name: None,
        enabled: None,
        email_verified: None,
        required_actions: Some(vec![]),
    };
    state
        .keycloak_client
        .update_user(&user.keycloak_id, &update)
        .await?;
    let updated = state.user_service.set_mfa_enabled(id, false).await?;
    let _ = write_audit_log(
        &state,
        &headers,
        "user.mfa.disable",
        "user",
        Some(updated.id),
        None,
        serde_json::to_value(&updated).ok(),
    )
    .await;
    Ok(Json(SuccessResponse::new(updated)))
}

/// List users in a tenant
pub async fn list_by_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<impl IntoResponse> {
    let users = state
        .user_service
        .list_tenant_users(tenant_id, pagination.page, pagination.per_page)
        .await?;
    Ok(Json(SuccessResponse::new(users)))
}
