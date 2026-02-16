//! Server initialization and routing

use crate::cache::CacheManager;
use crate::config::Config;
use crate::crypto::EncryptionKey;
use crate::domains;
use crate::grpc::interceptor::{ApiKeyAuthenticator, AuthInterceptor};
use crate::grpc::proto::token_exchange_server::TokenExchangeServer;
use crate::grpc::TokenExchangeService;

/// File descriptor set for gRPC reflection
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("auth9_descriptor");
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::{
    action::ActionRepositoryImpl, audit::AuditRepositoryImpl, invitation::InvitationRepositoryImpl,
    linked_identity::LinkedIdentityRepositoryImpl, login_event::LoginEventRepositoryImpl,
    password_reset::PasswordResetRepositoryImpl, rbac::RbacRepositoryImpl,
    security_alert::SecurityAlertRepositoryImpl, service::ServiceRepositoryImpl,
    session::SessionRepositoryImpl, system_settings::SystemSettingsRepositoryImpl,
    tenant::TenantRepositoryImpl, user::UserRepositoryImpl, webhook::WebhookRepositoryImpl,
};
use crate::service::{
    security_detection::SecurityDetectionConfig, tenant::TenantRepositoryBundle,
    user::UserRepositoryBundle, ActionEngine, ActionService, AnalyticsService, BrandingService,
    ClientService, EmailService, EmailTemplateService, IdentityProviderService, InvitationService,
    KeycloakSyncService, PasswordService, RbacService, SecurityDetectionService, SessionService,
    SystemSettingsService, TenantService, UserService, WebAuthnService, WebhookService,
};
use crate::state::{
    HasAnalytics, HasBranding, HasCache, HasDbPool, HasEmailTemplates, HasIdentityProviders,
    HasInvitations, HasPasswordManagement, HasSecurityAlerts, HasServices, HasSessionManagement,
    HasSystemSettings, HasWebAuthn, HasWebhooks,
};
use anyhow::Result;
use axum::{extract::DefaultBodyLimit, routing::get, Router};
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tonic::transport::Server as TonicServer;
use tower::ServiceBuilder;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::info;

use crate::config::CorsConfig;

use crate::middleware::rate_limit::{
    rate_limit_middleware, RateLimitConfig as RateLimitMiddlewareConfig, RateLimitRule,
    RateLimitState,
};
use crate::middleware::require_auth::AuthMiddlewareState;
use crate::middleware::security_headers::security_headers_middleware;

// ============================================================
// Production Service Type Aliases
// ============================================================

/// Production TenantService type with all concrete repository implementations
pub type ProductionTenantService = TenantService<
    TenantRepositoryImpl,
    ServiceRepositoryImpl,
    WebhookRepositoryImpl,
    InvitationRepositoryImpl,
    UserRepositoryImpl,
    RbacRepositoryImpl,
    LoginEventRepositoryImpl,
    SecurityAlertRepositoryImpl,
    ActionRepositoryImpl,
>;

/// Production UserService type with all concrete repository implementations
pub type ProductionUserService = UserService<
    UserRepositoryImpl,
    SessionRepositoryImpl,
    PasswordResetRepositoryImpl,
    LinkedIdentityRepositoryImpl,
    LoginEventRepositoryImpl,
    SecurityAlertRepositoryImpl,
    AuditRepositoryImpl,
    RbacRepositoryImpl,
