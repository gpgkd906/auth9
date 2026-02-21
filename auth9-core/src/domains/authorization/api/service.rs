//! Service/Client API handlers

use crate::api::{
    deserialize_page, deserialize_per_page, extract_actor_id_generic, extract_ip,
    write_audit_log_generic, MessageResponse, PaginatedResponse, SuccessResponse,
};
use crate::config::Config;
use crate::domain::{
    CreateClientInput, CreateServiceInput, Service, ServiceStatus, StringUuid, UpdateServiceInput,
};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakOidcClient;
use crate::middleware::auth::AuthUser;
use crate::policy::{
    enforce, enforce_with_state, is_platform_admin_with_db, PolicyAction, PolicyInput,
    ResourceScope,
};
use crate::repository::audit::CreateAuditLogInput;
use crate::repository::AuditRepository;
use crate::state::HasServices;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
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
/// - Platform admin (any token type with platform admin email): can manage any service
/// - TenantAccess tokens: can only manage services in their own tenant
fn require_service_access(
    config: &Config,
    auth: &AuthUser,
    service_tenant_id: Option<Uuid>,
) -> Result<()> {
    match service_tenant_id {
        Some(tid) => enforce(
            config,
            auth,
            &PolicyInput {
                action: PolicyAction::ServiceWrite,
                scope: ResourceScope::Tenant(StringUuid::from(tid)),
            },
        ),
        None => enforce(
            config,
            auth,
            &PolicyInput {
                action: PolicyAction::PlatformAdmin,
                scope: ResourceScope::Global,
            },
        ),
    }
}

// ============================================================================
// API Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListServicesQuery {
    #[serde(default = "default_page", deserialize_with = "deserialize_page")]
    pub page: i64,
    #[serde(
        default = "default_per_page",
        deserialize_with = "deserialize_per_page",
        alias = "limit"
    )]
    pub per_page: i64,
    pub tenant_id: Option<Uuid>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

#[utoipa::path(
    get,
    path = "/api/v1/services",
    tag = "Authorization",
    responses(
        (status = 200, description = "List of services")
    )
)]
/// List services
/// - Platform admin (Identity token): can list all services or filter by tenant
/// - Tenant user (TenantAccess token): can only list services in their tenant
pub async fn list<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Query(query): Query<ListServicesQuery>,
) -> Result<impl IntoResponse> {
    let tenant_filter = if let Some(requested_tenant) = query.tenant_id {
        let auth_result = enforce_with_state(
            &state,
            &auth,
            &PolicyInput {
                action: PolicyAction::ServiceList,
                scope: ResourceScope::Tenant(StringUuid::from(requested_tenant)),
            },
        )
        .await;
        if auth_result.is_err() {
            let _ = log_access_denied(
                &state,
                &headers,
                &auth,
                "service.list",
                "Cannot list services in another tenant",
            )
            .await;
        }
        auth_result?;
        Some(requested_tenant)
    } else if is_platform_admin_with_db(&state, &auth).await {
        None
    } else {
        let token_tenant = auth
            .tenant_id
            .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;
        let auth_result = enforce_with_state(
            &state,
            &auth,
            &PolicyInput {
                action: PolicyAction::ServiceList,
                scope: ResourceScope::Tenant(StringUuid::from(token_tenant)),
            },
        )
        .await;
        if auth_result.is_err() {
            let _ = log_access_denied(
                &state,
                &headers,
                &auth,
                "service.list",
                "Platform admin or tenant-scoped token required",
            )
            .await;
        }
        auth_result?;
        Some(token_tenant)
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

#[utoipa::path(
    get,
    path = "/api/v1/services/{id}",
    tag = "Authorization",
    responses(
        (status = 200, description = "Service details")
    )
)]
/// Get service by ID
pub async fn get<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;
    Ok(Json(SuccessResponse::new(service)))
}

#[utoipa::path(
    post,
    path = "/api/v1/services",
    tag = "Authorization",
    responses(
        (status = 201, description = "Service created")
    )
)]
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

#[utoipa::path(
    put,
    path = "/api/v1/services/{id}",
    tag = "Authorization",
    responses(
        (status = 200, description = "Service updated")
    )
)]
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
    require_service_access(
        state.config(),
        &auth,
        before.tenant_id.as_ref().map(|t| t.0),
    )?;
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

#[utoipa::path(
    get,
    path = "/api/v1/services/{id}/clients",
    tag = "Authorization",
    responses(
        (status = 200, description = "List of clients")
    )
)]
/// List clients of a service
pub async fn list_clients<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;
    let clients = state.client_service().list_clients(id).await?;
    Ok(Json(SuccessResponse::new(clients)))
}

