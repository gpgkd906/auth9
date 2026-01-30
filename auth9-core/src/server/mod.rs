//! Server initialization and routing

use crate::api;
use crate::cache::CacheManager;
use crate::config::Config;
use crate::grpc::proto::token_exchange_server::TokenExchangeServer;
use crate::grpc::TokenExchangeService;
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::{
    audit::AuditRepositoryImpl, rbac::RbacRepositoryImpl, service::ServiceRepositoryImpl,
    tenant::TenantRepositoryImpl, user::UserRepositoryImpl,
};
use crate::service::{ClientService, RbacService, TenantService, UserService};
use anyhow::Result;
use axum::{
    routing::{delete, get, post},
    Router,
};
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::Server as TonicServer;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: MySqlPool,
    pub tenant_service: Arc<TenantService<TenantRepositoryImpl>>,
    pub user_service: Arc<UserService<UserRepositoryImpl>>,
    pub client_service: Arc<ClientService<ServiceRepositoryImpl>>,
    pub rbac_service: Arc<RbacService<RbacRepositoryImpl>>,
    pub audit_repo: Arc<AuditRepositoryImpl>,
    pub jwt_manager: JwtManager,
    pub cache_manager: CacheManager,
    pub keycloak_client: KeycloakClient,
}

/// Run the server
pub async fn run(config: Config) -> Result<()> {
    // Create database connection pool
    let db_pool = MySqlPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .connect(&config.database.url)
        .await?;

    info!("Connected to database");

    // Create cache manager
    let cache_manager = CacheManager::new(&config.redis).await?;
    info!("Connected to Redis");

    // Create repositories
    let tenant_repo = Arc::new(TenantRepositoryImpl::new(db_pool.clone()));
    let user_repo = Arc::new(UserRepositoryImpl::new(db_pool.clone()));
    let service_repo = Arc::new(ServiceRepositoryImpl::new(db_pool.clone()));
    let rbac_repo = Arc::new(RbacRepositoryImpl::new(db_pool.clone()));
    let audit_repo = Arc::new(AuditRepositoryImpl::new(db_pool.clone()));

    // Create services
    let tenant_service = Arc::new(TenantService::new(
        tenant_repo.clone(),
        Some(cache_manager.clone()),
    ));
    let user_service = Arc::new(UserService::new(user_repo.clone()));
    let client_service = Arc::new(ClientService::new(
        service_repo.clone(),
        Some(cache_manager.clone()),
    ));
    let rbac_service = Arc::new(RbacService::new(
        rbac_repo.clone(),
        Some(cache_manager.clone()),
    ));

    // Create JWT manager
    let jwt_manager = JwtManager::new(config.jwt.clone());

    // Create Keycloak client
    let keycloak_client = KeycloakClient::new(config.keycloak.clone());

    // Create app state
    let state = AppState {
        config: Arc::new(config.clone()),
        db_pool: db_pool.clone(),
        tenant_service,
        user_service,
        client_service,
        rbac_service,
        audit_repo: audit_repo.clone(),
        jwt_manager: jwt_manager.clone(),
        cache_manager: cache_manager.clone(),
        keycloak_client,
    };

    // Create gRPC service
    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    // Build HTTP router
    let app = build_router(state);

    // Get addresses
    let http_addr = config.http_addr();
    let grpc_addr = config.grpc_addr();

    // Run HTTP and gRPC servers concurrently
    let http_server = async {
        let listener = TcpListener::bind(&http_addr).await?;
        info!("HTTP server started on {}", http_addr);
        axum::serve(listener, app).await?;
        Ok::<_, anyhow::Error>(())
    };

    let grpc_server = async {
        let addr = grpc_addr.parse()?;
        info!("gRPC server started on {}", grpc_addr);
        TonicServer::builder()
            .add_service(TokenExchangeServer::new(grpc_service))
            .serve(addr)
            .await?;
        Ok::<_, anyhow::Error>(())
    };

    tokio::try_join!(http_server, grpc_server)?;

    Ok(())
}

