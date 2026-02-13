//! HTTP API Handler Tests Infrastructure
//!
//! This module provides test utilities for HTTP handler testing without
//! external dependencies (no database, no Redis, no Keycloak).
//!
//! Key components:
//! - `TestAppState` - Test-friendly version of AppState implementing `HasServices`
//! - Uses production `build_full_router()` with `TestAppState` for actual handler coverage
//! - Helper functions for making HTTP requests (get_json, post_json, etc.)

pub mod action_http_test;
pub mod analytics_http_test;
pub mod audit_http_test;
pub mod auth_http_test;
pub mod branding_http_test;
pub mod email_template_http_test;
pub mod identity_provider_http_test;
pub mod invitation_http_test;
pub mod keycloak_event_http_test;
pub mod mock_keycloak;
pub mod password_http_test;
pub mod role_http_test;
pub mod security_alert_http_test;
pub mod service_http_test;
pub mod session_http_test;
pub mod system_settings_http_test;
pub mod tenant_http_test;
pub mod user_http_test;
pub mod webauthn_http_test;
pub mod webhook_http_test;

use crate::api::{
    create_test_jwt_manager, TestActionRepository, TestAuditRepository, TestInvitationRepository,
    TestLinkedIdentityRepository, TestLoginEventRepository, TestPasswordResetRepository,
    TestRbacRepository, TestSecurityAlertRepository, TestServiceRepository, TestSessionRepository,
    TestSystemSettingsRepository, TestTenantRepository, TestUserRepository, TestWebhookRepository,
};
use auth9_core::cache::NoOpCacheManager;
use auth9_core::config::{
    Config, CorsConfig, DatabaseConfig, GrpcSecurityConfig, JwtConfig, KeycloakConfig,
    RateLimitConfig, RedisConfig, ServerConfig,
};
use auth9_core::jwt::JwtManager;
use auth9_core::keycloak::KeycloakClient;
use auth9_core::middleware::RateLimitState;
use auth9_core::server::build_full_router;
use auth9_core::service::{
    tenant::TenantRepositoryBundle, user::UserRepositoryBundle, ActionService, AnalyticsService,
    BrandingService, ClientService, EmailService, EmailTemplateService, IdentityProviderService,
    InvitationService, KeycloakSyncService, PasswordService, RbacService,
    SecurityDetectionService, SessionService, SystemSettingsService, TenantService, UserService,
    WebAuthnService, WebhookService,
};
use auth9_core::state::{
    HasAnalytics, HasBranding, HasCache, HasDbPool, HasEmailTemplates, HasIdentityProviders,
    HasInvitations, HasPasswordManagement, HasSecurityAlerts, HasServices, HasSessionManagement,
    HasSystemSettings, HasWebAuthn, HasWebhooks,
};
use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
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
        environment: "development".to_string(),
        http_host: "127.0.0.1".to_string(),
        http_port: 3000,
        grpc_host: "127.0.0.1".to_string(),
        grpc_port: 50051,
        database: DatabaseConfig {
            url: "mysql://test:test@localhost/test".to_string(),
            max_connections: 1,
            min_connections: 1,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
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
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        },
        grpc_security: GrpcSecurityConfig::default(),
        rate_limit: RateLimitConfig::default(),
        cors: CorsConfig::default(),
        telemetry: auth9_core::config::TelemetryConfig::default(),
        platform_admin_emails: vec!["admin@auth9.local".to_string()],
        webauthn: auth9_core::config::WebAuthnConfig {
            rp_id: "localhost".to_string(),
            rp_name: "Auth9 Test".to_string(),
            rp_origin: "http://localhost:3000".to_string(),
            challenge_ttl_secs: 300,
        },
        server: ServerConfig::default(),
        jwt_tenant_access_allowed_audiences: vec![],
        security_headers: auth9_core::config::SecurityHeadersConfig::default(),
        portal_client_id: None,
        password_reset: auth9_core::config::PasswordResetConfig {
            hmac_key: "test-password-reset-hmac-key".to_string(),
            token_ttl_secs: 3600,
        },
    }
}

// ============================================================================
// Test AppState (uses test repositories)
// ============================================================================

// Test Service Type Aliases
pub type TestTenantService = TenantService<
    TestTenantRepository,
    TestServiceRepository,
    TestWebhookRepository,
    TestInvitationRepository,
    TestUserRepository,
    TestRbacRepository,
    TestLoginEventRepository,
    TestSecurityAlertRepository,
    TestActionRepository,
>;

