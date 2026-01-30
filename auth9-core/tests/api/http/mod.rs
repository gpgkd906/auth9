//! HTTP API Handler Tests Infrastructure
//!
//! This module provides test utilities for HTTP handler testing without
//! external dependencies (no database, no Redis, no Keycloak).
//!
//! Key components:
//! - `TestAppState` - Test-friendly version of AppState using test repositories
//! - `build_test_router()` - Build a router with test state
//! - Helper functions for making HTTP requests (get_json, post_json, etc.)

pub mod mock_keycloak;
pub mod role_http_test;
pub mod service_http_test;
pub mod tenant_http_test;
pub mod user_http_test;

use crate::api::{
    create_test_jwt_manager, TestAuditRepository, TestRbacRepository, TestServiceRepository,
    TestTenantRepository, TestUserRepository,
};
use auth9_core::cache::NoOpCacheManager;
use auth9_core::config::{Config, DatabaseConfig, JwtConfig, KeycloakConfig, RedisConfig};
use auth9_core::keycloak::KeycloakClient;
use auth9_core::service::{ClientService, RbacService, TenantService, UserService};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    routing::{delete, get, post},
    Router,
};
use mock_keycloak::MockKeycloakServer;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tower::ServiceExt;

// ============================================================================
// Test Configuration
// ============================================================================

/// Create a test config with the given Keycloak base URL
pub fn create_test_config(keycloak_url: &str) -> Config {
    Config {
        http_host: "127.0.0.1".to_string(),
        http_port: 3000,
        grpc_host: "127.0.0.1".to_string(),
        grpc_port: 50051,
        database: DatabaseConfig {
            url: "mysql://test:test@localhost/test".to_string(),
            max_connections: 1,
            min_connections: 1,
        },
        redis: RedisConfig {
            url: "redis://localhost".to_string(),
        },
        jwt: JwtConfig {
            secret: "test-secret-key-for-http-testing".to_string(),
            issuer: "https://auth9.test".to_string(),
            access_token_ttl_secs: 3600,
            refresh_token_ttl_secs: 604800,
            private_key_pem: None,
            public_key_pem: None,
        },
        keycloak: KeycloakConfig {
            url: keycloak_url.to_string(),
            public_url: keycloak_url.to_string(),
            realm: "test".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "test-secret".to_string(),
            ssl_required: "none".to_string(),
        },
    }
}

// ============================================================================
// Test AppState (uses test repositories)
// ============================================================================

/// Test-friendly version of AppState using test repository implementations
#[derive(Clone)]
pub struct TestAppState {
    pub config: Arc<Config>,
    pub tenant_service: Arc<TenantService<TestTenantRepository>>,
    pub user_service: Arc<UserService<TestUserRepository>>,
    pub client_service: Arc<ClientService<TestServiceRepository>>,
    pub rbac_service: Arc<RbacService<TestRbacRepository>>,
    pub audit_repo: Arc<TestAuditRepository>,
    pub jwt_manager: auth9_core::jwt::JwtManager,
    pub keycloak_client: KeycloakClient,
    pub cache_manager: NoOpCacheManager,
    // Keep references to raw repositories for test setup
    pub tenant_repo: Arc<TestTenantRepository>,
    pub user_repo: Arc<TestUserRepository>,
    pub service_repo: Arc<TestServiceRepository>,
    pub rbac_repo: Arc<TestRbacRepository>,
}

impl TestAppState {
    /// Create a new test app state with the given Keycloak base URL
    pub fn new(keycloak_url: &str) -> Self {
        let config = Arc::new(create_test_config(keycloak_url));
        let tenant_repo = Arc::new(TestTenantRepository::new());
        let user_repo = Arc::new(TestUserRepository::new());
        let service_repo = Arc::new(TestServiceRepository::new());
        let rbac_repo = Arc::new(TestRbacRepository::new());
        let audit_repo = Arc::new(TestAuditRepository::new());

        let tenant_service = Arc::new(TenantService::new(tenant_repo.clone(), None));
        let user_service = Arc::new(UserService::new(user_repo.clone()));
        let client_service = Arc::new(ClientService::new(service_repo.clone(), None));
        let rbac_service = Arc::new(RbacService::new(rbac_repo.clone(), None));

        let jwt_manager = create_test_jwt_manager();
        let keycloak_client = KeycloakClient::new(config.keycloak.clone());
        let cache_manager = NoOpCacheManager::new();

        Self {
            config,
            tenant_service,
            user_service,
            client_service,
            rbac_service,
            audit_repo,
            jwt_manager,
            keycloak_client,
            cache_manager,
            tenant_repo,
            user_repo,
            service_repo,
            rbac_repo,
        }
    }