>;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: MySqlPool,
    pub tenant_service: Arc<ProductionTenantService>,
    pub user_service: Arc<ProductionUserService>,
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
            TenantRepositoryImpl,
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
    pub action_service: Arc<ActionService<ActionRepositoryImpl>>,
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
    type ActionRepo = ActionRepositoryImpl;

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
        Self::ActionRepo,
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

    fn action_service(&self) -> &ActionService<Self::ActionRepo> {
        &self.action_service
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
    type PasswordTenantRepo = TenantRepositoryImpl;

    fn password_service(
        &self,
    ) -> &PasswordService<
        Self::PasswordResetRepo,
        Self::PasswordUserRepo,
        Self::PasswordSystemSettingsRepo,
        Self::PasswordTenantRepo,
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

/// Implement HasDbPool trait for production AppState
impl HasDbPool for AppState {
    fn db_pool(&self) -> &MySqlPool {
        &self.db_pool
    }
}

impl HasCache for AppState {
    type Cache = CacheManager;

    fn cache(&self) -> &Self::Cache {
        &self.cache_manager
    }
}

/// Run the server
pub async fn run(config: Config, prometheus_handle: Option<PrometheusHandle>) -> Result<()> {
    // Create database connection pool
    let db_pool = MySqlPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(config.database.min_connections)
        .acquire_timeout(Duration::from_secs(config.database.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.database.idle_timeout_secs))
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
    let action_repo = Arc::new(ActionRepositoryImpl::new(db_pool.clone()));

    // Create JWT manager
    let jwt_manager = JwtManager::new(config.jwt.clone());

    // Create Keycloak client
    let keycloak_client = KeycloakClient::new(config.keycloak.clone());

    // Create services
    // Create Arc-wrapped Keycloak client for services that need it
    let keycloak_arc = Arc::new(keycloak_client.clone());

    // Create webhook service first (needed for webhook event publishing)
    let webhook_service = Arc::new(WebhookService::new(webhook_repo.clone()));

    // Create ActionEngine (for Auth9 Actions system)
    let action_engine = Arc::new(ActionEngine::with_config(
        action_repo.clone(),
        config.async_action.clone(),
    ));
    info!(
        "ActionEngine initialized (allowed_domains: {:?})",
        config.async_action.allowed_domains
    );

    // Create ActionService
    let action_service = Arc::new(ActionService::new(
        action_repo.clone(),
        Some(action_engine.clone()),
    ));
    info!("ActionService initialized");

    // Create TenantService with repository bundle
    let tenant_repos = TenantRepositoryBundle::new(
        tenant_repo.clone(),
        service_repo.clone(),
        webhook_repo.clone(),
        invitation_repo.clone(),
        user_repo.clone(),
        rbac_repo.clone(),
        login_event_repo.clone(),
        security_alert_repo.clone(),
        action_repo.clone(),
    );
    let tenant_service = Arc::new(TenantService::new(
        tenant_repos,
        Some(cache_manager.clone()),
    ));

    // Create UserService with repository bundle
    let user_repos = UserRepositoryBundle::new(
        user_repo.clone(),
        session_repo.clone(),
        password_reset_repo.clone(),
        linked_identity_repo.clone(),
        login_event_repo.clone(),
        security_alert_repo.clone(),
        audit_repo.clone(),
        rbac_repo.clone(),
    );
    let user_service = Arc::new(UserService::new(
        user_repos,
        Some(keycloak_client.clone()),
        Some(webhook_service.clone()), // webhook event publisher
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

    // Create Keycloak sync service (shared between branding and system settings)
    let keycloak_updater: Arc<dyn crate::service::keycloak_sync::KeycloakRealmUpdater> =
        keycloak_arc.clone();
    let keycloak_sync_service = Arc::new(KeycloakSyncService::new(keycloak_updater));

    // Create system settings service with Keycloak sync
    let system_settings_service = Arc::new(SystemSettingsService::with_sync_service(
        system_settings_repo.clone(),
        encryption_key,
        keycloak_sync_service.clone(),
    ));

    // Create email service
    let email_service = Arc::new(EmailService::new(system_settings_service.clone()));

    // Create email template service
    let email_template_service = Arc::new(EmailTemplateService::new(system_settings_repo.clone()));

    // Create branding service with Keycloak sync
    let branding_service = Arc::new(BrandingService::with_sync_service(
        system_settings_repo.clone(),
        keycloak_sync_service.clone(),
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
    let password_service = Arc::new(PasswordService::with_tenant_repo(
        password_reset_repo.clone(),
        user_repo.clone(),
        email_service.clone(),
        keycloak_arc.clone(),
        tenant_repo.clone(),
        keycloak_sync_service.clone(),
        config.password_reset.hmac_key.clone(),
    ));

    let session_service = Arc::new(SessionService::new(
        session_repo.clone(),
        user_repo.clone(),
        keycloak_arc.clone(),
        Some(webhook_service.clone()), // webhook event publisher
    ));

    // Create WebAuthn service with native passkey support
    let webauthn_repo = Arc::new(crate::repository::webauthn::WebAuthnRepositoryImpl::new(
        db_pool.clone(),
    ));
    let webauthn_instance = {
        let rp_origin = url::Url::parse(&config.webauthn.rp_origin).map_err(|e| {
            anyhow::anyhow!(
                "Invalid WEBAUTHN_RP_ORIGIN '{}': {}",
                config.webauthn.rp_origin,
                e
            )
        })?;
        let builder = webauthn_rs::WebauthnBuilder::new(&config.webauthn.rp_id, &rp_origin)?
            .rp_name(&config.webauthn.rp_name);
        Arc::new(builder.build()?)
    };
    let webauthn_service = Arc::new(WebAuthnService::new(
        webauthn_instance,
        webauthn_repo,
        Arc::new(cache_manager.clone()),
        Some(keycloak_arc.clone()),
        config.webauthn.challenge_ttl_secs,
    ));

    let identity_provider_service = Arc::new(IdentityProviderService::new(
        linked_identity_repo.clone(),
        user_repo.clone(),
        keycloak_arc,
    ));

    let analytics_service = Arc::new(AnalyticsService::new(login_event_repo.clone()));

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
        action_service,
    };

    // Create rate limit state for middleware
    let rate_limit_state = if config.rate_limit.enabled {
        // Convert config to middleware format with login endpoint override
        let mut endpoints = std::collections::HashMap::new();
        // Add strict rate limit for login endpoint (10 requests per minute per IP)
        endpoints.insert(
            "POST:/api/v1/auth/token".to_string(),
            RateLimitRule {
                requests: 10,
                window_secs: 60,
            },
        );
        // Add strict rate limit for password reset (5 requests per minute)
        endpoints.insert(
            "POST:/api/v1/auth/forgot-password".to_string(),
            RateLimitRule {
                requests: 5,
                window_secs: 60,
            },
        );

        let rate_limit_config = RateLimitMiddlewareConfig {
            enabled: true,
            default: RateLimitRule {
                requests: config.rate_limit.default_requests,
                window_secs: config.rate_limit.default_window_secs,
            },
            endpoints,
            tenant_multipliers: std::collections::HashMap::new(),
        };
        RateLimitState::new(
            rate_limit_config,
            cache_manager.get_connection_manager(),
            jwt_manager.clone(),
            config.jwt_tenant_access_allowed_audiences.clone(),
            config.is_production(),
        )
    } else {
        RateLimitState::noop()
    };

    // Create gRPC service (clone cache_manager before move)
    let grpc_service = TokenExchangeService::with_tenant_repo(
        jwt_manager,
        cache_manager,
        user_repo,
        service_repo,
        rbac_repo,
        tenant_repo.clone(),
        config.is_production(),
    )
    .with_audit_repo(audit_repo.clone());

    // Wrap prometheus handle in Arc for sharing
    let prom_handle = Arc::new(prometheus_handle);

    // Build HTTP router with all features and rate limiting
    let app = build_full_router(state, rate_limit_state, prom_handle.clone());

    // Start background metrics tasks (DB pool + business gauges)
    if prom_handle.is_some() {
        let pool_clone = db_pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                interval.tick().await;
                let size = pool_clone.size() as f64;
                let idle = pool_clone.num_idle() as f64;
                metrics::gauge!("auth9_db_pool_connections_active").set(size - idle);
                metrics::gauge!("auth9_db_pool_connections_idle").set(idle);
            }
        });

        let biz_pool = db_pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                // Tenant count
                if let Ok(row) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tenants")
                    .fetch_one(&biz_pool)
                    .await
                {
                    metrics::gauge!("auth9_tenants_active_total").set(row as f64);
                }
                // User count
                if let Ok(row) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
                    .fetch_one(&biz_pool)
                    .await
                {
                    metrics::gauge!("auth9_users_active_total").set(row as f64);
                }
                // Session count
                if let Ok(row) = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM sessions WHERE revoked_at IS NULL",
                )
                .fetch_one(&biz_pool)
                .await
                {
                    metrics::gauge!("auth9_sessions_active_total").set(row as f64);
                }
            }
        });
    }

    // Get addresses
    let http_addr = config.http_addr();
    let grpc_addr = config.grpc_addr();

    // Log security configuration warnings
    if !config.rate_limit.enabled {
        tracing::warn!("⚠️  Rate limiting is DISABLED. This is a security risk in production!");
    } else {
        info!(
            "Rate limiting enabled: {} requests per {} seconds",
            config.rate_limit.default_requests, config.rate_limit.default_window_secs
        );
    }

    // Warn about gRPC authentication mode
    if config.grpc_security.auth_mode == "none" {
        tracing::warn!(
            "⚠️  gRPC authentication is DISABLED. Set GRPC_AUTH_MODE=api_key for production."
        );
    }

    if config.cors.allowed_origins.len() == 1 && config.cors.allowed_origins[0] == "*" {
        tracing::warn!(
            "CORS is configured with wildcard (*). Set CORS_ALLOWED_ORIGINS for production."
        );
    } else {
        info!("CORS allowed origins: {:?}", config.cors.allowed_origins);
    }

    // Log server resource limits
    info!(
        "Server resource limits: body={}KB, concurrency={}, timeout={}s",
        config.server.body_limit_bytes / 1024,
        config.server.concurrency_limit,
        config.server.request_timeout_secs
    );
    info!(
        "DB pool: max={}, min={}, acquire_timeout={}s, idle_timeout={}s",
        config.database.max_connections,
        config.database.min_connections,
        config.database.acquire_timeout_secs,
        config.database.idle_timeout_secs
    );

    // Run HTTP and gRPC servers concurrently
    let http_server = async {
        let listener = TcpListener::bind(&http_addr).await?;
        info!("HTTP server started on {}", http_addr);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;
        Ok::<_, anyhow::Error>(())
    };

    // Create gRPC authentication interceptor based on config
    let grpc_auth_interceptor = create_grpc_auth_interceptor(&config)?;

    let grpc_server = async {
        use anyhow::Context as _;

        let addr = grpc_addr.parse()?;

        // Load TLS configuration if mTLS mode is enabled
        let tls_config = if config.grpc_security.auth_mode == "mtls" {
            let cert_path = config
                .grpc_security
                .tls_cert_path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("mTLS mode requires GRPC_TLS_CERT_PATH"))?;
            let key_path = config
                .grpc_security
                .tls_key_path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("mTLS mode requires GRPC_TLS_KEY_PATH"))?;
            let ca_cert_path = config
                .grpc_security
                .tls_ca_cert_path
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("mTLS mode requires GRPC_TLS_CA_CERT_PATH"))?;

            info!(
                "Loading gRPC mTLS certificates: cert={}, key={}, ca={}",
                cert_path, key_path, ca_cert_path
            );

            let cert = tokio::fs::read(cert_path)
                .await
                .context("Failed to read TLS certificate")?;
            let key = tokio::fs::read(key_path)
                .await
                .context("Failed to read TLS private key")?;
            let ca_cert = tokio::fs::read(ca_cert_path)
                .await
                .context("Failed to read CA certificate")?;

            let identity = tonic::transport::Identity::from_pem(&cert, &key);
            let ca = tonic::transport::Certificate::from_pem(&ca_cert);

            let tls = tonic::transport::ServerTlsConfig::new()
                .identity(identity)
                .client_ca_root(ca);

            info!(
                "gRPC server starting with mTLS on {} (client verification enabled)",
                grpc_addr
            );
            Some(tls)
        } else {
            info!(
                "gRPC server starting on {} (auth_mode: {}, reflection: {})",
                grpc_addr, config.grpc_security.auth_mode, config.grpc_security.enable_reflection
            );
            None
        };

        // Build server with optional TLS
        let mut server_builder = if let Some(tls) = tls_config {
            TonicServer::builder()
                .tls_config(tls)
                .context("Failed to configure TLS")?
        } else {
            TonicServer::builder()
        };

        // Add services based on configuration
        if config.grpc_security.enable_reflection {
            let reflection_service = tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
                .build_v1()?;
            info!("gRPC reflection enabled");

            server_builder
                .add_service(reflection_service)
                .add_service(TokenExchangeServer::with_interceptor(
                    grpc_service,
                    grpc_auth_interceptor,
                ))
                .serve_with_shutdown(addr, shutdown_signal())
                .await?;
        } else {
            server_builder
                .add_service(TokenExchangeServer::with_interceptor(
                    grpc_service,
                    grpc_auth_interceptor,
                ))
                .serve_with_shutdown(addr, shutdown_signal())
                .await?;
        }

        Ok::<_, anyhow::Error>(())
    };

    tokio::try_join!(http_server, grpc_server)?;

    Ok(())
}