pub type TestUserService = UserService<
    TestUserRepository,
    TestSessionRepository,
    TestPasswordResetRepository,
    TestLinkedIdentityRepository,
    TestLoginEventRepository,
    TestSecurityAlertRepository,
    TestAuditRepository,
    TestRbacRepository,
>;

/// Test-friendly version of AppState using test repository implementations
#[derive(Clone)]
pub struct TestAppState {
    pub config: Arc<Config>,
    pub tenant_service: Arc<TestTenantService>,
    pub user_service: Arc<TestUserService>,
    pub client_service: Arc<ClientService<TestServiceRepository, TestRbacRepository>>,
    pub rbac_service: Arc<RbacService<TestRbacRepository>>,
    pub system_settings_service: Arc<SystemSettingsService<TestSystemSettingsRepository>>,
    pub email_service: Arc<EmailService<TestSystemSettingsRepository>>,
    pub email_template_service: Arc<EmailTemplateService<TestSystemSettingsRepository>>,
    pub branding_service: Arc<BrandingService<TestSystemSettingsRepository>>,
    pub password_service: Arc<
        PasswordService<
            TestPasswordResetRepository,
            TestUserRepository,
            TestSystemSettingsRepository,
            TestTenantRepository,
        >,
    >,
    pub session_service: Arc<SessionService<TestSessionRepository, TestUserRepository>>,
    pub identity_provider_service:
        Arc<IdentityProviderService<TestLinkedIdentityRepository, TestUserRepository>>,
    pub webauthn_service: Arc<WebAuthnService>,
    pub webhook_service: Arc<WebhookService<TestWebhookRepository>>,
    pub invitation_service: Arc<
        InvitationService<
            TestInvitationRepository,
            TestTenantRepository,
            TestSystemSettingsRepository,
        >,
    >,
    pub analytics_service: Arc<AnalyticsService<TestLoginEventRepository>>,
    pub security_detection_service: Arc<
        SecurityDetectionService<
            TestLoginEventRepository,
            TestSecurityAlertRepository,
            TestWebhookRepository,
        >,
    >,
    pub action_service: Arc<ActionService<TestActionRepository>>,
    pub audit_repo: Arc<TestAuditRepository>,
    pub jwt_manager: auth9_core::jwt::JwtManager,
    pub keycloak_client: KeycloakClient,
    #[allow(dead_code)]
    pub cache_manager: NoOpCacheManager,
    pub db_pool: sqlx::MySqlPool,
    // Keep references to raw repositories for test setup
    pub tenant_repo: Arc<TestTenantRepository>,
    pub user_repo: Arc<TestUserRepository>,
    pub service_repo: Arc<TestServiceRepository>,
    pub rbac_repo: Arc<TestRbacRepository>,
    pub system_settings_repo: Arc<TestSystemSettingsRepository>,
    #[allow(dead_code)]
    pub password_reset_repo: Arc<TestPasswordResetRepository>,
    pub session_repo: Arc<TestSessionRepository>,
    pub linked_identity_repo: Arc<TestLinkedIdentityRepository>,
    pub webhook_repo: Arc<TestWebhookRepository>,
    pub login_event_repo: Arc<TestLoginEventRepository>,
    pub security_alert_repo: Arc<TestSecurityAlertRepository>,
    #[allow(dead_code)]
    pub invitation_repo: Arc<TestInvitationRepository>,
    pub action_repo: Arc<TestActionRepository>,
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
        let system_settings_repo = Arc::new(TestSystemSettingsRepository::new());
        let password_reset_repo = Arc::new(TestPasswordResetRepository::new());
        let session_repo = Arc::new(TestSessionRepository::new());
        let linked_identity_repo = Arc::new(TestLinkedIdentityRepository::new());
        let webhook_repo = Arc::new(TestWebhookRepository::new());
        let login_event_repo = Arc::new(TestLoginEventRepository::new());
        let security_alert_repo = Arc::new(TestSecurityAlertRepository::new());
        let invitation_repo = Arc::new(TestInvitationRepository::new());
        let action_repo = Arc::new(TestActionRepository::new());

