//! Server initialization and routing

use crate::api;
use crate::cache::CacheManager;
use crate::config::Config;
use crate::crypto::EncryptionKey;
use crate::grpc::interceptor::{ApiKeyAuthenticator, AuthInterceptor};
use crate::grpc::proto::token_exchange_server::TokenExchangeServer;
use crate::grpc::TokenExchangeService;
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::{
    audit::AuditRepositoryImpl, invitation::InvitationRepositoryImpl, rbac::RbacRepositoryImpl,
    service::ServiceRepositoryImpl, system_settings::SystemSettingsRepositoryImpl,
    tenant::TenantRepositoryImpl, user::UserRepositoryImpl,
};
use crate::service::{
    BrandingService, ClientService, EmailService, EmailTemplateService, InvitationService,
    RbacService, SystemSettingsService, TenantService, UserService,
};
use crate::state::{
    HasBranding, HasEmailTemplates, HasInvitations, HasServices, HasSystemSettings,
};
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
    pub system_settings_service: Arc<SystemSettingsService<SystemSettingsRepositoryImpl>>,
    pub email_service: Arc<EmailService<SystemSettingsRepositoryImpl>>,
    pub email_template_service: Arc<EmailTemplateService<SystemSettingsRepositoryImpl>>,
    pub invitation_service: Arc<
        InvitationService<
            InvitationRepositoryImpl,
            TenantRepositoryImpl,
            SystemSettingsRepositoryImpl,
        >,
    >,
    pub branding_service: Arc<BrandingService<SystemSettingsRepositoryImpl>>,
}

/// Implement HasServices trait for production AppState
impl HasServices for AppState {
    type TenantRepo = TenantRepositoryImpl;
    type UserRepo = UserRepositoryImpl;
    type ServiceRepo = ServiceRepositoryImpl;
    type RbacRepo = RbacRepositoryImpl;
    type AuditRepo = AuditRepositoryImpl;

    fn config(&self) -> &Config {
        &self.config
    }

    fn tenant_service(&self) -> &TenantService<Self::TenantRepo> {
        &self.tenant_service
    }

    fn user_service(&self) -> &UserService<Self::UserRepo> {
        &self.user_service
    }

    fn client_service(&self) -> &ClientService<Self::ServiceRepo> {
        &self.client_service
    }

    fn rbac_service(&self) -> &RbacService<Self::RbacRepo> {
        &self.rbac_service
    }

    fn audit_repo(&self) -> &Self::AuditRepo {
        &self.audit_repo
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }

    fn keycloak_client(&self) -> &KeycloakClient {
        &self.keycloak_client
    }

    async fn check_ready(&self) -> (bool, bool) {
        let db_ok = sqlx::query("SELECT 1").execute(&self.db_pool).await.is_ok();
        let cache_ok = self.cache_manager.ping().await.is_ok();
        (db_ok, cache_ok)
    }
}

/// Implement HasSystemSettings trait for production AppState
impl HasSystemSettings for AppState {
    type SystemSettingsRepo = SystemSettingsRepositoryImpl;

    fn system_settings_service(&self) -> &SystemSettingsService<Self::SystemSettingsRepo> {
        &self.system_settings_service
    }

    fn email_service(&self) -> &EmailService<Self::SystemSettingsRepo> {
        &self.email_service
    }
}

/// Implement HasInvitations trait for production AppState
impl HasInvitations for AppState {
    type InvitationRepo = InvitationRepositoryImpl;

    fn invitation_service(
        &self,
    ) -> &InvitationService<Self::InvitationRepo, Self::TenantRepo, Self::SystemSettingsRepo> {
        &self.invitation_service
    }
}

/// Implement HasEmailTemplates trait for production AppState
impl HasEmailTemplates for AppState {
    fn email_template_service(&self) -> &EmailTemplateService<Self::SystemSettingsRepo> {
        &self.email_template_service
    }
}

/// Implement HasBranding trait for production AppState
impl HasBranding for AppState {
    type BrandingRepo = SystemSettingsRepositoryImpl;

    fn branding_service(&self) -> &BrandingService<Self::BrandingRepo> {
        &self.branding_service
    }
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
    let system_settings_repo = Arc::new(SystemSettingsRepositoryImpl::new(db_pool.clone()));
    let invitation_repo = Arc::new(InvitationRepositoryImpl::new(db_pool.clone()));

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

    // Load encryption key for settings (optional)
    let encryption_key = EncryptionKey::from_env().ok();
    if encryption_key.is_none() {
        info!("SETTINGS_ENCRYPTION_KEY not set, sensitive settings will not be encrypted");
    }

