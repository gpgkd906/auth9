//! Service/Client API handlers

use crate::api::{
    deserialize_page, deserialize_per_page, write_audit_log_generic, MessageResponse,
    PaginatedResponse, SuccessResponse,
};
use crate::config::Config;
use crate::domain::{
    CreateClientInput, CreateServiceInput, Service, ServiceStatus, UpdateServiceInput,
};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakOidcClient;
use crate::middleware::auth::{AuthUser, TokenType};
use crate::state::HasServices;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

// ============================================================================
// Helper functions (testable without AppState)
// ============================================================================

/// Build Keycloak attributes from logout URIs
pub fn build_logout_attributes(logout_uris: &[String]) -> Option<HashMap<String, String>> {
    if logout_uris.is_empty() {
        None
    } else {
        let mut attrs = HashMap::new();
        attrs.insert(
            "post.logout.redirect.uris".to_string(),
            logout_uris.join(" "),
        );
        Some(attrs)
    }
}

/// Build KeycloakOidcClient from CreateServiceInput
pub fn build_keycloak_client_from_create_input(input: &CreateServiceInput) -> KeycloakOidcClient {
    let logout_uris = input.logout_uris.clone().unwrap_or_default();
    let attributes = build_logout_attributes(&logout_uris);

    KeycloakOidcClient {
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
    }
}

/// Merge update input with existing service values
pub struct MergedServiceUpdate {
    pub name: String,
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub logout_uris: Vec<String>,
    pub status: ServiceStatus,
}

pub fn merge_service_update(before: &Service, input: &UpdateServiceInput) -> MergedServiceUpdate {
    MergedServiceUpdate {
        name: input.name.clone().unwrap_or_else(|| before.name.clone()),
        base_url: input.base_url.clone().or(before.base_url.clone()),
        redirect_uris: input
            .redirect_uris
            .clone()
            .unwrap_or_else(|| before.redirect_uris.clone()),
        logout_uris: input
            .logout_uris
            .clone()
            .unwrap_or_else(|| before.logout_uris.clone()),
        status: input.status.clone().unwrap_or(before.status.clone()),
    }
}

/// Build KeycloakOidcClient for update from merged values
pub fn build_keycloak_client_for_update(
    client_id: &str,
    merged: &MergedServiceUpdate,
) -> KeycloakOidcClient {
    let attributes = build_logout_attributes(&merged.logout_uris);

    KeycloakOidcClient {
        id: None,
        client_id: client_id.to_string(),
        name: Some(merged.name.clone()),
        enabled: true,
        protocol: "openid-connect".to_string(),
        base_url: merged.base_url.clone(),
        root_url: merged.base_url.clone(),
        admin_url: merged.base_url.clone(),
        redirect_uris: merged.redirect_uris.clone(),
        web_origins: merged
            .base_url
            .as_ref()
            .map(|url| vec![url.clone()])
            .unwrap_or_default(),
        attributes,
        public_client: false,
        secret: None,
    }
}

// ============================================================================
// Authorization helpers
// ============================================================================

/// Check if the authenticated user can manage a service.
/// - Identity tokens (platform admin): can manage any service
/// - TenantAccess tokens: can only manage services in their own tenant
fn require_service_access(
    config: &Config,
    auth: &AuthUser,
    service_tenant_id: Option<Uuid>,
) -> Result<()> {
    match auth.token_type {
        TokenType::Identity => {
            if config.is_platform_admin_email(&auth.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Platform admin required: identity token is not a platform admin".to_string(),
                ))
            }
        }
        TokenType::TenantAccess => {
            let token_tenant = auth
                .tenant_id
                .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;

            match service_tenant_id {
                Some(tid) if tid == token_tenant => {
                    // Check if user has admin/owner role or service:write permissions
                    let has_admin_role =
                        auth.roles.iter().any(|r| r == "admin" || r == "owner");
                    let has_service_permission = auth
                        .permissions
                        .iter()
                        .any(|p| p == "service:write" || p == "service:*");

                    if has_admin_role || has_service_permission {
                        Ok(())
                    } else {
                        Err(AppError::Forbidden(
                            "Admin access required to manage services".to_string(),
                        ))
                    }
                }
                _ => Err(AppError::Forbidden(
                    "Cannot access services in another tenant".to_string(),
                )),
            }
        }
        TokenType::ServiceClient => {
            // Service client tokens cannot manage services
            Err(AppError::Forbidden(
                "Service client tokens cannot manage services".to_string(),
            ))
        }
    }
}