        // Create webhook service first (needed for webhook event publishing)
        let webhook_service = Arc::new(WebhookService::new(webhook_repo.clone()));

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
        let tenant_service = Arc::new(TenantService::new(tenant_repos, None));

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
            None,
            Some(webhook_service.clone()), // webhook event publisher
        ));
        let client_service = Arc::new(ClientService::new(
            service_repo.clone(),
            rbac_repo.clone(),
            None,
        ));
        let rbac_service = Arc::new(RbacService::new(rbac_repo.clone(), None));
        let system_settings_service = Arc::new(SystemSettingsService::new(
            system_settings_repo.clone(),
            None,
        ));
        let email_service = Arc::new(EmailService::new(system_settings_service.clone()));
        let email_template_service =
            Arc::new(EmailTemplateService::new(system_settings_repo.clone()));
        let branding_service = Arc::new(BrandingService::new(system_settings_repo.clone()));

        let jwt_manager = create_test_jwt_manager();
        let keycloak_client = KeycloakClient::new(config.keycloak.clone());
        let cache_manager = NoOpCacheManager::new();
        let db_pool = sqlx::MySqlPool::connect_lazy(&config.database.url).unwrap();

        // Create Keycloak sync service for tests
        let keycloak_updater: Arc<dyn auth9_core::service::keycloak_sync::KeycloakRealmUpdater> =
            Arc::new(KeycloakClient::new(config.keycloak.clone()));
        let keycloak_sync_service = Arc::new(KeycloakSyncService::new(keycloak_updater));

        // Create new services
        let password_service = Arc::new(PasswordService::with_tenant_repo(
            password_reset_repo.clone(),
            user_repo.clone(),
            email_service.clone(),
            Arc::new(KeycloakClient::new(config.keycloak.clone())),
            tenant_repo.clone(),
            keycloak_sync_service,
            config.password_reset.hmac_key.clone(),
        ));
        let session_service = Arc::new(SessionService::new(
            session_repo.clone(),
            user_repo.clone(),
            Arc::new(KeycloakClient::new(config.keycloak.clone())),
            Some(webhook_service.clone()), // webhook event publisher
        ));
        let identity_provider_service = Arc::new(IdentityProviderService::new(
            linked_identity_repo.clone(),
            user_repo.clone(),
            Arc::new(KeycloakClient::new(config.keycloak.clone())),
        ));
        let webauthn_service = {
            let rp_origin = url::Url::parse(&config.webauthn.rp_origin).unwrap();
            let webauthn_instance = Arc::new(
                webauthn_rs::WebauthnBuilder::new(&config.webauthn.rp_id, &rp_origin)
                    .unwrap()
                    .rp_name(&config.webauthn.rp_name)
                    .build()
                    .unwrap(),
            );
            let webauthn_repo = Arc::new(super::TestWebAuthnRepository::new());
            Arc::new(WebAuthnService::new(
                webauthn_instance,
                webauthn_repo,
                Arc::new(auth9_core::cache::NoOpCacheManager::new()),
                Some(Arc::new(KeycloakClient::new(config.keycloak.clone()))),
                config.webauthn.challenge_ttl_secs,
            ))
        };
        let invitation_service = Arc::new(InvitationService::new(
            invitation_repo.clone(),
            tenant_repo.clone(),
            email_service.clone(),
            "http://localhost:3000".to_string(),
        ));
        let analytics_service = Arc::new(AnalyticsService::new(login_event_repo.clone()));
        let security_detection_service = Arc::new(SecurityDetectionService::new(
            login_event_repo.clone(),
            security_alert_repo.clone(),
            webhook_service.clone(),
            Default::default(),
        ));
        // Use None for action_engine in tests to avoid slow V8 initialization
        let action_service = Arc::new(ActionService::new(action_repo.clone(), None));

        Self {
            config,
            tenant_service,
            user_service,
            client_service,
            rbac_service,
            system_settings_service,
            email_service,
            email_template_service,
            branding_service,
            password_service,
            session_service,
            identity_provider_service,
            webauthn_service,
            webhook_service,
            invitation_service,
            analytics_service,
            security_detection_service,
            action_service,
            audit_repo,
            jwt_manager,
            keycloak_client,
            cache_manager,
            db_pool,
            tenant_repo,
            user_repo,
            service_repo,
            rbac_repo,
            system_settings_repo,
            password_reset_repo,
            session_repo,
            linked_identity_repo,
            webhook_repo,
            login_event_repo,
            security_alert_repo,
            invitation_repo,
            action_repo,
        }
    }

    /// Create with an already started mock Keycloak server
    pub fn with_mock_keycloak(mock_server: &MockKeycloakServer) -> Self {
        Self::new(&mock_server.uri())
    }

    /// Enable public registration by setting allow_registration to true in branding config
    pub async fn enable_public_registration(&self) {
        use auth9_core::domain::BrandingConfig;
        let mut config = BrandingConfig::default();
        config.allow_registration = true;
        self.branding_service.update_branding(config).await.unwrap();
    }
}