    // Create system settings service
    let system_settings_service = Arc::new(SystemSettingsService::new(
        system_settings_repo.clone(),
        encryption_key,
    ));

    // Create email service
    let email_service = Arc::new(EmailService::new(system_settings_service.clone()));

    // Create email template service
    let email_template_service = Arc::new(EmailTemplateService::new(system_settings_repo.clone()));

    // Create branding service
    let branding_service = Arc::new(BrandingService::new(system_settings_repo.clone()));

    // Get app base URL for invitation links
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create invitation service
    let invitation_service = Arc::new(InvitationService::new(
        invitation_repo,
        tenant_repo.clone(),
        email_service.clone(),
        app_base_url,
    ));

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
        system_settings_service,
        email_service,
        email_template_service,
        invitation_service,
        branding_service,
    };

    // Create gRPC service
    let grpc_service = TokenExchangeService::new(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
    );

    // Build HTTP router with all features
    let app = build_full_router(state);

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

    // Create gRPC authentication interceptor based on config
    let grpc_auth_interceptor = create_grpc_auth_interceptor(&config.grpc_security);

    let grpc_server = async {
        let addr = grpc_addr.parse()?;
        info!(
            "gRPC server started on {} (auth_mode: {})",
            grpc_addr, config.grpc_security.auth_mode
        );
        TonicServer::builder()
            .add_service(TokenExchangeServer::with_interceptor(
                grpc_service,
                grpc_auth_interceptor,
            ))
            .serve(addr)
            .await?;
        Ok::<_, anyhow::Error>(())
    };

    tokio::try_join!(http_server, grpc_server)?;

    Ok(())
}

/// Create gRPC authentication interceptor based on configuration
fn create_grpc_auth_interceptor(
    config: &crate::config::GrpcSecurityConfig,
) -> AuthInterceptor {
    match config.auth_mode.as_str() {
        "api_key" => {
            if config.api_keys.is_empty() {
                tracing::warn!(
                    "gRPC auth_mode is 'api_key' but no API keys configured. Falling back to no auth."
                );
                AuthInterceptor::noop()
            } else {
                info!(
                    "gRPC authentication enabled: API key mode ({} keys configured)",
                    config.api_keys.len()
                );
                let authenticator = ApiKeyAuthenticator::new(config.api_keys.clone());
                AuthInterceptor::api_key(authenticator)
            }
        }
        "mtls" => {
            // mTLS is handled at the transport layer, not as an interceptor
            // For now, we just log and use noop (mTLS validation happens in TLS handshake)
            info!("gRPC authentication enabled: mTLS mode");
            AuthInterceptor::noop()
        }
        "none" => {
            info!("gRPC authentication disabled");
            AuthInterceptor::noop()
        }
        other => {
            tracing::warn!(
                "Unknown gRPC auth_mode '{}'. Falling back to no auth.",
                other
            );
            AuthInterceptor::noop()
        }
    }
}