// ============================================================================
// API Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListServicesQuery {
    #[serde(default = "default_page", deserialize_with = "deserialize_page")]
    pub page: i64,
    #[serde(default = "default_per_page", deserialize_with = "deserialize_per_page")]
    pub per_page: i64,
    pub tenant_id: Option<Uuid>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// List services
/// - Platform admin (Identity token): can list all services or filter by tenant
/// - Tenant user (TenantAccess token): can only list services in their tenant
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Query(query): Query<ListServicesQuery>,
) -> Result<impl IntoResponse> {
    let tenant_filter = match auth.token_type {
        TokenType::Identity => {
            if !state.config().is_platform_admin_email(&auth.email) {
                return Err(AppError::Forbidden(
                    "Platform admin required to list services without tenant scope".to_string(),
                ));
            }
            query.tenant_id // Platform admin: optional filter
        }
        TokenType::TenantAccess | TokenType::ServiceClient => {
            // Tenant user / service client: must scope to their tenant
            let token_tenant = auth
                .tenant_id
                .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
            // If they specified a different tenant, deny
            if let Some(requested) = query.tenant_id {
                if requested != token_tenant {
                    return Err(AppError::Forbidden(
                        "Cannot list services in another tenant".to_string(),
                    ));
                }
            }
            Some(token_tenant)
        }
    };

    let (services, total) = state
        .client_service()
        .list(tenant_filter, query.page, query.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        services,
        query.page,
        query.per_page,
        total,
    )))
}

/// Get service by ID
pub async fn get<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(state.config(), &auth, service.tenant_id.as_ref().map(|t| t.0))?;
    Ok(Json(SuccessResponse::new(service)))
}

/// Create service
pub async fn create<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Json(input): Json<CreateServiceInput>,
) -> Result<impl IntoResponse> {
    input.validate()?;
    require_service_access(state.config(), &auth, input.tenant_id)?;
    let keycloak_client = build_keycloak_client_from_create_input(&input);

    let client_uuid = state
        .keycloak_client()
        .create_oidc_client(&keycloak_client)
        .await?;
    let client_secret = state
        .keycloak_client()
        .get_client_secret(&client_uuid)
        .await?;

    // create_with_secret now creates the service AND an initial client
    let service_with_client = state
        .client_service()
        .create_with_secret(input, client_secret)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.create",
        "service",
        Some(service_with_client.service.id.0),
        None,
        serde_json::to_value(&service_with_client.service).ok(),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse::new(service_with_client)),
    ))
}

/// Update service
pub async fn update<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateServiceInput>,
) -> Result<impl IntoResponse> {
    input.validate()?;
    let before = state.client_service().get(id).await?;
    require_service_access(state.config(), &auth, before.tenant_id.as_ref().map(|t| t.0))?;
    let merged = merge_service_update(&before, &input);

    // Update all associated Keycloak clients with new service settings
    let keycloak_clients = state.client_service().list_clients(id).await?;
    for client in keycloak_clients {
        let keycloak_client = build_keycloak_client_for_update(&client.client_id, &merged);
        if let Ok(kc_uuid) = state
            .keycloak_client()
            .get_client_uuid_by_client_id(&client.client_id)
            .await
        {
            let _ = state
                .keycloak_client()
                .update_oidc_client(&kc_uuid, &keycloak_client)
                .await;
        }
    }

    let service = state.client_service().update(id, input).await?;
    let _ = write_audit_log_generic(
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

/// List clients of a service
pub async fn list_clients<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(state.config(), &auth, service.tenant_id.as_ref().map(|t| t.0))?;
    let clients = state.client_service().list_clients(id).await?;
    Ok(Json(SuccessResponse::new(clients)))
}

/// Create a new client for a service
pub async fn create_client<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateClientInput>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(state.config(), &auth, service.tenant_id.as_ref().map(|t| t.0))?;

    // Create new Keycloak client
    // We need to generate a client_id logic or let Keycloak do it?
    // User said "clientId and clientSecret auto generated".
    // Keycloak usually can generate. Or we generate UUID.
    // Let's generate a UUID based client_id.
    let new_client_id = Uuid::new_v4().to_string();

    // Setup Keycloak Client with Service defaults
    // ... logic to create keycloak client ...
    let logout_uris = service.logout_uris.clone();
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
        client_id: new_client_id.clone(),
        name: Some(format!(
            "{} - {}",
            service.name,
            input.name.clone().unwrap_or("Client".to_string())
        )),
        enabled: service.status == crate::domain::ServiceStatus::Active,
        protocol: "openid-connect".to_string(),
        base_url: service.base_url.clone(),
        root_url: service.base_url.clone(),
        admin_url: service.base_url.clone(),
        redirect_uris: service.redirect_uris.clone(),
        web_origins: service
            .base_url
            .as_ref()
            .map(|url| vec![url.clone()])
            .unwrap_or_default(),
        attributes,
        public_client: false,
        secret: None, // Keycloak will generate
    };

    let kc_uuid = state
        .keycloak_client()
        .create_oidc_client(&keycloak_client)
        .await?;
    let client_secret = state.keycloak_client().get_client_secret(&kc_uuid).await?;

    let client_with_secret = state
        .client_service()
        .create_client_with_secret(id, new_client_id, client_secret, input.name.clone())
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.client.create",
        "client",
        Some(client_with_secret.client.id.0),
        None,
        None,
    )
    .await;

    Ok(Json(SuccessResponse::new(client_with_secret)))
}

