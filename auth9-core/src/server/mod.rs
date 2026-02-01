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
    audit::AuditRepositoryImpl, invitation::InvitationRepositoryImpl,
    linked_identity::LinkedIdentityRepositoryImpl, login_event::LoginEventRepositoryImpl,
    password_reset::PasswordResetRepositoryImpl, rbac::RbacRepositoryImpl,
    security_alert::SecurityAlertRepositoryImpl, service::ServiceRepositoryImpl,
    session::SessionRepositoryImpl, system_settings::SystemSettingsRepositoryImpl,
    tenant::TenantRepositoryImpl, user::UserRepositoryImpl, webhook::WebhookRepositoryImpl,
};
use crate::service::{
    security_detection::SecurityDetectionConfig, AnalyticsService, BrandingService, ClientService,
    EmailService, EmailTemplateService, IdentityProviderService, InvitationService,
    KeycloakSyncService, PasswordService, RbacService, SecurityDetectionService, SessionService,
    SystemSettingsService, TenantService, UserService, WebAuthnService, WebhookService,
};
use crate::state::{
    HasAnalytics, HasBranding, HasEmailTemplates, HasIdentityProviders, HasInvitations,
    HasPasswordManagement, HasSecurityAlerts, HasServices, HasSessionManagement, HasSystemSettings,
    HasWebAuthn, HasWebhooks,
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
    pub tenant_service: Arc<
        TenantService<
            TenantRepositoryImpl,
            ServiceRepositoryImpl,
            WebhookRepositoryImpl,
            InvitationRepositoryImpl,
            UserRepositoryImpl,
            RbacRepositoryImpl,
            LoginEventRepositoryImpl,
            SecurityAlertRepositoryImpl,
        >,
    >,
    pub user_service: Arc<
        UserService<
            UserRepositoryImpl,
            SessionRepositoryImpl,
            PasswordResetRepositoryImpl,
            LinkedIdentityRepositoryImpl,
            LoginEventRepositoryImpl,
            SecurityAlertRepositoryImpl,
            AuditRepositoryImpl,
            RbacRepositoryImpl,
        >,
    >,
    pub client_service: Arc<ClientService<ServiceRepositoryImpl, RbacRepositoryImpl>>,
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
    // New services for 5 features
    pub password_service: Arc<
        PasswordService<
            PasswordResetRepositoryImpl,
            UserRepositoryImpl,
            SystemSettingsRepositoryImpl,
        >,
    >,
    pub session_service: Arc<SessionService<SessionRepositoryImpl, UserRepositoryImpl>>,
    pub webauthn_service: Arc<WebAuthnService>,
    pub identity_provider_service:
        Arc<IdentityProviderService<LinkedIdentityRepositoryImpl, UserRepositoryImpl>>,
    pub analytics_service: Arc<AnalyticsService<LoginEventRepositoryImpl>>,
    pub webhook_service: Arc<WebhookService<WebhookRepositoryImpl>>,
    pub security_detection_service: Arc<
        SecurityDetectionService<
            LoginEventRepositoryImpl,
            SecurityAlertRepositoryImpl,
            WebhookRepositoryImpl,
        >,
    >,
}

/// Implement HasServices trait for production AppState
impl HasServices for AppState {
    type TenantRepo = TenantRepositoryImpl;
    type UserRepo = UserRepositoryImpl;
    type ServiceRepo = ServiceRepositoryImpl;
    type RbacRepo = RbacRepositoryImpl;
    type AuditRepo = AuditRepositoryImpl;
    type SessionRepo = SessionRepositoryImpl;
    type PasswordResetRepo = PasswordResetRepositoryImpl;
    type LinkedIdentityRepo = LinkedIdentityRepositoryImpl;
    type LoginEventRepo = LoginEventRepositoryImpl;
    type SecurityAlertRepo = SecurityAlertRepositoryImpl;
    type WebhookRepo = WebhookRepositoryImpl;
    type CascadeInvitationRepo = InvitationRepositoryImpl;

    fn config(&self) -> &Config {
        &self.config
    }

    fn tenant_service(
        &self,
    ) -> &TenantService<
        Self::TenantRepo,
        Self::ServiceRepo,
        Self::WebhookRepo,
        Self::CascadeInvitationRepo,
        Self::UserRepo,
        Self::RbacRepo,
        Self::LoginEventRepo,
        Self::SecurityAlertRepo,
    > {
        &self.tenant_service
    }

    fn user_service(
        &self,
    ) -> &UserService<
        Self::UserRepo,
        Self::SessionRepo,
        Self::PasswordResetRepo,
        Self::LinkedIdentityRepo,
        Self::LoginEventRepo,
        Self::SecurityAlertRepo,
        Self::AuditRepo,
        Self::RbacRepo,
    > {
        &self.user_service
    }