    /// Create with an already started mock Keycloak server
    pub fn with_mock_keycloak(mock_server: &MockKeycloakServer) -> Self {
        Self::new(&mock_server.uri())
    }
}

// ============================================================================
// Test Router Builder
// ============================================================================

/// Build a router for HTTP handler tests.
///
/// This creates routes that mirror the production router but use TestAppState.
/// The handlers are implemented as generic functions that work with TestAppState.
pub fn build_test_router(state: TestAppState) -> Router {
    Router::new()
        // Tenant endpoints
        .route("/api/v1/tenants", get(tenant_handlers::list).post(tenant_handlers::create))
        .route(
            "/api/v1/tenants/:id",
            get(tenant_handlers::get)
                .put(tenant_handlers::update)
                .delete(tenant_handlers::delete),
        )
        // User endpoints
        .route("/api/v1/users", get(user_handlers::list).post(user_handlers::create))
        .route(
            "/api/v1/users/:id",
            get(user_handlers::get)
                .put(user_handlers::update)
                .delete(user_handlers::delete),
        )
        .route(
            "/api/v1/users/:id/tenants",
            get(user_handlers::get_tenants).post(user_handlers::add_to_tenant),
        )
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id",
            delete(user_handlers::remove_from_tenant),
        )
        .route("/api/v1/tenants/:tenant_id/users", get(user_handlers::list_by_tenant))
        .route(
            "/api/v1/users/:id/mfa",
            post(user_handlers::enable_mfa).delete(user_handlers::disable_mfa),
        )
        // Service endpoints
        .route(
            "/api/v1/services",
            get(service_handlers::list).post(service_handlers::create),
        )
        .route(
            "/api/v1/services/:id",
            get(service_handlers::get)
                .put(service_handlers::update)
                .delete(service_handlers::delete),
        )
        .route(
            "/api/v1/services/:id/clients",
            get(service_handlers::list_clients).post(service_handlers::create_client),
        )
        .route(
            "/api/v1/services/:service_id/clients/:client_id",
            delete(service_handlers::delete_client),
        )
        .route(
            "/api/v1/services/:service_id/clients/:client_id/regenerate-secret",
            post(service_handlers::regenerate_client_secret),
        )
        // Permission endpoints
        .route("/api/v1/permissions", post(role_handlers::create_permission))
        .route("/api/v1/permissions/:id", delete(role_handlers::delete_permission))
        .route(
            "/api/v1/services/:service_id/permissions",
            get(role_handlers::list_permissions),
        )
        // Role endpoints
        .route("/api/v1/roles", post(role_handlers::create_role))
        .route(
            "/api/v1/roles/:id",
            get(role_handlers::get_role)
                .put(role_handlers::update_role)
                .delete(role_handlers::delete_role),
        )
        .route("/api/v1/services/:service_id/roles", get(role_handlers::list_roles))
        .route("/api/v1/roles/:role_id/permissions", post(role_handlers::assign_permission))
        .route(
            "/api/v1/roles/:role_id/permissions/:permission_id",
            delete(role_handlers::remove_permission),
        )
        // RBAC assignment
        .route("/api/v1/rbac/assign", post(role_handlers::assign_roles))
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id/roles",
            get(role_handlers::get_user_roles),
        )
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id/assigned-roles",
            get(role_handlers::get_user_assigned_roles),
        )
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id/roles/:role_id",
            delete(role_handlers::unassign_role),
        )
        .with_state(state)
}

// ============================================================================
// HTTP Test Helpers
// ============================================================================

/// Make a GET request and parse JSON response
pub async fn get_json<T: DeserializeOwned>(
    app: &Router,
    path: &str,
) -> (StatusCode, Option<T>) {
    let request = Request::builder()
        .method(Method::GET)
        .uri(path)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    // Convert body to bytes using axum's built-in method
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    if body_bytes.is_empty() {
        return (status, None);
    }

    match serde_json::from_slice(&body_bytes) {
        Ok(data) => (status, Some(data)),
        Err(_) => (status, None),
    }
}