/// Build the HTTP router with generic state type
///
/// This function is generic over the state type, allowing it to work with
/// both production `AppState` and test implementations that implement `HasServices`.
pub fn build_router<S: HasServices>(state: S) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health endpoints
        .route("/health", get(api::health::health))
        .route("/ready", get(api::health::ready::<S>))
        // OpenID Connect Discovery
        .route(
            "/.well-known/openid-configuration",
            get(api::auth::openid_configuration::<S>),
        )
        .route("/.well-known/jwks.json", get(api::auth::jwks::<S>))
        // Auth endpoints
        .route("/api/v1/auth/authorize", get(api::auth::authorize::<S>))
        .route("/api/v1/auth/callback", get(api::auth::callback::<S>))
        .route("/api/v1/auth/token", post(api::auth::token::<S>))
        .route("/api/v1/auth/logout", get(api::auth::logout::<S>))
        .route("/api/v1/auth/userinfo", get(api::auth::userinfo::<S>))
        // Tenant endpoints
        .route(
            "/api/v1/tenants",
            get(api::tenant::list::<S>).post(api::tenant::create::<S>),
        )
        .route(
            "/api/v1/tenants/{id}",
            get(api::tenant::get::<S>)
                .put(api::tenant::update::<S>)
                .delete(api::tenant::delete::<S>),
        )
        // User endpoints
        .route(
            "/api/v1/users",
            get(api::user::list::<S>).post(api::user::create::<S>),
        )
        .route(
            "/api/v1/users/{id}",
            get(api::user::get::<S>)
                .put(api::user::update::<S>)
                .delete(api::user::delete::<S>),
        )
        .route(
            "/api/v1/users/{id}/mfa",
            post(api::user::enable_mfa::<S>).delete(api::user::disable_mfa::<S>),
        )
        .route(
            "/api/v1/users/{id}/tenants",
            get(api::user::get_tenants::<S>).post(api::user::add_to_tenant::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}",
            delete(api::user::remove_from_tenant::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/users",
            get(api::user::list_by_tenant::<S>),
        )
        // Service endpoints
        .route(
            "/api/v1/services",
            get(api::service::list::<S>).post(api::service::create::<S>),
        )
        .route(
            "/api/v1/services/{id}",
            get(api::service::get::<S>)
                .put(api::service::update::<S>)
                .delete(api::service::delete::<S>),
        )
        // .route(
        //     "/api/v1/services/:id/secret",
        //     post(api::service::regenerate_secret),
        // )
        .route(
            "/api/v1/services/{id}/clients",
            get(api::service::list_clients::<S>).post(api::service::create_client::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/clients/{client_id}",
            delete(api::service::delete_client::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/clients/{client_id}/regenerate-secret",
            post(api::service::regenerate_client_secret::<S>),
        )
        // Permission endpoints
        .route(
            "/api/v1/permissions",
            post(api::role::create_permission::<S>),
        )
        .route(
            "/api/v1/permissions/{id}",
            delete(api::role::delete_permission::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/permissions",
            get(api::role::list_permissions::<S>),
        )
        // Role endpoints
        .route("/api/v1/roles", post(api::role::create_role::<S>))
        .route(
            "/api/v1/roles/{id}",
            get(api::role::get_role::<S>)
                .put(api::role::update_role::<S>)
                .delete(api::role::delete_role::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/roles",
            get(api::role::list_roles::<S>),
        )
        .route(
            "/api/v1/roles/{role_id}/permissions",
            post(api::role::assign_permission::<S>),
        )
        .route(
            "/api/v1/roles/{role_id}/permissions/{permission_id}",
            delete(api::role::remove_permission::<S>),
        )
        // RBAC assignment
        .route("/api/v1/rbac/assign", post(api::role::assign_roles::<S>))
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}/roles",
            get(api::role::get_user_roles::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}/assigned-roles",
            get(api::role::get_user_assigned_roles::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}/roles/{role_id}",
            delete(api::role::unassign_role::<S>),
        )
        // Audit logs
        .route("/api/v1/audit-logs", get(api::audit::list::<S>))
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Build the full HTTP router with all features (including system settings and invitations)
///
/// This function requires the state to implement both HasServices and the new traits.
pub fn build_full_router<S>(state: S) -> Router
where
    S: HasServices + HasSystemSettings + HasInvitations + HasEmailTemplates + HasBranding,
{
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Start with the base router routes (manually copied since we can't use generic build_router)
    Router::new()
        // Health endpoints
        .route("/health", get(api::health::health))
        .route("/ready", get(api::health::ready::<S>))
        // OpenID Connect Discovery
        .route(
            "/.well-known/openid-configuration",
            get(api::auth::openid_configuration::<S>),
        )
        .route("/.well-known/jwks.json", get(api::auth::jwks::<S>))
        // Auth endpoints
        .route("/api/v1/auth/authorize", get(api::auth::authorize::<S>))
        .route("/api/v1/auth/callback", get(api::auth::callback::<S>))
        .route("/api/v1/auth/token", post(api::auth::token::<S>))
        .route("/api/v1/auth/logout", get(api::auth::logout::<S>))
        .route("/api/v1/auth/userinfo", get(api::auth::userinfo::<S>))
        // Tenant endpoints
        .route(
            "/api/v1/tenants",
            get(api::tenant::list::<S>).post(api::tenant::create::<S>),
        )
        .route(
            "/api/v1/tenants/{id}",
            get(api::tenant::get::<S>)
                .put(api::tenant::update::<S>)
                .delete(api::tenant::delete::<S>),
        )
        // User endpoints
        .route(
            "/api/v1/users",
            get(api::user::list::<S>).post(api::user::create::<S>),
        )
        .route(
            "/api/v1/users/{id}",
            get(api::user::get::<S>)
                .put(api::user::update::<S>)
                .delete(api::user::delete::<S>),
        )
        .route(
            "/api/v1/users/{id}/mfa",
            post(api::user::enable_mfa::<S>).delete(api::user::disable_mfa::<S>),
        )
        .route(
            "/api/v1/users/{id}/tenants",
            get(api::user::get_tenants::<S>).post(api::user::add_to_tenant::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}",
            delete(api::user::remove_from_tenant::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/users",
            get(api::user::list_by_tenant::<S>),
        )
        // Service endpoints
        .route(
            "/api/v1/services",
            get(api::service::list::<S>).post(api::service::create::<S>),
        )
        .route(
            "/api/v1/services/{id}",
            get(api::service::get::<S>)
                .put(api::service::update::<S>)
                .delete(api::service::delete::<S>),
        )
        .route(
            "/api/v1/services/{id}/clients",
            get(api::service::list_clients::<S>).post(api::service::create_client::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/clients/{client_id}",
            delete(api::service::delete_client::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/clients/{client_id}/regenerate-secret",
            post(api::service::regenerate_client_secret::<S>),
        )
        // Permission endpoints
        .route(
            "/api/v1/permissions",
            post(api::role::create_permission::<S>),
        )
        .route(
            "/api/v1/permissions/{id}",
            delete(api::role::delete_permission::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/permissions",
            get(api::role::list_permissions::<S>),
        )
        // Role endpoints
        .route("/api/v1/roles", post(api::role::create_role::<S>))
        .route(
            "/api/v1/roles/{id}",
            get(api::role::get_role::<S>)
                .put(api::role::update_role::<S>)
                .delete(api::role::delete_role::<S>),
        )
        .route(
            "/api/v1/services/{service_id}/roles",
            get(api::role::list_roles::<S>),
        )
        .route(
            "/api/v1/roles/{role_id}/permissions",
            post(api::role::assign_permission::<S>),
        )
        .route(
            "/api/v1/roles/{role_id}/permissions/{permission_id}",
            delete(api::role::remove_permission::<S>),
        )
        // RBAC assignment
        .route("/api/v1/rbac/assign", post(api::role::assign_roles::<S>))
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}/roles",
            get(api::role::get_user_roles::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}/assigned-roles",
            get(api::role::get_user_assigned_roles::<S>),
        )
        .route(
            "/api/v1/users/{user_id}/tenants/{tenant_id}/roles/{role_id}",
            delete(api::role::unassign_role::<S>),
        )
        // Audit logs
        .route("/api/v1/audit-logs", get(api::audit::list::<S>))
        // System settings endpoints (admin only)
        .route(
            "/api/v1/system/email",
            get(api::system_settings::get_email_settings::<S>)
                .put(api::system_settings::update_email_settings::<S>),
        )
        .route(
            "/api/v1/system/email/test",
            post(api::system_settings::test_email_connection::<S>),
        )
        .route(
            "/api/v1/system/email/send-test",
            post(api::system_settings::send_test_email::<S>),
        )
        // Email template endpoints
        .route(
            "/api/v1/system/email-templates",
            get(api::email_template::list_templates::<S>),
        )
        .route(
            "/api/v1/system/email-templates/{type}",
            get(api::email_template::get_template::<S>)
                .put(api::email_template::update_template::<S>)
                .delete(api::email_template::reset_template::<S>),
        )
        .route(
            "/api/v1/system/email-templates/{type}/preview",
            post(api::email_template::preview_template::<S>),
        )
        .route(
            "/api/v1/system/email-templates/{type}/send-test",
            post(api::email_template::send_test_email::<S>),
        )
        // Invitation endpoints
        .route(
            "/api/v1/tenants/{tenant_id}/invitations",
            get(api::invitation::list::<S>).post(api::invitation::create::<S>),
        )
        .route(
            "/api/v1/invitations/{id}",
            get(api::invitation::get::<S>).delete(api::invitation::delete::<S>),
        )
        .route(
            "/api/v1/invitations/{id}/revoke",
            post(api::invitation::revoke::<S>),
        )
        .route(
            "/api/v1/invitations/{id}/resend",
            post(api::invitation::resend::<S>),
        )
        // Public endpoint for accepting invitations
        .route(
            "/api/v1/invitations/accept",
            post(api::invitation::accept::<S>),
        )
        // Branding endpoints
        // Public endpoint (no auth required) for Keycloak themes
        .route(
            "/api/v1/public/branding",
            get(api::branding::get_public_branding::<S>),
        )
        // Admin endpoints (auth required)
        .route(
            "/api/v1/system/branding",
            get(api::branding::get_branding::<S>).put(api::branding::update_branding::<S>),
        )
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