/// Delete a client from a service
pub async fn delete_client<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((service_id, client_id)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    // Check if client exists and belongs to service
    let service = state.client_service().get(service_id).await?;
    require_service_access(state.config(), &auth, service.tenant_id.as_ref().map(|t| t.0))?;

    // Also delete from Keycloak
    if let Ok(kc_uuid) = state
        .keycloak_client()
        .get_client_uuid_by_client_id(&client_id)
        .await
    {
        let _ = state.keycloak_client().delete_oidc_client(&kc_uuid).await;
    }

    state
        .client_service()
        .delete_client(service_id, &client_id)
        .await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.client.delete",
        "client",
        Some(service_id), // Log under service?
        Some(serde_json::json!({ "client_id": client_id })),
        None,
    )
    .await;

    Ok(Json(MessageResponse::new("Client deleted successfully")))
}

/// Delete service
pub async fn delete<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(state.config(), &auth, service.tenant_id.as_ref().map(|t| t.0))?;
    // Also delete all Keycloak clients associated with this service
    // If we assume a name prefix or just delete clients in DB...
    // The clients in DB have `client_id` which maps to Keycloak.
    // Ideally we should iterate clients and delete them from Keycloak first.
    let clients = state.client_service().list_clients(id).await?;
    for client in clients {
        if let Ok(kc_uuid) = state
            .keycloak_client()
            .get_client_uuid_by_client_id(&client.client_id)
            .await
        {
            let _ = state.keycloak_client().delete_oidc_client(&kc_uuid).await;
        }
    }

    state.client_service().delete(id).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.delete",
        "service",
        Some(id),
        None,
        None,
    )
    .await;
    Ok(Json(MessageResponse::new("Service deleted successfully")))
}