/// Make a POST request with JSON body and parse JSON response
pub async fn post_json<T: Serialize, R: DeserializeOwned>(
    app: &Router,
    path: &str,
    body: &T,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    if body_bytes.is_empty() {
        return (status, None);
    }

    match serde_json::from_slice(&body_bytes) {
        Ok(data) => (status, Some(data)),
        Err(_) => (status, None),
    }
}

/// Make a PUT request with JSON body and parse JSON response
pub async fn put_json<T: Serialize, R: DeserializeOwned>(
    app: &Router,
    path: &str,
    body: &T,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::PUT)
        .uri(path)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    if body_bytes.is_empty() {
        return (status, None);
    }

    match serde_json::from_slice(&body_bytes) {
        Ok(data) => (status, Some(data)),
        Err(_) => (status, None),
    }
}

/// Make a DELETE request and parse JSON response
pub async fn delete_json<R: DeserializeOwned>(
    app: &Router,
    path: &str,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::DELETE)
        .uri(path)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    if body_bytes.is_empty() {
        return (status, None);
    }

    match serde_json::from_slice(&body_bytes) {
        Ok(data) => (status, Some(data)),
        Err(_) => (status, None),
    }
}

// ============================================================================
// Test Handlers (adapted from production handlers to use TestAppState)
// ============================================================================

/// Tenant API handlers for testing
pub mod tenant_handlers {
    use super::TestAppState;
    use auth9_core::api::{MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse};
    use auth9_core::domain::{CreateTenantInput, StringUuid, UpdateTenantInput};
    use auth9_core::error::Result;
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    };
    use uuid::Uuid;

    pub async fn list(
        State(state): State<TestAppState>,
        Query(pagination): Query<PaginationQuery>,
    ) -> Result<impl IntoResponse> {
        let (tenants, total) = state
            .tenant_service
            .list(pagination.page, pagination.per_page)
            .await?;
        Ok(Json(PaginatedResponse::new(
            tenants,
            pagination.page,
            pagination.per_page,
            total,
        )))
    }

    pub async fn get(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let tenant = state.tenant_service.get(StringUuid::from(id)).await?;
        Ok(Json(SuccessResponse::new(tenant)))
    }

    pub async fn create(
        State(state): State<TestAppState>,
        Json(input): Json<CreateTenantInput>,
    ) -> Result<impl IntoResponse> {
        let tenant = state.tenant_service.create(input).await?;
        Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant))))
    }

    pub async fn update(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
        Json(input): Json<UpdateTenantInput>,
    ) -> Result<impl IntoResponse> {
        let tenant = state.tenant_service.update(StringUuid::from(id), input).await?;
        Ok(Json(SuccessResponse::new(tenant)))
    }

    pub async fn delete(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let _tenant = state.tenant_service.disable(StringUuid::from(id)).await?;
        Ok(Json(MessageResponse::new("Tenant disabled successfully")))
    }
}