/// Create gRPC authentication interceptor based on configuration
fn create_grpc_auth_interceptor(config: &Config) -> Result<AuthInterceptor> {
    match config.grpc_security.auth_mode.as_str() {
        "api_key" => {
            if config.grpc_security.api_keys.is_empty() {
                if config.is_production() {
                    anyhow::bail!(
                        "gRPC auth_mode is 'api_key' but no API keys configured (GRPC_API_KEYS)"
                    );
                }
                tracing::warn!("gRPC auth_mode is 'api_key' but no API keys configured. Falling back to no auth (non-production).");
                Ok(AuthInterceptor::noop())
            } else {
                info!(
                    "gRPC authentication enabled: API key mode ({} keys configured)",
                    config.grpc_security.api_keys.len()
                );
                let authenticator = ApiKeyAuthenticator::new(config.grpc_security.api_keys.clone());
                Ok(AuthInterceptor::api_key(authenticator))
            }
        }
        "mtls" => {
            // mTLS is handled at the transport layer, not as an interceptor
            // For now, we just log and use noop (mTLS validation happens in TLS handshake)
            info!("gRPC authentication enabled: mTLS mode");
            Ok(AuthInterceptor::noop())
        }
        "none" => {
            if config.is_production() {
                anyhow::bail!("gRPC authentication is disabled (GRPC_AUTH_MODE=none)");
            }
            info!("gRPC authentication disabled (non-production)");
            Ok(AuthInterceptor::noop())
        }
        other => {
            if config.is_production() {
                anyhow::bail!("Unknown gRPC auth_mode '{}'", other);
            }
            tracing::warn!(
                "Unknown gRPC auth_mode '{}'. Falling back to no auth (non-production).",
                other
            );
            Ok(AuthInterceptor::noop())
        }
    }
}