/// Build the HTTP router
pub fn build_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health endpoints
        .route("/health", get(api::health::health))
        .route("/ready", get(api::health::ready))
        // OpenID Connect Discovery
        .route(
            "/.well-known/openid-configuration",
            get(api::auth::openid_configuration),
        )
        .route("/.well-known/jwks.json", get(api::auth::jwks))
        // Auth endpoints
        .route("/api/v1/auth/authorize", get(api::auth::authorize))
        .route("/api/v1/auth/callback", get(api::auth::callback))
        .route("/api/v1/auth/token", post(api::auth::token))
        .route("/api/v1/auth/logout", get(api::auth::logout))
        .route("/api/v1/auth/userinfo", get(api::auth::userinfo))
        // Tenant endpoints
        .route(
            "/api/v1/tenants",
            get(api::tenant::list).post(api::tenant::create),
        )
        .route(
            "/api/v1/tenants/:id",
            get(api::tenant::get)
                .put(api::tenant::update)
                .delete(api::tenant::delete),
        )
        // User endpoints
        .route(
            "/api/v1/users",
            get(api::user::list).post(api::user::create),
        )
        .route(
            "/api/v1/users/:id",
            get(api::user::get)
                .put(api::user::update)
                .delete(api::user::delete),
        )
        .route(
            "/api/v1/users/:id/mfa",
            post(api::user::enable_mfa).delete(api::user::disable_mfa),
        )
        .route(
            "/api/v1/users/:id/tenants",
            get(api::user::get_tenants).post(api::user::add_to_tenant),
        )
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id",
            delete(api::user::remove_from_tenant),
        )
        .route(
            "/api/v1/tenants/:tenant_id/users",
            get(api::user::list_by_tenant),
        )
        // Service endpoints
        .route(
            "/api/v1/services",
            get(api::service::list).post(api::service::create),
        )
        .route(
            "/api/v1/services/:id",
            get(api::service::get)
                .put(api::service::update)
                .delete(api::service::delete),
        )
        // .route(
        //     "/api/v1/services/:id/secret",
        //     post(api::service::regenerate_secret),
        // )
        .route(
            "/api/v1/services/:id/clients",
            get(api::service::list_clients).post(api::service::create_client),
        )
        .route(
            "/api/v1/services/:service_id/clients/:client_id",
            delete(api::service::delete_client),
        )
        .route(
            "/api/v1/services/:service_id/clients/:client_id/regenerate-secret",
            post(api::service::regenerate_client_secret),
        )
        // Permission endpoints
        .route("/api/v1/permissions", post(api::role::create_permission))
        .route(
            "/api/v1/permissions/:id",
            delete(api::role::delete_permission),
        )
        .route(
            "/api/v1/services/:service_id/permissions",
            get(api::role::list_permissions),
        )
        // Role endpoints
        .route("/api/v1/roles", post(api::role::create_role))
        .route(
            "/api/v1/roles/:id",
            get(api::role::get_role)
                .put(api::role::update_role)
                .delete(api::role::delete_role),
        )
        .route(
            "/api/v1/services/:service_id/roles",
            get(api::role::list_roles),
        )
        .route(
            "/api/v1/roles/:role_id/permissions",
            post(api::role::assign_permission),
        )
        .route(
            "/api/v1/roles/:role_id/permissions/:permission_id",
            delete(api::role::remove_permission),
        )
        // RBAC assignment
        .route("/api/v1/rbac/assign", post(api::role::assign_roles))
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id/roles",
            get(api::role::get_user_roles),
        )
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id/assigned-roles",
            get(api::role::get_user_assigned_roles),
        )
        .route(
            "/api/v1/users/:user_id/tenants/:tenant_id/roles/:role_id",
            delete(api::role::unassign_role),
        )
        // Audit logs
        .route("/api/v1/audit-logs", get(api::audit::list))
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