#[utoipa::path(
    post,
    path = "/api/v1/services/{id}/clients",
    tag = "Authorization",
    responses(
        (status = 200, description = "Client created")
    )
)]
/// Create a new client for a service
pub async fn create_client<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateClientInput>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;

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

#[utoipa::path(
    delete,
    path = "/api/v1/services/{service_id}/clients/{client_id}",
    tag = "Authorization",
    responses(
        (status = 200, description = "Client deleted")
    )
)]
/// Delete a client from a service
pub async fn delete_client<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((service_id, client_id)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    // Check if client exists and belongs to service
    let service = state.client_service().get(service_id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;

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

#[utoipa::path(
    delete,
    path = "/api/v1/services/{id}",
    tag = "Authorization",
    responses(
        (status = 200, description = "Service deleted")
    )
)]
/// Delete service
pub async fn delete<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;
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

// ============================================================================
// Integration Info Types & Handler
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceIntegrationInfo {
    pub service: ServiceBasicInfo,
    pub clients: Vec<ClientIntegrationInfo>,
    pub endpoints: EndpointInfo,
    pub grpc: GrpcInfo,
    pub environment_variables: Vec<EnvVar>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceBasicInfo {
    pub id: String,
    pub name: String,
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub logout_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ClientIntegrationInfo {
    pub client_id: String,
    pub name: Option<String>,
    pub public_client: bool,
    pub client_secret: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EndpointInfo {
    pub auth9_domain: String,
    pub auth9_public_url: String,
    pub authorize: String,
    pub token: String,
    pub callback: String,
    pub logout: String,
    pub userinfo: String,
    pub openid_configuration: String,
    pub jwks: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GrpcInfo {
    pub address: String,
    pub auth_mode: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub description: String,
}

/// Build endpoint info from config
fn build_endpoint_info(config: &Config) -> EndpointInfo {
    let domain = config
        .keycloak
        .core_public_url
        .clone()
        .unwrap_or_else(|| config.jwt.issuer.clone());
    let public_url = domain.clone();

    EndpointInfo {
        authorize: format!("{}/api/v1/auth/authorize", domain),
        token: format!("{}/api/v1/auth/token", domain),
        callback: format!("{}/api/v1/auth/callback", domain),
        logout: format!("{}/api/v1/auth/logout", domain),
        userinfo: format!("{}/api/v1/auth/userinfo", domain),
        openid_configuration: format!("{}/.well-known/openid-configuration", domain),
        jwks: format!("{}/.well-known/jwks.json", domain),
        auth9_domain: domain,
        auth9_public_url: public_url,
    }
}

/// Build environment variables list from integration data
fn build_env_vars(
    endpoints: &EndpointInfo,
    grpc: &GrpcInfo,
    clients: &[ClientIntegrationInfo],
) -> Vec<EnvVar> {
    let mut vars = vec![
        EnvVar {
            key: "AUTH9_DOMAIN".to_string(),
            value: endpoints.auth9_domain.clone(),
            description: "Auth9 Core API base URL".to_string(),
        },
        EnvVar {
            key: "AUTH9_PUBLIC_URL".to_string(),
            value: endpoints.auth9_public_url.clone(),
            description: "Auth9 public-facing URL".to_string(),
        },
        EnvVar {
            key: "AUTH9_GRPC_ADDRESS".to_string(),
            value: grpc.address.clone(),
            description: "gRPC Token Exchange endpoint".to_string(),
        },
        EnvVar {
            key: "AUTH9_GRPC_API_KEY".to_string(),
            value: "<your-grpc-api-key>".to_string(),
            description: "gRPC API key (create separately)".to_string(),
        },
    ];

    if let Some(first) = clients.first() {
        vars.push(EnvVar {
            key: "AUTH9_AUDIENCE".to_string(),
            value: first.client_id.clone(),
            description: "OAuth audience (client_id of primary client)".to_string(),
        });
        vars.push(EnvVar {
            key: "AUTH9_CLIENT_ID".to_string(),
            value: first.client_id.clone(),
            description: "OAuth Client ID".to_string(),
        });

        if first.public_client {
            vars.push(EnvVar {
                key: "AUTH9_CLIENT_SECRET".to_string(),
                value: "(not required — public client)".to_string(),
                description: "Public client does not use a secret".to_string(),
            });
        } else if let Some(ref secret) = first.client_secret {
            vars.push(EnvVar {
                key: "AUTH9_CLIENT_SECRET".to_string(),
                value: secret.clone(),
                description: "OAuth Client Secret (confidential)".to_string(),
            });
        }
    }

    vars
}

#[utoipa::path(
    get,
    path = "/api/v1/services/{id}/integration",
    tag = "Authorization",
    responses(
        (status = 200, description = "Service integration info")
    )
)]
/// Get integration info for a service
pub async fn integration_info<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let service = state.client_service().get(id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;

    let db_clients = state.client_service().list_clients(id).await?;

    // Enrich each client with Keycloak data (public_client flag, secret)
    let mut clients = Vec::new();
    for c in &db_clients {
        let kc_result = state
            .keycloak_client()
            .get_client_by_client_id(&c.client_id)
            .await;

        let (public_client, client_secret) = match kc_result {
            Ok(kc_client) => {
                if kc_client.public_client {
                    (true, None)
                } else {
                    // Confidential — fetch secret from Keycloak
                    let secret = match kc_client.id {
                        Some(ref kc_uuid) => {
                            match state.keycloak_client().get_client_secret(kc_uuid).await {
                                Ok(s) => Some(s),
                                Err(e) => {
                                    tracing::warn!(
                                        client_id = %c.client_id,
                                        keycloak_uuid = %kc_uuid,
                                        error = %e,
                                        "Failed to fetch client secret from Keycloak, attempting to regenerate"
                                    );
                                    // Secret may not exist yet — try regenerating
                                    match state
                                        .keycloak_client()
                                        .regenerate_client_secret(kc_uuid)
                                        .await
                                    {
                                        Ok(s) => Some(s),
                                        Err(e2) => {
                                            tracing::error!(
                                                client_id = %c.client_id,
                                                error = %e2,
                                                "Failed to regenerate client secret"
                                            );
                                            None
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            tracing::warn!(
                                client_id = %c.client_id,
                                "Client missing UUID in Keycloak response"
                            );
                            None
                        }
                    };
                    (false, secret)
                }
            }
            Err(_) => {
                // Client not in Keycloak — check if it's a database-managed confidential client
                let is_confidential = !c.client_secret_hash.is_empty();
                (
                    !is_confidential,
                    if is_confidential {
                        // Secret is managed in auth9 database (e.g., M2M client_credentials flow)
                        // We cannot return the plaintext since it's hashed; indicate it exists
                        Some("(set — use the secret configured at creation)".to_string())
                    } else {
                        None
                    },
                )
            }
        };

        clients.push(ClientIntegrationInfo {
            client_id: c.client_id.clone(),
            name: c.name.clone(),
            public_client,
            client_secret,
            created_at: c.created_at,
        });
    }

    let endpoints = build_endpoint_info(state.config());
    let grpc = GrpcInfo {
        address: state.config().grpc_addr(),
        auth_mode: state.config().grpc_security.auth_mode.clone(),
    };
    let environment_variables = build_env_vars(&endpoints, &grpc, &clients);

    let info = ServiceIntegrationInfo {
        service: ServiceBasicInfo {
            id: service.id.0.to_string(),
            name: service.name.clone(),
            base_url: service.base_url.clone(),
            redirect_uris: service.redirect_uris.clone(),
            logout_uris: service.logout_uris.clone(),
        },
        clients,
        endpoints,
        grpc,
        environment_variables,
    };

    Ok(Json(SuccessResponse::new(info)))
}

#[utoipa::path(
    post,
    path = "/api/v1/services/{service_id}/clients/{client_id}/regenerate-secret",
    tag = "Authorization",
    responses(
        (status = 200, description = "Client secret regenerated")
    )
)]
/// Regenerate client secret
pub async fn regenerate_client_secret<S: HasServices>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((service_id, client_id)): Path<(Uuid, String)>,
) -> Result<impl IntoResponse> {
    // Verify service exists and check access
    let service = state.client_service().get(service_id).await?;
    require_service_access(
        state.config(),
        &auth,
        service.tenant_id.as_ref().map(|t| t.0),
    )?;

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

/// Log an access_denied event to the audit log
async fn log_access_denied<S: HasServices>(
    state: &S,
    headers: &HeaderMap,
    auth: &AuthUser,
    action: &str,
    reason: &str,
) {
    let actor_id = extract_actor_id_generic(state, headers);
    let ip_address = extract_ip(headers);
    let _ = state
        .audit_repo()
        .create(&CreateAuditLogInput {
            actor_id,
            action: "access_denied".to_string(),
            resource_type: action.to_string(),
            resource_id: None,
            old_value: None,
            new_value: serde_json::to_value(serde_json::json!({
                "reason": reason,
                "actor_email": &auth.email,
            }))
            .ok(),
            ip_address,
        })
        .await;
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
    // Integration info struct tests
    // ========================================================================

    fn test_config() -> Config {
        Config {
            environment: "development".to_string(),
            http_host: "127.0.0.1".to_string(),
            http_port: 8080,
            grpc_host: "127.0.0.1".to_string(),
            grpc_port: 50051,
            database: crate::config::DatabaseConfig {
                url: "mysql://localhost/test".to_string(),
                max_connections: 10,
                min_connections: 2,
                acquire_timeout_secs: 30,
                idle_timeout_secs: 600,
            },
            redis: crate::config::RedisConfig {
                url: "redis://localhost:6379".to_string(),
            },
            jwt: crate::config::JwtConfig {
                secret: "test-secret".to_string(),
                issuer: "http://localhost:8080".to_string(),
                access_token_ttl_secs: 3600,
                refresh_token_ttl_secs: 604800,
                private_key_pem: None,
                public_key_pem: None,
                previous_public_key_pem: None,
            },
            keycloak: crate::config::KeycloakConfig {
                url: "http://localhost:8081".to_string(),
                public_url: "http://localhost:8081".to_string(),
                realm: "auth9".to_string(),
                admin_client_id: "admin-cli".to_string(),
                admin_client_secret: "secret".to_string(),
                ssl_required: "none".to_string(),
                core_public_url: None,
                portal_url: None,
                webhook_secret: None,
                event_source: "redis_stream".to_string(),
                event_stream_key: "auth9:keycloak:events".to_string(),
                event_stream_group: "auth9-core".to_string(),
                event_stream_consumer: "auth9-core-1".to_string(),
            },
            grpc_security: crate::config::GrpcSecurityConfig::default(),
            rate_limit: crate::config::RateLimitConfig::default(),
            cors: crate::config::CorsConfig::default(),
            webauthn: crate::config::WebAuthnConfig {
                rp_id: "localhost".to_string(),
                rp_name: "Auth9".to_string(),
                rp_origin: "http://localhost:3000".to_string(),
                challenge_ttl_secs: 300,
            },
            server: crate::config::ServerConfig::default(),
            telemetry: crate::config::TelemetryConfig::default(),
            password_reset: crate::config::PasswordResetConfig {
                hmac_key: "test-key".to_string(),
                token_ttl_secs: 3600,
            },
            platform_admin_emails: vec!["admin@auth9.local".to_string()],
            jwt_tenant_access_allowed_audiences: vec![],
            security_headers: crate::config::SecurityHeadersConfig::default(),
            portal_client_id: None,
            async_action: crate::domain::action::AsyncActionConfig::default(),
            branding_allowed_domains: vec![],
        }
    }

    #[test]
    fn test_build_endpoint_info_default() {
        let config = test_config();
        let ep = build_endpoint_info(&config);

        assert_eq!(ep.auth9_domain, "http://localhost:8080");
        assert_eq!(ep.authorize, "http://localhost:8080/api/v1/auth/authorize");
        assert_eq!(ep.token, "http://localhost:8080/api/v1/auth/token");
        assert_eq!(ep.callback, "http://localhost:8080/api/v1/auth/callback");
        assert_eq!(ep.logout, "http://localhost:8080/api/v1/auth/logout");
        assert_eq!(ep.userinfo, "http://localhost:8080/api/v1/auth/userinfo");
        assert_eq!(
            ep.openid_configuration,
            "http://localhost:8080/.well-known/openid-configuration"
        );
        assert_eq!(ep.jwks, "http://localhost:8080/.well-known/jwks.json");
    }

    #[test]
    fn test_build_endpoint_info_with_core_public_url() {
        let mut config = test_config();
        config.keycloak.core_public_url = Some("https://api.auth9.example.com".to_string());
        let ep = build_endpoint_info(&config);

        assert_eq!(ep.auth9_domain, "https://api.auth9.example.com");
        assert_eq!(
            ep.authorize,
            "https://api.auth9.example.com/api/v1/auth/authorize"
        );
    }

    #[test]
    fn test_build_env_vars_confidential_client() {
        let ep = EndpointInfo {
            auth9_domain: "http://localhost:8080".to_string(),
            auth9_public_url: "http://localhost:8080".to_string(),
            authorize: String::new(),
            token: String::new(),
            callback: String::new(),
            logout: String::new(),
            userinfo: String::new(),
            openid_configuration: String::new(),
            jwks: String::new(),
        };
        let grpc = GrpcInfo {
            address: "127.0.0.1:50051".to_string(),
            auth_mode: "api_key".to_string(),
        };
        let clients = vec![ClientIntegrationInfo {
            client_id: "my-client".to_string(),
            name: Some("Main".to_string()),
            public_client: false,
            client_secret: Some("secret-123".to_string()),
            created_at: chrono::Utc::now(),
        }];

        let vars = build_env_vars(&ep, &grpc, &clients);

        assert!(vars.iter().any(|v| v.key == "AUTH9_DOMAIN"));
        assert!(vars
            .iter()
            .any(|v| v.key == "AUTH9_CLIENT_ID" && v.value == "my-client"));
        assert!(vars
            .iter()
            .any(|v| v.key == "AUTH9_CLIENT_SECRET" && v.value == "secret-123"));
        assert!(vars.iter().any(|v| v.key == "AUTH9_GRPC_ADDRESS"));
    }

    #[test]
    fn test_build_env_vars_public_client() {
        let ep = EndpointInfo {
            auth9_domain: "http://localhost:8080".to_string(),
            auth9_public_url: "http://localhost:8080".to_string(),
            authorize: String::new(),
            token: String::new(),
            callback: String::new(),
            logout: String::new(),
            userinfo: String::new(),
            openid_configuration: String::new(),
            jwks: String::new(),
        };
        let grpc = GrpcInfo {
            address: "127.0.0.1:50051".to_string(),
            auth_mode: "none".to_string(),
        };
        let clients = vec![ClientIntegrationInfo {
            client_id: "spa-client".to_string(),
            name: None,
            public_client: true,
            client_secret: None,
            created_at: chrono::Utc::now(),
        }];

        let vars = build_env_vars(&ep, &grpc, &clients);

        let secret_var = vars.iter().find(|v| v.key == "AUTH9_CLIENT_SECRET");
        assert!(secret_var.is_some());
        assert!(secret_var.unwrap().value.contains("not required"));
    }

    #[test]
    fn test_build_env_vars_no_clients() {
        let ep = EndpointInfo {
            auth9_domain: "http://localhost:8080".to_string(),
            auth9_public_url: "http://localhost:8080".to_string(),
            authorize: String::new(),
            token: String::new(),
            callback: String::new(),
            logout: String::new(),
            userinfo: String::new(),
            openid_configuration: String::new(),
            jwks: String::new(),
        };
        let grpc = GrpcInfo {
            address: "127.0.0.1:50051".to_string(),
            auth_mode: "none".to_string(),
        };

        let vars = build_env_vars(&ep, &grpc, &[]);

        // Should only have the 4 base vars, no client-specific vars
        assert_eq!(vars.len(), 4);
        assert!(!vars.iter().any(|v| v.key == "AUTH9_CLIENT_ID"));
    }

    #[test]
    fn test_service_integration_info_serialization() {
        let info = ServiceIntegrationInfo {
            service: ServiceBasicInfo {
                id: "test-id".to_string(),
                name: "Test Service".to_string(),
                base_url: Some("https://test.com".to_string()),
                redirect_uris: vec!["https://test.com/cb".to_string()],
                logout_uris: vec![],
            },
            clients: vec![ClientIntegrationInfo {
                client_id: "client-1".to_string(),
                name: Some("Main Client".to_string()),
                public_client: false,
                client_secret: Some("secret-value".to_string()),
                created_at: chrono::Utc::now(),
            }],
            endpoints: EndpointInfo {
                auth9_domain: "http://localhost:8080".to_string(),
                auth9_public_url: "http://localhost:8080".to_string(),
                authorize: "http://localhost:8080/api/v1/auth/authorize".to_string(),
                token: "http://localhost:8080/api/v1/auth/token".to_string(),
                callback: "http://localhost:8080/api/v1/auth/callback".to_string(),
                logout: "http://localhost:8080/api/v1/auth/logout".to_string(),
                userinfo: "http://localhost:8080/api/v1/auth/userinfo".to_string(),
                openid_configuration: "http://localhost:8080/.well-known/openid-configuration"
                    .to_string(),
                jwks: "http://localhost:8080/.well-known/jwks.json".to_string(),
            },
            grpc: GrpcInfo {
                address: "127.0.0.1:50051".to_string(),
                auth_mode: "api_key".to_string(),
            },
            environment_variables: vec![EnvVar {
                key: "AUTH9_DOMAIN".to_string(),
                value: "http://localhost:8080".to_string(),
                description: "Auth9 base URL".to_string(),
            }],
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Test Service"));
        assert!(json.contains("client-1"));
        assert!(json.contains("secret-value"));
        assert!(json.contains("api_key"));
        assert!(json.contains("AUTH9_DOMAIN"));
    }

    #[test]
    fn test_grpc_info_serialization() {
        let info = GrpcInfo {
            address: "localhost:50051".to_string(),
            auth_mode: "mtls".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("localhost:50051"));
        assert!(json.contains("mtls"));
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