/// Wait for a shutdown signal (Ctrl+C or SIGTERM).
///
/// Each call registers independent signal listeners, so both the HTTP and gRPC
/// servers can await their own copy of this future.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown");
}

const CORS_ALLOW_METHODS: &str = "GET,POST,PUT,DELETE,PATCH,OPTIONS";
const CORS_ALLOW_HEADERS: &str = "authorization,content-type,accept,origin,x-tenant-id,x-api-key";

/// Custom CORS middleware service that only returns CORS headers when the origin matches.
#[derive(Clone)]
struct CorsMiddleware<S> {
    inner: S,
    allowed_origins: Arc<Vec<String>>,
    allow_credentials: bool,
    is_wildcard: bool,
}

impl<S> tower::Service<axum::http::Request<axum::body::Body>> for CorsMiddleware<S>
where
    S: tower::Service<axum::http::Request<axum::body::Body>, Response = axum::response::Response>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = axum::response::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: axum::http::Request<axum::body::Body>) -> Self::Future {
        use axum::http::{header, HeaderValue, Method};
        use axum::response::IntoResponse;

        let allowed_origins = self.allowed_origins.clone();
        let allow_credentials = self.allow_credentials;
        let is_wildcard = self.is_wildcard;

        let origin = req
            .headers()
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let is_preflight = req.method() == Method::OPTIONS;

        // Check if origin is allowed
        let origin_allowed = match &origin {
            Some(o) if is_wildcard => Some(o.clone()),
            Some(o) if allowed_origins.iter().any(|allowed| allowed == o) => Some(o.clone()),
            _ => None,
        };

        let set_cors_headers = move |headers: &mut axum::http::HeaderMap, matched_origin: &str| {
            if is_wildcard {
                headers.insert(
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );
            } else {
                if let Ok(val) = HeaderValue::from_str(matched_origin) {
                    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, val);
                }
                if allow_credentials {
                    headers.insert(
                        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                        HeaderValue::from_static("true"),
                    );
                }
            }
            headers.insert(header::VARY, HeaderValue::from_static("origin"));
        };

        if is_preflight {
            return Box::pin(async move {
                let mut response = axum::http::StatusCode::OK.into_response();
                if let Some(matched_origin) = origin_allowed {
                    let headers = response.headers_mut();
                    set_cors_headers(headers, &matched_origin);
                    headers.insert(
                        header::ACCESS_CONTROL_ALLOW_METHODS,
                        HeaderValue::from_static(CORS_ALLOW_METHODS),
                    );
                    headers.insert(
                        header::ACCESS_CONTROL_ALLOW_HEADERS,
                        HeaderValue::from_static(CORS_ALLOW_HEADERS),
                    );
                }
                Ok(response)
            });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move {
            let mut response = inner.call(req).await?;
            if let Some(matched_origin) = origin_allowed {
                set_cors_headers(response.headers_mut(), &matched_origin);
            }
            Ok(response)
        })
    }
}