/// Regenerate client secret
pub async fn regenerate_client_secret<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((service_id, client_id)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    // Verify service exists and check access
    let service = state.client_service().get(service_id).await?;
    require_service_access(state.config(), &auth, service.tenant_id.as_ref().map(|t| t.0))?;

    // Regenerate in Keycloak first (if it exists there)
    let new_secret = if let Ok(kc_uuid) = state
        .keycloak_client()
        .get_client_uuid_by_client_id(&client_id)
        .await
    {
        // Use Keycloak's regenerated secret
        state
            .keycloak_client()
            .regenerate_client_secret(&kc_uuid)
            .await?
    } else {
        // Fallback: generate our own secret using ClientService
        state
            .client_service()
            .regenerate_client_secret(&client_id)
            .await?
    };

    // If Keycloak was used, we need to sync the hash to our DB
    // ClientService.regenerate_client_secret already updates the hash if used
    // But if Keycloak generated, we need manual DB update
    if state
        .keycloak_client()
        .get_client_uuid_by_client_id(&client_id)
        .await
        .is_ok()
    {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let secret_hash = argon2
            .hash_password(new_secret.as_bytes(), &salt)
            .map_err(|e| {
                crate::error::AppError::Internal(anyhow::anyhow!("Failed to hash secret: {}", e))
            })?
            .to_string();

        // Use service method to update (we need to add this method)
        state
            .client_service()
            .update_client_secret_hash(&client_id, &secret_hash)
            .await?;
    }

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "service.client.regenerate_secret",
        "client",
        Some(service_id),
        Some(serde_json::json!({ "client_id": client_id })),
        None,
    )
    .await;

    Ok(Json(SuccessResponse::new(serde_json::json!({
        "client_id": client_id,
        "client_secret": new_secret
    }))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CreateServiceInput, ServiceStatus, UpdateServiceInput};

    #[test]
    fn test_list_services_query_defaults() {
        let query: ListServicesQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.page, 1);
        assert_eq!(query.per_page, 20);
        assert!(query.tenant_id.is_none());
    }

    #[test]
    fn test_list_services_query_with_tenant() {
        let json = r#"{
            "page": 2,
            "per_page": 50,
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }"#;
        let query: ListServicesQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, 2);
        assert_eq!(query.per_page, 50);
        assert!(query.tenant_id.is_some());
    }

    #[test]
    fn test_create_service_input_deserialization() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "My Service",
            "client_id": "my-service",
            "base_url": "https://myservice.example.com",
            "redirect_uris": ["https://myservice.example.com/callback"]
        }"#;
        let input: CreateServiceInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "My Service");
        assert_eq!(input.client_id, "my-service");
        assert_eq!(
            input.base_url,
            Some("https://myservice.example.com".to_string())
        );
        assert_eq!(input.redirect_uris.len(), 1);
    }

    #[test]
    fn test_create_service_input_with_logout_uris() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Service with Logout",
            "client_id": "logout-service",
            "redirect_uris": ["https://app.com/cb"],
            "logout_uris": ["https://app.com/logout", "https://app.com/signout"]
        }"#;
        let input: CreateServiceInput = serde_json::from_str(json).unwrap();
        assert!(input.logout_uris.is_some());
        assert_eq!(input.logout_uris.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_create_service_input_minimal() {
        let json = r#"{
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "name": "Minimal Service",
            "client_id": "minimal",
            "redirect_uris": []
        }"#;
        let input: CreateServiceInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "Minimal Service");
        assert!(input.base_url.is_none());
        assert!(input.logout_uris.is_none());
        assert!(input.redirect_uris.is_empty());
    }

    #[test]
    fn test_update_service_input_partial() {
        let json = r#"{"name": "Updated Service Name"}"#;
        let input: UpdateServiceInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Updated Service Name".to_string()));
        assert!(input.base_url.is_none());
        assert!(input.redirect_uris.is_none());
        assert!(input.status.is_none());
    }

    #[test]
    fn test_update_service_input_status_change() {
        let json = r#"{"status": "inactive"}"#;
        let input: UpdateServiceInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.status, Some(ServiceStatus::Inactive));
    }

    #[test]
    fn test_update_service_input_full() {
        let json = r#"{
            "name": "Full Update",
            "base_url": "https://new-url.example.com",
            "redirect_uris": ["https://new-url.example.com/cb"],
            "logout_uris": ["https://new-url.example.com/logout"],
            "status": "active"
        }"#;
        let input: UpdateServiceInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("Full Update".to_string()));
        assert_eq!(
            input.base_url,
            Some("https://new-url.example.com".to_string())
        );
        assert_eq!(input.redirect_uris.as_ref().unwrap().len(), 1);
        assert_eq!(input.logout_uris.as_ref().unwrap().len(), 1);
        assert_eq!(input.status, Some(ServiceStatus::Active));
    }

    #[test]
    fn test_service_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Inactive).unwrap(),
            "\"inactive\""
        );
    }

    #[test]
    fn test_service_status_deserialization() {
        let active: ServiceStatus = serde_json::from_str("\"active\"").unwrap();
        let inactive: ServiceStatus = serde_json::from_str("\"inactive\"").unwrap();

        assert_eq!(active, ServiceStatus::Active);
        assert_eq!(inactive, ServiceStatus::Inactive);
    }

    #[test]
    fn test_create_client_input_deserialization() {
        let json = r#"{"name": "My Client"}"#;
        let input: CreateClientInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, Some("My Client".to_string()));
    }

    #[test]
    fn test_create_client_input_empty() {
        let json = r#"{}"#;
        let input: CreateClientInput = serde_json::from_str(json).unwrap();
        assert!(input.name.is_none());
    }

    // ========================================================================
    // Tests for extracted helper functions
    // ========================================================================

    #[test]
    fn test_build_logout_attributes_empty() {
        let result = build_logout_attributes(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_build_logout_attributes_single() {
        let uris = vec!["https://app.com/logout".to_string()];
        let result = build_logout_attributes(&uris);

        assert!(result.is_some());
        let attrs = result.unwrap();
        assert_eq!(
            attrs.get("post.logout.redirect.uris"),
            Some(&"https://app.com/logout".to_string())
        );
    }

    #[test]
    fn test_build_logout_attributes_multiple() {
        let uris = vec![
            "https://app.com/logout".to_string(),
            "https://app.com/signout".to_string(),
        ];
        let result = build_logout_attributes(&uris);

        assert!(result.is_some());
        let attrs = result.unwrap();
        let value = attrs.get("post.logout.redirect.uris").unwrap();
        assert!(value.contains("https://app.com/logout"));
        assert!(value.contains("https://app.com/signout"));
        assert!(value.contains(" ")); // Space-separated
    }

    #[test]
    fn test_build_keycloak_client_from_create_input() {
        let input = CreateServiceInput {
            tenant_id: Some(uuid::Uuid::new_v4()),
            name: "Test Service".to_string(),
            client_id: "test-client".to_string(),
            base_url: Some("https://test.example.com".to_string()),
            redirect_uris: vec!["https://test.example.com/callback".to_string()],
            logout_uris: Some(vec!["https://test.example.com/logout".to_string()]),
        };

        let kc_client = build_keycloak_client_from_create_input(&input);

        assert_eq!(kc_client.client_id, "test-client");
        assert_eq!(kc_client.name, Some("Test Service".to_string()));
        assert!(kc_client.enabled);
        assert_eq!(kc_client.protocol, "openid-connect");
        assert_eq!(
            kc_client.base_url,
            Some("https://test.example.com".to_string())
        );
        assert_eq!(kc_client.redirect_uris.len(), 1);
        assert!(kc_client.attributes.is_some());
        assert!(!kc_client.public_client);
    }

    #[test]
    fn test_build_keycloak_client_from_create_input_minimal() {
        let input = CreateServiceInput {
            tenant_id: Some(uuid::Uuid::new_v4()),
            name: "Minimal".to_string(),
            client_id: "minimal".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        let kc_client = build_keycloak_client_from_create_input(&input);

        assert_eq!(kc_client.client_id, "minimal");
        assert!(kc_client.base_url.is_none());
        assert!(kc_client.redirect_uris.is_empty());
        assert!(kc_client.web_origins.is_empty());
        assert!(kc_client.attributes.is_none());
    }

    #[test]
    fn test_merge_service_update_all_fields() {
        let before = Service {
            id: crate::domain::StringUuid::new_v4(),
            tenant_id: None,
            name: "Old Name".to_string(),
            base_url: Some("https://old.example.com".to_string()),
            redirect_uris: vec!["https://old.example.com/cb".to_string()],
            logout_uris: vec!["https://old.example.com/logout".to_string()],
            status: ServiceStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let input = UpdateServiceInput {
            name: Some("New Name".to_string()),
            base_url: Some("https://new.example.com".to_string()),
            redirect_uris: Some(vec!["https://new.example.com/cb".to_string()]),
            logout_uris: Some(vec!["https://new.example.com/logout".to_string()]),
            status: Some(ServiceStatus::Inactive),
        };

        let merged = merge_service_update(&before, &input);

        assert_eq!(merged.name, "New Name");
        assert_eq!(merged.base_url, Some("https://new.example.com".to_string()));
        assert_eq!(merged.redirect_uris.len(), 1);
        assert_eq!(merged.logout_uris.len(), 1);
        assert_eq!(merged.status, ServiceStatus::Inactive);
    }

    #[test]
    fn test_merge_service_update_partial() {
        let before = Service {
            id: crate::domain::StringUuid::new_v4(),
            tenant_id: None,
            name: "Original".to_string(),
            base_url: Some("https://original.com".to_string()),
            redirect_uris: vec!["https://original.com/cb".to_string()],
            logout_uris: vec![],
            status: ServiceStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let input = UpdateServiceInput {
            name: Some("Updated Name".to_string()),
            base_url: None,      // Keep original
            redirect_uris: None, // Keep original
            logout_uris: None,   // Keep original
            status: None,        // Keep original
        };

        let merged = merge_service_update(&before, &input);

        assert_eq!(merged.name, "Updated Name");
        assert_eq!(merged.base_url, Some("https://original.com".to_string()));
        assert_eq!(
            merged.redirect_uris,
            vec!["https://original.com/cb".to_string()]
        );
        assert_eq!(merged.status, ServiceStatus::Active);
    }

    #[test]
    fn test_build_keycloak_client_for_update() {
        let merged = MergedServiceUpdate {
            name: "Updated Service".to_string(),
            base_url: Some("https://updated.example.com".to_string()),
            redirect_uris: vec!["https://updated.example.com/cb".to_string()],
            logout_uris: vec!["https://updated.example.com/logout".to_string()],
            status: ServiceStatus::Active,
        };

        let kc_client = build_keycloak_client_for_update("my-client-id", &merged);

        assert_eq!(kc_client.client_id, "my-client-id");
        assert_eq!(kc_client.name, Some("Updated Service".to_string()));
        assert_eq!(
            kc_client.base_url,
            Some("https://updated.example.com".to_string())
        );
        assert!(kc_client.attributes.is_some());
    }
}