    fn client_service(&self) -> &ClientService<Self::ServiceRepo, Self::RbacRepo> {
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

/// Implement HasPasswordManagement trait for production AppState
impl HasPasswordManagement for AppState {
    type PasswordResetRepo = PasswordResetRepositoryImpl;
    type PasswordUserRepo = UserRepositoryImpl;
    type PasswordSystemSettingsRepo = SystemSettingsRepositoryImpl;

    fn password_service(
        &self,
    ) -> &PasswordService<
        Self::PasswordResetRepo,
        Self::PasswordUserRepo,
        Self::PasswordSystemSettingsRepo,
    > {
        &self.password_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasSessionManagement trait for production AppState
impl HasSessionManagement for AppState {
    type SessionRepo = SessionRepositoryImpl;
    type SessionUserRepo = UserRepositoryImpl;

    fn session_service(&self) -> &SessionService<Self::SessionRepo, Self::SessionUserRepo> {
        &self.session_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasWebAuthn trait for production AppState
impl HasWebAuthn for AppState {
    fn webauthn_service(&self) -> &WebAuthnService {
        &self.webauthn_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasIdentityProviders trait for production AppState
impl HasIdentityProviders for AppState {
    type LinkedIdentityRepo = LinkedIdentityRepositoryImpl;
    type IdpUserRepo = UserRepositoryImpl;

    fn identity_provider_service(
        &self,
    ) -> &IdentityProviderService<Self::LinkedIdentityRepo, Self::IdpUserRepo> {
        &self.identity_provider_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasAnalytics trait for production AppState
impl HasAnalytics for AppState {
    type LoginEventRepo = LoginEventRepositoryImpl;

    fn analytics_service(&self) -> &AnalyticsService<Self::LoginEventRepo> {
        &self.analytics_service
    }
}

/// Implement HasWebhooks trait for production AppState
impl HasWebhooks for AppState {
    type WebhookRepo = WebhookRepositoryImpl;

    fn webhook_service(&self) -> &WebhookService<Self::WebhookRepo> {
        &self.webhook_service
    }
}

/// Implement HasSecurityAlerts trait for production AppState
impl HasSecurityAlerts for AppState {
    type SecurityLoginEventRepo = LoginEventRepositoryImpl;
    type SecurityAlertRepo = SecurityAlertRepositoryImpl;
    type SecurityWebhookRepo = WebhookRepositoryImpl;

    fn security_detection_service(
        &self,
    ) -> &SecurityDetectionService<
        Self::SecurityLoginEventRepo,
        Self::SecurityAlertRepo,
        Self::SecurityWebhookRepo,
    > {
        &self.security_detection_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
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
    // New repositories for 5 features
    let password_reset_repo = Arc::new(PasswordResetRepositoryImpl::new(db_pool.clone()));
    let session_repo = Arc::new(SessionRepositoryImpl::new(db_pool.clone()));
    let linked_identity_repo = Arc::new(LinkedIdentityRepositoryImpl::new(db_pool.clone()));
    let login_event_repo = Arc::new(LoginEventRepositoryImpl::new(db_pool.clone()));
    let webhook_repo = Arc::new(WebhookRepositoryImpl::new(db_pool.clone()));
    let security_alert_repo = Arc::new(SecurityAlertRepositoryImpl::new(db_pool.clone()));

    // Create JWT manager
    let jwt_manager = JwtManager::new(config.jwt.clone());

    // Create Keycloak client
    let keycloak_client = KeycloakClient::new(config.keycloak.clone());

    // Create services
    // Create Arc-wrapped Keycloak client for services that need it
    let keycloak_arc = Arc::new(keycloak_client.clone());

    let tenant_service = Arc::new(TenantService::new(
        tenant_repo.clone(),
        service_repo.clone(),
        webhook_repo.clone(),
        invitation_repo.clone(),
        user_repo.clone(),
        rbac_repo.clone(),
        login_event_repo.clone(),
        security_alert_repo.clone(),
        Some(cache_manager.clone()),
    ));
    let user_service = Arc::new(UserService::new(
        user_repo.clone(),
        session_repo.clone(),
        password_reset_repo.clone(),
        linked_identity_repo.clone(),
        login_event_repo.clone(),
        security_alert_repo.clone(),
        audit_repo.clone(),
        rbac_repo.clone(),
        Some(keycloak_client.clone()),
    ));
    let client_service = Arc::new(ClientService::new(
        service_repo.clone(),
        rbac_repo.clone(),
        Some(cache_manager.clone()),
    ));
    let rbac_service = Arc::new(RbacService::new(
        rbac_repo.clone(),
        Some(cache_manager.clone()),
    ));

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

    // Create Keycloak sync service for branding sync
    let keycloak_sync_service = Arc::new(KeycloakSyncService::new(keycloak_arc.clone()));

    // Create branding service with Keycloak sync
    let branding_service = Arc::new(BrandingService::with_sync_service(
        system_settings_repo.clone(),
        keycloak_sync_service,
    ));

    // Get app base URL for invitation links
    let app_base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create invitation service
    let invitation_service = Arc::new(InvitationService::new(
        invitation_repo.clone(),
        tenant_repo.clone(),
        email_service.clone(),
        app_base_url.clone(),
    ));

    // Create new services for 5 features
    let password_service = Arc::new(PasswordService::new(
        password_reset_repo.clone(),
        user_repo.clone(),
        email_service.clone(),
        keycloak_arc.clone(),
    ));

    let session_service = Arc::new(SessionService::new(
        session_repo.clone(),
        user_repo.clone(),
        keycloak_arc.clone(),
    ));

    let webauthn_service = Arc::new(WebAuthnService::new(keycloak_arc.clone()));

    let identity_provider_service = Arc::new(IdentityProviderService::new(
        linked_identity_repo.clone(),
        user_repo.clone(),
        keycloak_arc,
    ));

    let analytics_service = Arc::new(AnalyticsService::new(login_event_repo.clone()));

    let webhook_service = Arc::new(WebhookService::new(webhook_repo.clone()));

    let security_detection_service = Arc::new(SecurityDetectionService::new(
        login_event_repo,
        security_alert_repo,
        webhook_service.clone(),
        SecurityDetectionConfig::default(),
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
        // New services for 5 features
        password_service,
        session_service,
        webauthn_service,
        identity_provider_service,
        analytics_service,
        webhook_service,
        security_detection_service,
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
fn create_grpc_auth_interceptor(config: &crate::config::GrpcSecurityConfig) -> AuthInterceptor {
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
    S: HasServices
        + HasSystemSettings
        + HasInvitations
        + HasEmailTemplates
        + HasBranding
        + HasPasswordManagement
        + HasSessionManagement
        + HasWebAuthn
        + HasIdentityProviders
        + HasAnalytics
        + HasWebhooks
        + HasSecurityAlerts,
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
        // === Password Management endpoints ===
        .route(
            "/api/v1/auth/forgot-password",
            post(api::password::forgot_password::<S>),
        )
        .route(
            "/api/v1/auth/reset-password",
            post(api::password::reset_password::<S>),
        )
        .route(
            "/api/v1/users/me/password",
            post(api::password::change_password::<S>),
        )
        .route(
            "/api/v1/tenants/{id}/password-policy",
            get(api::password::get_password_policy::<S>)
                .put(api::password::update_password_policy::<S>),
        )
        // === Session Management endpoints ===
        .route(
            "/api/v1/users/me/sessions",
            get(api::session::list_my_sessions::<S>)
                .delete(api::session::revoke_other_sessions::<S>),
        )
        .route(
            "/api/v1/users/me/sessions/{id}",
            delete(api::session::revoke_session::<S>),
        )
        .route(
            "/api/v1/admin/users/{id}/logout",
            post(api::session::force_logout_user::<S>),
        )
        // === WebAuthn/Passkey endpoints ===
        .route(
            "/api/v1/users/me/passkeys",
            get(api::webauthn::list_passkeys::<S>),
        )
        .route(
            "/api/v1/users/me/passkeys/{id}",
            delete(api::webauthn::delete_passkey::<S>),
        )
        .route(
            "/api/v1/auth/webauthn/register",
            get(api::webauthn::get_register_url::<S>),
        )
        // === Identity Provider endpoints ===
        .route(
            "/api/v1/identity-providers",
            get(api::identity_provider::list_providers::<S>)
                .post(api::identity_provider::create_provider::<S>),
        )
        .route(
            "/api/v1/identity-providers/{alias}",
            get(api::identity_provider::get_provider::<S>)
                .put(api::identity_provider::update_provider::<S>)
                .delete(api::identity_provider::delete_provider::<S>),
        )
        .route(
            "/api/v1/users/me/linked-identities",
            get(api::identity_provider::list_my_linked_identities::<S>),
        )
        .route(
            "/api/v1/users/me/linked-identities/{id}",
            delete(api::identity_provider::unlink_identity::<S>),
        )
        // === Analytics endpoints ===
        .route(
            "/api/v1/analytics/login-stats",
            get(api::analytics::get_stats::<S>),
        )
        .route(
            "/api/v1/analytics/login-events",
            get(api::analytics::list_events::<S>),
        )
        // === Webhook endpoints ===
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks",
            get(api::webhook::list_webhooks::<S>).post(api::webhook::create_webhook::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{id}",
            get(api::webhook::get_webhook::<S>)
                .put(api::webhook::update_webhook::<S>)
                .delete(api::webhook::delete_webhook::<S>),
        )
        .route(
            "/api/v1/tenants/{tenant_id}/webhooks/{id}/test",
            post(api::webhook::test_webhook::<S>),
        )
        // === Security Alert endpoints ===
        .route(
            "/api/v1/security/alerts",
            get(api::security_alert::list_alerts::<S>),
        )
        .route(
            "/api/v1/security/alerts/{id}/resolve",
            post(api::security_alert::resolve_alert::<S>),
        )
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