/// Implement HasServices trait for TestAppState
/// This allows using production handlers with test repositories
impl HasServices for TestAppState {
    type TenantRepo = TestTenantRepository;
    type UserRepo = TestUserRepository;
    type ServiceRepo = TestServiceRepository;
    type RbacRepo = TestRbacRepository;
    type AuditRepo = TestAuditRepository;
    type SessionRepo = TestSessionRepository;
    type PasswordResetRepo = TestPasswordResetRepository;
    type LinkedIdentityRepo = TestLinkedIdentityRepository;
    type LoginEventRepo = TestLoginEventRepository;
    type SecurityAlertRepo = TestSecurityAlertRepository;
    type WebhookRepo = TestWebhookRepository;
    type CascadeInvitationRepo = TestInvitationRepository;
    type ActionRepo = TestActionRepository;

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
        // In tests, always return ready
        (true, true)
    }
}

/// Implement HasSystemSettings trait for TestAppState
impl HasSystemSettings for TestAppState {
    type SystemSettingsRepo = TestSystemSettingsRepository;

    fn system_settings_service(&self) -> &SystemSettingsService<Self::SystemSettingsRepo> {
        &self.system_settings_service
    }

    fn email_service(&self) -> &EmailService<Self::SystemSettingsRepo> {
        &self.email_service
    }
}

/// Implement HasEmailTemplates trait for TestAppState
impl HasEmailTemplates for TestAppState {
    fn email_template_service(&self) -> &EmailTemplateService<Self::SystemSettingsRepo> {
        &self.email_template_service
    }
}

/// Implement HasBranding trait for TestAppState
impl HasBranding for TestAppState {
    type BrandingRepo = TestSystemSettingsRepository;

    fn branding_service(&self) -> &BrandingService<Self::BrandingRepo> {
        &self.branding_service
    }
}

/// Implement HasPasswordManagement trait for TestAppState
impl HasPasswordManagement for TestAppState {
    type PasswordResetRepo = TestPasswordResetRepository;
    type PasswordUserRepo = TestUserRepository;
    type PasswordSystemSettingsRepo = TestSystemSettingsRepository;
    type PasswordTenantRepo = TestTenantRepository;

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

/// Implement HasSessionManagement trait for TestAppState
impl HasSessionManagement for TestAppState {
    type SessionRepo = TestSessionRepository;
    type SessionUserRepo = TestUserRepository;