/// Layer that wraps services with CorsMiddleware
#[derive(Clone)]
struct CorsLayer {
    allowed_origins: Arc<Vec<String>>,
    allow_credentials: bool,
    is_wildcard: bool,
}

impl<S> tower::Layer<S> for CorsLayer {
    type Service = CorsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CorsMiddleware {
            inner,
            allowed_origins: self.allowed_origins.clone(),
            allow_credentials: self.allow_credentials,
            is_wildcard: self.is_wildcard,
        }
    }
}

/// Build a custom CORS layer that only returns CORS headers when the origin matches.
fn build_cors_layer(config: &CorsConfig) -> CorsLayer {
    let is_wildcard = config.allowed_origins.len() == 1 && config.allowed_origins[0] == "*";
    CorsLayer {
        allowed_origins: Arc::new(config.allowed_origins.clone()),
        allow_credentials: config.allow_credentials && !is_wildcard,
        is_wildcard,
    }
}

/// Build the full HTTP router with all features (including system settings and invitations)
///
/// This function requires the state to implement both HasServices and the new traits.
/// Routes are split into public (no auth) and protected (auth required) groups.
pub fn build_full_router<S>(
    state: S,
    rate_limit_state: RateLimitState,
    prometheus_handle: Arc<Option<PrometheusHandle>>,
) -> Router
where
    S: domains::DomainRouterState + HasServices + HasCache,
{
    // Get CORS configuration from state
    let cors_config = state.config().cors.clone();
    let cors = build_cors_layer(&cors_config);

    let security_headers_config = state.config().security_headers.clone();

    // Create auth middleware state with cache for token blacklist checking
    let auth_state = AuthMiddlewareState::new(
        HasServices::jwt_manager(&state).clone(),
        state.config().jwt_tenant_access_allowed_audiences.clone(),
        state.config().is_production(),
    )
    .with_cache(std::sync::Arc::new(state.cache().clone()));

    // ============================================================
    // PUBLIC ROUTES (no authentication required)
    // ============================================================
    let public_routes = Router::new()
        .merge(domains::security_observability::routes::public_routes::<S>())
        .merge(domains::identity::routes::public_routes::<S>())
        .merge(domains::platform::routes::public_routes::<S>())
        .merge(domains::integration::routes::public_routes::<S>())
        .merge(domains::tenant_access::routes::public_routes::<S>());

    // ============================================================
    // PROTECTED ROUTES (authentication required)
    // ============================================================
    let protected_routes = Router::new()
        .merge(domains::identity::routes::protected_routes::<S>())
        .merge(domains::tenant_access::routes::protected_routes::<S>())
        .merge(domains::authorization::routes::protected_routes::<S>())
        .merge(domains::platform::routes::protected_routes::<S>())
        .merge(domains::integration::routes::protected_routes::<S>())
        .merge(domains::security_observability::routes::protected_routes::<S>())
        // Apply authentication middleware to all protected routes
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            crate::middleware::require_auth::require_auth_middleware,
        ));

    // ============================================================
    // METRICS ENDPOINT (separate state, nested router)
    // When METRICS_TOKEN is set, requires Bearer token auth.
    // In production without METRICS_TOKEN, returns 404.
    // ============================================================
    let metrics_state = crate::api::metrics::MetricsState {
        handle: prometheus_handle.clone(),
        required_token: state.config().telemetry.metrics_token.clone(),
    };

    // Require METRICS_TOKEN to be set for metrics access in all environments.
    // Without a token, the endpoint is disabled to prevent information disclosure.
    let effective_metrics_state = if metrics_state.required_token.is_none() {
        if state.config().is_production() {
            tracing::warn!("⚠️  /metrics endpoint disabled (set METRICS_TOKEN to enable)");
        } else {
            tracing::info!("ℹ️  /metrics endpoint disabled (set METRICS_TOKEN to enable)");
        }
        crate::api::metrics::MetricsState {
            handle: Arc::new(None), // Disable metrics endpoint
            required_token: None,
        }
    } else {
        metrics_state
    };

    let metrics_route: Router<()> = Router::new()
        .route("/metrics", get(crate::api::metrics::metrics_handler))
        .with_state(effective_metrics_state);

    // Server resource limits
    let body_limit = state.config().server.body_limit_bytes;
    let concurrency_limit = state.config().server.concurrency_limit;
    let request_timeout = Duration::from_secs(state.config().server.request_timeout_secs);

    // ============================================================
    // COMBINE ROUTES AND APPLY GLOBAL MIDDLEWARE
    // ============================================================
    // Layers are applied bottom-to-top: the last `.layer()` call is the outermost.
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // Explicit fallback so unmatched routes pass through middleware layers
        // (axum skips .layer() middleware for its implicit 404 fallback)
        .fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "Not Found") })
        // --- Innermost layers (run first on request, last on response) ---
        // 0. Path traversal guard - reject requests with `..` in path
        .layer(axum::middleware::from_fn(
            crate::middleware::path_guard::path_guard_middleware,
        ))
        // 1. Body size limit - reject oversized request bodies (prevents OOM)
        .layer(DefaultBodyLimit::max(body_limit))
        // 2. Error response normalization - consistent JSON error format
        //    (must be inner to security headers so headers are added AFTER
        //     normalization potentially replaces the response object)
        .layer(axum::middleware::from_fn(
            crate::middleware::normalize_error_response,
        ))
        // 3. Security headers - adds security headers to all responses
        .layer(axum::middleware::from_fn_with_state(
            security_headers_config,
            security_headers_middleware,
        ))
        // 4. Tracing - for request logging
        .layer(TraceLayer::new_for_http())
        // 5. Request ID + HTTP Metrics - record request count/duration/in-flight
        .layer(crate::middleware::metrics::ObservabilityLayer)
        // 6. Request timeout - return 408 if handler exceeds limit (prevents slow-loris)
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        // 7. Rate limiting - reject excessive requests early
        .layer(axum::middleware::from_fn_with_state(
            rate_limit_state,
            rate_limit_middleware,
        ))
        // 8. Concurrency limit with load shedding - returns 503 when at capacity.
        //    HandleErrorLayer converts tower BoxError → HTTP response.
        //    load_shed() rejects immediately when inner service is not ready.
        //    concurrency_limit() caps in-flight requests via semaphore.
        .layer(
            ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(
                    |_: tower::BoxError| async { axum::http::StatusCode::SERVICE_UNAVAILABLE },
                ))
                .load_shed()
                .concurrency_limit(concurrency_limit),
        )
        // --- Outermost layer (runs first on response, last on request) ---
        // 9. CORS - must be outermost for preflight requests
        .layer(cors)
        .with_state(state)
        // Nest the metrics route outside .with_state() since it uses its own state
        .merge(metrics_route)
}