/// User API handlers for testing
pub mod user_handlers {
    use super::TestAppState;
    use auth9_core::api::{MessageResponse, PaginatedResponse, PaginationQuery, SuccessResponse};
    use auth9_core::domain::{AddUserToTenantInput, CreateUserInput, StringUuid, UpdateUserInput};
    use auth9_core::error::Result;
    use auth9_core::keycloak::{CreateKeycloakUserInput, KeycloakCredential, KeycloakUserUpdate};
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    };
    use serde::Deserialize;
    use uuid::Uuid;

    #[derive(Debug, Deserialize)]
    pub struct CreateUserRequest {
        #[serde(flatten)]
        pub user: CreateUserInput,
        pub password: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct AddToTenantRequest {
        pub tenant_id: Uuid,
        pub role_in_tenant: String,
    }

    pub async fn list(
        State(state): State<TestAppState>,
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

    pub async fn get(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let user = state.user_service.get(StringUuid::from(id)).await?;
        Ok(Json(SuccessResponse::new(user)))
    }

    pub async fn create(
        State(state): State<TestAppState>,
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
        Ok((StatusCode::CREATED, Json(SuccessResponse::new(user))))
    }

    pub async fn update(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
        Json(input): Json<UpdateUserInput>,
    ) -> Result<impl IntoResponse> {
        let id = StringUuid::from(id);
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
        Ok(Json(SuccessResponse::new(user)))
    }

    pub async fn delete(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let id = StringUuid::from(id);
        let before = state.user_service.get(id).await?;
        if let Err(err) = state.keycloak_client.delete_user(&before.keycloak_id).await {
            if !matches!(err, auth9_core::error::AppError::NotFound(_)) {
                return Err(err);
            }
        }
        state.user_service.delete(id).await?;
        Ok(Json(MessageResponse::new("User deleted successfully")))
    }

    pub async fn add_to_tenant(
        State(state): State<TestAppState>,
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
        Ok((StatusCode::CREATED, Json(SuccessResponse::new(tenant_user))))
    }

    pub async fn remove_from_tenant(
        State(state): State<TestAppState>,
        Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
    ) -> Result<impl IntoResponse> {
        state
            .user_service
            .remove_from_tenant(StringUuid::from(user_id), StringUuid::from(tenant_id))
            .await?;
        Ok(Json(MessageResponse::new("User removed from tenant")))
    }

    pub async fn get_tenants(
        State(state): State<TestAppState>,
        Path(user_id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let tenants = state
            .user_service
            .get_user_tenants(StringUuid::from(user_id))
            .await?;
        Ok(Json(SuccessResponse::new(tenants)))
    }

    pub async fn list_by_tenant(
        State(state): State<TestAppState>,
        Path(tenant_id): Path<Uuid>,
        Query(pagination): Query<PaginationQuery>,
    ) -> Result<impl IntoResponse> {
        let users = state
            .user_service
            .list_tenant_users(StringUuid::from(tenant_id), pagination.page, pagination.per_page)
            .await?;
        Ok(Json(SuccessResponse::new(users)))
    }

    pub async fn enable_mfa(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let id = StringUuid::from(id);
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
        Ok(Json(SuccessResponse::new(updated)))
    }

    pub async fn disable_mfa(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let id = StringUuid::from(id);
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
        Ok(Json(SuccessResponse::new(updated)))
    }
}

/// Service/Client API handlers for testing
pub mod service_handlers {
    use super::TestAppState;
    use auth9_core::api::{MessageResponse, PaginatedResponse, SuccessResponse};
    use auth9_core::domain::{CreateClientInput, CreateServiceInput, UpdateServiceInput};
    use auth9_core::error::Result;
    use auth9_core::keycloak::KeycloakOidcClient;
    use axum::{
        extract::{Path, Query, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    };
    use serde::Deserialize;
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

    fn default_page() -> i64 { 1 }
    fn default_per_page() -> i64 { 20 }

    pub async fn list(
        State(state): State<TestAppState>,
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

    pub async fn get(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let service = state.client_service.get(id).await?;
        Ok(Json(SuccessResponse::new(service)))
    }

    pub async fn create(
        State(state): State<TestAppState>,
        Json(input): Json<CreateServiceInput>,
    ) -> Result<impl IntoResponse> {
        let logout_uris = input.logout_uris.clone().unwrap_or_default();
        let attributes = if logout_uris.is_empty() {
            None
        } else {
            let mut attrs = HashMap::new();
            attrs.insert("post.logout.redirect.uris".to_string(), logout_uris.join(" "));
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
            web_origins: input.base_url.as_ref().map(|url| vec![url.clone()]).unwrap_or_default(),
            attributes,
            public_client: false,
            secret: None,
        };

        let client_uuid = state.keycloak_client.create_oidc_client(&keycloak_client).await?;
        let client_secret = state.keycloak_client.get_client_secret(&client_uuid).await?;

        let service_with_client = state
            .client_service
            .create_with_secret(input, client_secret)
            .await?;

        Ok((StatusCode::CREATED, Json(SuccessResponse::new(service_with_client))))
    }

    pub async fn update(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
        Json(input): Json<UpdateServiceInput>,
    ) -> Result<impl IntoResponse> {
        let service = state.client_service.update(id, input).await?;
        Ok(Json(SuccessResponse::new(service)))
    }

    pub async fn delete(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let clients = state.client_service.list_clients(id).await?;
        for client in clients {
            if let Ok(kc_uuid) = state
                .keycloak_client
                .get_client_uuid_by_client_id(&client.client_id)
                .await
            {
                let _ = state.keycloak_client.delete_oidc_client(&kc_uuid).await;
            }
        }
        state.client_service.delete(id).await?;
        Ok(Json(MessageResponse::new("Service deleted successfully")))
    }

    pub async fn list_clients(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let clients = state.client_service.list_clients(id).await?;
        Ok(Json(SuccessResponse::new(clients)))
    }

    pub async fn create_client(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
        Json(input): Json<CreateClientInput>,
    ) -> Result<impl IntoResponse> {
        let service = state.client_service.get(id).await?;
        let new_client_id = Uuid::new_v4().to_string();

        let logout_uris = service.logout_uris.clone();
        let attributes = if logout_uris.is_empty() {
            None
        } else {
            let mut attrs = HashMap::new();
            attrs.insert("post.logout.redirect.uris".to_string(), logout_uris.join(" "));
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
            enabled: service.status == auth9_core::domain::ServiceStatus::Active,
            protocol: "openid-connect".to_string(),
            base_url: service.base_url.clone(),
            root_url: service.base_url.clone(),
            admin_url: service.base_url.clone(),
            redirect_uris: service.redirect_uris.clone(),
            web_origins: service.base_url.as_ref().map(|url| vec![url.clone()]).unwrap_or_default(),
            attributes,
            public_client: false,
            secret: None,
        };

        let kc_uuid = state.keycloak_client.create_oidc_client(&keycloak_client).await?;
        let client_secret = state.keycloak_client.get_client_secret(&kc_uuid).await?;

        let client_with_secret = state
            .client_service
            .create_client_with_secret(id, new_client_id, client_secret, input.name)
            .await?;

        Ok(Json(SuccessResponse::new(client_with_secret)))
    }

    pub async fn delete_client(
        State(state): State<TestAppState>,
        Path((service_id, client_id)): Path<(Uuid, String)>,
    ) -> Result<impl IntoResponse> {
        let _ = state.client_service.get(service_id).await?;

        if let Ok(kc_uuid) = state
            .keycloak_client
            .get_client_uuid_by_client_id(&client_id)
            .await
        {
            let _ = state.keycloak_client.delete_oidc_client(&kc_uuid).await;
        }

        state.client_service.delete_client(service_id, &client_id).await?;
        Ok(Json(MessageResponse::new("Client deleted successfully")))
    }

    pub async fn regenerate_client_secret(
        State(state): State<TestAppState>,
        Path((service_id, client_id)): Path<(Uuid, String)>,
    ) -> Result<impl IntoResponse> {
        let _ = state.client_service.get(service_id).await?;

        let new_secret = if let Ok(kc_uuid) = state
            .keycloak_client
            .get_client_uuid_by_client_id(&client_id)
            .await
        {
            state.keycloak_client.regenerate_client_secret(&kc_uuid).await?
        } else {
            state.client_service.regenerate_client_secret(&client_id).await?
        };

        Ok(Json(SuccessResponse::new(serde_json::json!({
            "client_id": client_id,
            "client_secret": new_secret
        }))))
    }
}

/// Role/Permission API handlers for testing
pub mod role_handlers {
    use super::TestAppState;
    use auth9_core::api::{MessageResponse, SuccessResponse};
    use auth9_core::domain::{AssignRolesInput, CreatePermissionInput, CreateRoleInput, StringUuid, UpdateRoleInput};
    use auth9_core::error::Result;
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    };
    use serde::Deserialize;
    use uuid::Uuid;

    #[derive(Debug, Deserialize)]
    pub struct AssignPermissionInput {
        pub permission_id: Uuid,
    }

    pub async fn list_permissions(
        State(state): State<TestAppState>,
        Path(service_id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let permissions = state.rbac_service.list_permissions(StringUuid::from(service_id)).await?;
        Ok(Json(SuccessResponse::new(permissions)))
    }

    pub async fn create_permission(
        State(state): State<TestAppState>,
        Json(input): Json<CreatePermissionInput>,
    ) -> Result<impl IntoResponse> {
        let permission = state.rbac_service.create_permission(input).await?;
        Ok((StatusCode::CREATED, Json(SuccessResponse::new(permission))))
    }

    pub async fn delete_permission(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        state.rbac_service.delete_permission(StringUuid::from(id)).await?;
        Ok(Json(MessageResponse::new("Permission deleted successfully")))
    }

    pub async fn list_roles(
        State(state): State<TestAppState>,
        Path(service_id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let roles = state.rbac_service.list_roles(StringUuid::from(service_id)).await?;
        Ok(Json(SuccessResponse::new(roles)))
    }

    pub async fn get_role(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        let role = state.rbac_service.get_role_with_permissions(StringUuid::from(id)).await?;
        Ok(Json(SuccessResponse::new(role)))
    }

    pub async fn create_role(
        State(state): State<TestAppState>,
        Json(input): Json<CreateRoleInput>,
    ) -> Result<impl IntoResponse> {
        let role = state.rbac_service.create_role(input).await?;
        Ok((StatusCode::CREATED, Json(SuccessResponse::new(role))))
    }

    pub async fn update_role(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
        Json(input): Json<UpdateRoleInput>,
    ) -> Result<impl IntoResponse> {
        let role = state.rbac_service.update_role(StringUuid::from(id), input).await?;
        Ok(Json(SuccessResponse::new(role)))
    }

    pub async fn delete_role(
        State(state): State<TestAppState>,
        Path(id): Path<Uuid>,
    ) -> Result<impl IntoResponse> {
        state.rbac_service.delete_role(StringUuid::from(id)).await?;
        Ok(Json(MessageResponse::new("Role deleted successfully")))
    }

    pub async fn assign_permission(
        State(state): State<TestAppState>,
        Path(role_id): Path<Uuid>,
        Json(input): Json<AssignPermissionInput>,
    ) -> Result<impl IntoResponse> {
        state
            .rbac_service
            .assign_permission_to_role(StringUuid::from(role_id), StringUuid::from(input.permission_id))
            .await?;
        Ok(Json(MessageResponse::new("Permission assigned to role")))
    }

    pub async fn remove_permission(
        State(state): State<TestAppState>,
        Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
    ) -> Result<impl IntoResponse> {
        state
            .rbac_service
            .remove_permission_from_role(StringUuid::from(role_id), StringUuid::from(permission_id))
            .await?;
        Ok(Json(MessageResponse::new("Permission removed from role")))
    }

    pub async fn assign_roles(
        State(state): State<TestAppState>,
        Json(input): Json<AssignRolesInput>,
    ) -> Result<impl IntoResponse> {
        state.rbac_service.assign_roles(input, None).await?;
        Ok(Json(MessageResponse::new("Roles assigned successfully")))
    }

    pub async fn get_user_roles(
        State(state): State<TestAppState>,
        Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
    ) -> Result<impl IntoResponse> {
        let roles = state
            .rbac_service
            .get_user_roles(StringUuid::from(user_id), StringUuid::from(tenant_id))
            .await?;
        Ok(Json(SuccessResponse::new(roles)))
    }

    pub async fn get_user_assigned_roles(
        State(state): State<TestAppState>,
        Path((user_id, tenant_id)): Path<(Uuid, Uuid)>,
    ) -> Result<impl IntoResponse> {
        let roles = state
            .rbac_service
            .get_user_role_records(StringUuid::from(user_id), StringUuid::from(tenant_id))
            .await?;
        Ok(Json(SuccessResponse::new(roles)))
    }

    pub async fn unassign_role(
        State(state): State<TestAppState>,
        Path((user_id, tenant_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
    ) -> Result<impl IntoResponse> {
        state
            .rbac_service
            .unassign_role(
                StringUuid::from(user_id),
                StringUuid::from(tenant_id),
                StringUuid::from(role_id),
            )
            .await?;
        Ok(Json(MessageResponse::new("Role unassigned successfully")))
    }
}

// ============================================================================
// Tests for the infrastructure itself
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_test_config() {
        let config = create_test_config("http://localhost:8080");
        assert_eq!(config.keycloak.url, "http://localhost:8080");
        assert_eq!(config.keycloak.realm, "test");
    }

    #[tokio::test]
    async fn test_test_app_state_creation() {
        let state = TestAppState::new("http://localhost:8080");
        assert!(state.config.keycloak.url.contains("localhost"));
    }
}
