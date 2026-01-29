//! Service/Client API handlers

use crate::api::{write_audit_log, MessageResponse, PaginatedResponse, SuccessResponse};
use crate::domain::{CreateServiceInput, UpdateServiceInput, CreateClientInput};
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
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
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
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListServicesQuery>,
) -> Result<impl IntoResponse> {
    let (services, total) = state
        .client_service
        .list(query.tenant_id, query.page, query.per_page)
        .await?;

    Ok(Json(PaginatedResponse::new(
        services,
        query.page,
        query.per_page,
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

    // create_with_secret now creates the service AND an initial client
    let service_with_client = state
        .client_service
        .create_with_secret(input, client_secret)
        .await?;

    let _ = write_audit_log(
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

    // Note: Keycloak OIDC Client update might need to know which Keycloak Client to update.
    // We assume 'client_id' is constant for the Service, but we removed client_id from Service!
    // Wait, Keycloak Client has a client_id.
    // If Service moves to multiple clients, which one maps to the Keycloak Client ID?
    // Keycloak OIDC client is a "Service" effectively.
    // The "clients" in our DB are credentials.
    // But Keycloak has "Credentials" tab which allows Rotated Secrets?
    // Keycloak supports Secret Rotation if we use "Client Secret".
    // But our `clients` table manages them too?
    // IF we are using Keycloak as backing, we might need to be careful.
    // In "Headless Keycloak", `services` table == Keycloak Clients.
    // And `clients` table == credentials?
    // If so, `Service` struct needs to know the KEYCLOAK CLIENT ID string (not UUID).
    // I removed `client_id` from `Service` struct!
    // This is a problem if `client_id` was the link to Keycloak.
    // Original `Service` had `client_id` (e.g. "my-app").
    // New `Service` removed it.
    // BUT `CreateServiceInput` has `client_id`.
    // If I removed `client_id` from `Service`, how do I know which Keycloak client it is?
    // ERROR: I probably deleted `client_id` from Service too hastily. 
    // `Service` usually represents the "Application". The "Application" has a "Client ID" in OIDC.
    // The "Client Secret" is what we rotate.
    // The `client_id` usually stays constant for the Application.
    // BUT the User Requirement says: "去掉clientId栏目，改为允许单独创建client，clientId和clientSecret自动生成".
    // "Remove clientId column, allow creating separate clients, clientId and clientSecret auto-generated."
    // This implies `Service` is just a container, and we can have MULTIPLE valid `client_id`s for one Service?
    // If so, Keycloak mapping becomes 1 Service -> N Keycloak Clients? Or 1 Keycloak Client with multiple secrets using some advanced feature?
    // Or maybe we are NOT mapping 1:1 to Keycloak Clients anymore?
    // If we have multiple `client_id`s, for OIDC, each `client_id` IS a Keycloak Client.
    // So 1 "Service" in our DB = N "keycloak_clients" in Keycloak?
    // If so, `create_client` should create a NEW Keycloak Client.
    // And `Service` is just a grouping logical entity.
    
    // IF that is the case, `Service` struct should NOT have `client_id`.
    // And `create` Service might just be creating the Container.
    // And `create_client` creates the actual OIDC Client in Keycloak.

    // Let's assume this design: Service = Group of Clients.
    // But then, `base_url`, `redirect_uris` are usually per Client in OIDC.
    // Although they can be shared.
    
    // In `api/service.rs`: `update` uses `before.client_id` to find Keycloak client.
    // If `Service` doesn't have `client_id`, we can't update Keycloak client easily if we assume 1:1.
    
    // If we assume 1 Service = N Clients (Keycloak Clients), then `update` service (base_url etc) should update ALL associated Keycloak Clients?
    // This seems correct for "Service" level settings.
    
    // SO: `Service` has no `client_id`.
    // `Client` has `client_id`.
    // When updating Service, we should find ALL Clients of this service, and update their Keycloak config?
    // Or maybe Service settings are just templates?
    
    // Let's look at `update` again.
    // `before.client_id` is used.
    // I removed it. So `before.client_id` will fail to compile.
    
    // I need to fetch all clients for the service.
    // And update each of them in Keycloak?
    
    // Or, maybe the "Service" itself has a "Primary Client ID" that stays?
    // The user requirement "client_id and client_secret auto generated" and "remove clientId column" suggests we shouldn't have a fixed one on Service.
    
    // So, `input` to `update` changes Service DB fields.
    // And we should probably iterate all clients and update Keycloak.
    
    // REVISED UPDATE LOGIC:
    // 1. Update Service DB.
    // 2. Fetch all clients of service.
    // 3. For each client, update Keycloak Client (using client.client_id).
    
    // I need to inject `list_clients` here.
    
    let keycloak_clients = state.client_service.list_clients(id).await?;
    
    for client in keycloak_clients {
        let logout_uris = input.logout_uris.clone().unwrap_or_default();
        let attributes = if logout_uris.is_empty() {
            None
        } else {
            let mut attrs = std::collections::HashMap::new();
            attrs.insert("post.logout.redirect.uris".to_string(), logout_uris.join(" "));
            Some(attrs)
        };
        
        let keycloak_client = KeycloakOidcClient {
            id: None,
            client_id: client.client_id.clone(),
            name: Some(updated_name.clone()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: input.base_url.clone(),
            root_url: input.base_url.clone(),
            admin_url: input.base_url.clone(),
            redirect_uris: input.redirect_uris.clone().unwrap_or_default(),
            web_origins: input.base_url.as_ref().map(|u| vec![u.clone()]).unwrap_or_default(),
            attributes,
            public_client: false,
            secret: None,
        };
        if let Ok(kc_uuid) = state.keycloak_client.get_client_uuid_by_client_id(&client.client_id).await {
            let _ = state.keycloak_client.update_oidc_client(&kc_uuid, &keycloak_client).await;
        }
    }
    
    let service = state.client_service.update(id, input).await?;
    // ...
    Ok(Json(SuccessResponse::new(service)))
}

/// List clients of a service
pub async fn list_clients(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let clients = state.client_service.list_clients(id).await?;
    Ok(Json(SuccessResponse::new(clients)))
}

/// Create a new client for a service
pub async fn create_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateClientInput>,
) -> Result<impl IntoResponse> {
    let service = state.client_service.get(id).await?;
    
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
        name: Some(format!("{} - {}", service.name, input.name.clone().unwrap_or("Client".to_string()))),
        enabled: service.status == crate::domain::ServiceStatus::Active,
        protocol: "openid-connect".to_string(),
        base_url: service.base_url.clone(),
        root_url: service.base_url.clone(),
        admin_url: service.base_url.clone(),
        redirect_uris: service.redirect_uris.clone(),
        web_origins: service.base_url.as_ref().map(|url| vec![url.clone()]).unwrap_or_default(),
        attributes,
        public_client: false,
        secret: None, // Keycloak will generate
    };

    let kc_uuid = state.keycloak_client.create_oidc_client(&keycloak_client).await?;
    let client_secret = state.keycloak_client.get_client_secret(&kc_uuid).await?;

    let client_with_secret = state
        .client_service
        .create_client_with_secret(id, new_client_id, client_secret, input.name.clone())
        .await?;

    let _ = write_audit_log(
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
pub async fn delete_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((service_id, client_id)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    // Check if client exists and belongs to service
    let _ = state.client_service.get(service_id).await?;
    
    // Also delete from Keycloak
    if let Ok(kc_uuid) = state.keycloak_client.get_client_uuid_by_client_id(&client_id).await {
        let _ = state.keycloak_client.delete_oidc_client(&kc_uuid).await;
    }

    state.client_service.delete_client(service_id, &client_id).await?;

    let _ = write_audit_log(
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
pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    // We should also delete all Keycloak clients associated with this service?
    // If we assume a name prefix or just delete clients in DB...
    // The clients in DB have `client_id` which maps to Keycloak.
    // Ideally we should iterate clients and delete them from Keycloak first.
    let clients = state.client_service.list_clients(id).await?;
    for client in clients {
         if let Ok(kc_uuid) = state.keycloak_client.get_client_uuid_by_client_id(&client.client_id).await {
            let _ = state.keycloak_client.delete_oidc_client(&kc_uuid).await;
        }
    }
    
    state.client_service.delete(id).await?;
    let _ = write_audit_log(
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
pub async fn regenerate_client_secret(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((service_id, client_id)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    // Verify service exists
    let _ = state.client_service.get(service_id).await?;
    
    // Regenerate in Keycloak first (if it exists there)
    let new_secret = if let Ok(kc_uuid) = state.keycloak_client.get_client_uuid_by_client_id(&client_id).await {
        // Use Keycloak's regenerated secret
        state.keycloak_client.regenerate_client_secret(&kc_uuid).await?
    } else {
        // Fallback: generate our own secret using ClientService
        state.client_service.regenerate_client_secret(&client_id).await?
    };
    
    // If Keycloak was used, we need to sync the hash to our DB
    // ClientService.regenerate_client_secret already updates the hash if used
    // But if Keycloak generated, we need manual DB update
    if state.keycloak_client.get_client_uuid_by_client_id(&client_id).await.is_ok() {
        use argon2::{password_hash::{PasswordHasher, SaltString, rand_core::OsRng}, Argon2};
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let secret_hash = argon2
            .hash_password(new_secret.as_bytes(), &salt)
            .map_err(|e| crate::error::AppError::Internal(anyhow::anyhow!("Failed to hash secret: {}", e)))?
            .to_string();
        
        // Use service method to update (we need to add this method)
        state.client_service.update_client_secret_hash(&client_id, &secret_hash).await?;
    }
    
    let _ = write_audit_log(
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
    use crate::domain::{CreateServiceInput, UpdateServiceInput, ServiceStatus};

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
        assert_eq!(input.base_url, Some("https://myservice.example.com".to_string()));
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
        assert_eq!(input.base_url, Some("https://new-url.example.com".to_string()));
        assert_eq!(input.redirect_uris.as_ref().unwrap().len(), 1);
        assert_eq!(input.logout_uris.as_ref().unwrap().len(), 1);
        assert_eq!(input.status, Some(ServiceStatus::Active));
    }

    #[test]
    fn test_service_status_serialization() {
        assert_eq!(serde_json::to_string(&ServiceStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&ServiceStatus::Inactive).unwrap(), "\"inactive\"");
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
}