    fn session_service(&self) -> &SessionService<Self::SessionRepo, Self::SessionUserRepo> {
        &self.session_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasIdentityProviders trait for TestAppState
impl HasIdentityProviders for TestAppState {
    type LinkedIdentityRepo = TestLinkedIdentityRepository;
    type IdpUserRepo = TestUserRepository;

    fn identity_provider_service(
        &self,
    ) -> &IdentityProviderService<Self::LinkedIdentityRepo, Self::IdpUserRepo> {
        &self.identity_provider_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasWebAuthn trait for TestAppState
impl HasWebAuthn for TestAppState {
    fn webauthn_service(&self) -> &WebAuthnService {
        &self.webauthn_service
    }

    fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

/// Implement HasWebhooks trait for TestAppState
impl HasWebhooks for TestAppState {
    type WebhookRepo = TestWebhookRepository;

    fn webhook_service(&self) -> &WebhookService<Self::WebhookRepo> {
        &self.webhook_service
    }
}

/// Implement HasAnalytics trait for TestAppState
impl HasAnalytics for TestAppState {
    type LoginEventRepo = TestLoginEventRepository;

    fn analytics_service(&self) -> &AnalyticsService<Self::LoginEventRepo> {
        &self.analytics_service
    }
}

/// Implement HasSecurityAlerts trait for TestAppState
impl HasSecurityAlerts for TestAppState {
    type SecurityLoginEventRepo = TestLoginEventRepository;
    type SecurityAlertRepo = TestSecurityAlertRepository;
    type SecurityWebhookRepo = TestWebhookRepository;

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

/// Implement HasInvitations trait for TestAppState
impl HasInvitations for TestAppState {
    type InvitationRepo = TestInvitationRepository;

    fn invitation_service(
        &self,
    ) -> &InvitationService<Self::InvitationRepo, Self::TenantRepo, Self::SystemSettingsRepo> {
        &self.invitation_service
    }
}

/// Implement HasDbPool trait for TestAppState
impl HasDbPool for TestAppState {
    fn db_pool(&self) -> &sqlx::MySqlPool {
        &self.db_pool
    }
}

/// Implement HasCache trait for TestAppState
impl HasCache for TestAppState {
    type Cache = NoOpCacheManager;

    fn cache(&self) -> &Self::Cache {
        &self.cache_manager
    }
}

// ============================================================================
// Test Router Builder
// ============================================================================

/// Build a router for HTTP handler tests using the PRODUCTION router.
///
/// This uses the actual `build_full_router` from `auth9_core::server` with TestAppState,
/// which means these tests cover the real production handler code in `src/api/*.rs`.
pub fn build_test_router(state: TestAppState) -> Router {
    // Use the production router with TestAppState and disabled rate limiting
    // This ensures we're testing the actual production handlers
    build_full_router(state, RateLimitState::noop(), std::sync::Arc::new(None))
}

/// Build a router with email template endpoints for testing.
///
/// This creates a minimal router that includes the email template handlers,
/// allowing us to test them without implementing all traits required by build_full_router.
pub fn build_email_template_test_router(state: TestAppState) -> Router {
    use auth9_core::api::email_template;
    use axum::routing::{get, post};

    Router::new()
        .route(
            "/api/v1/system/email-templates",
            get(email_template::list_templates::<TestAppState>),
        )
        .route(
            "/api/v1/system/email-templates/{type}",
            get(email_template::get_template::<TestAppState>)
                .put(email_template::update_template::<TestAppState>)
                .delete(email_template::reset_template::<TestAppState>),
        )
        .route(
            "/api/v1/system/email-templates/{type}/preview",
            post(email_template::preview_template::<TestAppState>),
        )
        .with_state(state)
}

/// Build a router with system settings endpoints for testing.
///
/// This creates a minimal router that includes the system settings handlers.
pub fn build_system_settings_test_router(state: TestAppState) -> Router {
    use auth9_core::api::system_settings;
    use axum::routing::{get, post};

    Router::new()
        .route(
            "/api/v1/system/email",
            get(system_settings::get_email_settings::<TestAppState>)
                .put(system_settings::update_email_settings::<TestAppState>),
        )
        .route(
            "/api/v1/system/email/test",
            post(system_settings::test_email_connection::<TestAppState>),
        )
        .route(
            "/api/v1/system/email/send-test",
            post(system_settings::send_test_email::<TestAppState>),
        )
        .with_state(state)
}

/// Build a router with branding endpoints for testing.
///
/// This creates a minimal router that includes the branding handlers.
pub fn build_branding_test_router(state: TestAppState) -> Router {
    use auth9_core::api::branding;
    use axum::routing::get;

    Router::new()
        .route(
            "/api/v1/public/branding",
            get(branding::get_public_branding::<TestAppState>),
        )
        .route(
            "/api/v1/system/branding",
            get(branding::get_branding::<TestAppState>)
                .put(branding::update_branding::<TestAppState>),
        )
        .with_state(state)
}

// ============================================================================
// HTTP Test Helpers
// ============================================================================

/// Make a raw GET request and return the response
pub async fn get_raw(app: &Router, path: &str) -> (StatusCode, axum::body::Bytes) {
    let request = Request::builder()
        .method(Method::GET)
        .uri(path)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_default();

    (status, body_bytes)
}

/// Make a GET request with Authorization header and parse JSON response
pub async fn get_json_with_auth<T: DeserializeOwned>(
    app: &Router,
    path: &str,
    token: &str,
) -> (StatusCode, Option<T>) {
    let request = Request::builder()
        .method(Method::GET)
        .uri(path)
        .header("Authorization", format!("Bearer {}", token))
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

/// Make a GET request and parse JSON response
pub async fn get_json<T: DeserializeOwned>(app: &Router, path: &str) -> (StatusCode, Option<T>) {
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

/// Make a POST request with Authorization header, JSON body and parse JSON response
pub async fn post_json_with_auth<T: Serialize, R: DeserializeOwned>(
    app: &Router,
    path: &str,
    body: &T,
    token: &str,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
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

/// Make a PUT request with Authorization header, JSON body and parse JSON response
pub async fn put_json_with_auth<T: Serialize, R: DeserializeOwned>(
    app: &Router,
    path: &str,
    body: &T,
    token: &str,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::PUT)
        .uri(path)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
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

/// Make a PATCH request with JSON body and auth header
pub async fn patch_json_with_auth<T: Serialize, R: DeserializeOwned>(
    app: &Router,
    path: &str,
    body: &T,
    token: &str,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::PATCH)
        .uri(path)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
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
pub async fn delete_json<R: DeserializeOwned>(app: &Router, path: &str) -> (StatusCode, Option<R>) {
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

/// Make a DELETE request with Authorization header and parse JSON response
pub async fn delete_json_with_auth<R: DeserializeOwned>(
    app: &Router,
    path: &str,
    token: &str,
) -> (StatusCode, Option<R>) {
    let request = Request::builder()
        .method(Method::DELETE)
        .uri(path)
        .header("Authorization", format!("Bearer {}", token))
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
mod rbac_cross_service_test;
