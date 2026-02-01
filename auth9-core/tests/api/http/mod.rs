//! HTTP API Handler Tests Infrastructure
//!
//! This module provides test utilities for HTTP handler testing without
//! external dependencies (no database, no Redis, no Keycloak).
//!
//! Key components:
//! - `TestAppState` - Test-friendly version of AppState implementing `HasServices`
//! - Uses production `build_router()` with `TestAppState` for actual handler coverage
//! - Helper functions for making HTTP requests (get_json, post_json, etc.)

pub mod auth_http_test;
pub mod branding_http_test;
pub mod email_template_http_test;
pub mod mock_keycloak;
pub mod role_http_test;
pub mod service_http_test;
pub mod system_settings_http_test;
pub mod tenant_http_test;
pub mod user_http_test;

use crate::api::{
    create_test_jwt_manager, TestAuditRepository, TestRbacRepository, TestServiceRepository,
    TestSystemSettingsRepository, TestTenantRepository, TestUserRepository,
};
use auth9_core::cache::NoOpCacheManager;
use auth9_core::config::{
    Config, DatabaseConfig, GrpcSecurityConfig, JwtConfig, KeycloakConfig, RateLimitConfig,
    RedisConfig,
};
use auth9_core::jwt::JwtManager;
use auth9_core::keycloak::KeycloakClient;
use auth9_core::server::build_router;
use auth9_core::service::{
    BrandingService, ClientService, EmailService, EmailTemplateService, RbacService,
    SystemSettingsService, TenantService, UserService,
};
use auth9_core::state::{HasBranding, HasEmailTemplates, HasServices, HasSystemSettings};
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
            core_public_url: None,
            portal_url: None,
        },
        grpc_security: GrpcSecurityConfig::default(),
        rate_limit: RateLimitConfig::default(),
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
    pub system_settings_service: Arc<SystemSettingsService<TestSystemSettingsRepository>>,
    pub email_service: Arc<EmailService<TestSystemSettingsRepository>>,
    pub email_template_service: Arc<EmailTemplateService<TestSystemSettingsRepository>>,
    pub branding_service: Arc<BrandingService<TestSystemSettingsRepository>>,
    pub audit_repo: Arc<TestAuditRepository>,
    pub jwt_manager: auth9_core::jwt::JwtManager,
    pub keycloak_client: KeycloakClient,
    #[allow(dead_code)]
    pub cache_manager: NoOpCacheManager,
    // Keep references to raw repositories for test setup
    pub tenant_repo: Arc<TestTenantRepository>,
    pub user_repo: Arc<TestUserRepository>,
    pub service_repo: Arc<TestServiceRepository>,
    pub rbac_repo: Arc<TestRbacRepository>,
    pub system_settings_repo: Arc<TestSystemSettingsRepository>,
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

        let tenant_service = Arc::new(TenantService::new(tenant_repo.clone(), None));
        let user_service = Arc::new(UserService::new(user_repo.clone()));
        let client_service = Arc::new(ClientService::new(service_repo.clone(), None));
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
            audit_repo,
            jwt_manager,
            keycloak_client,
            cache_manager,
            tenant_repo,
            user_repo,
            service_repo,
            rbac_repo,
            system_settings_repo,
        }
    }

    /// Create with an already started mock Keycloak server
    pub fn with_mock_keycloak(mock_server: &MockKeycloakServer) -> Self {
        Self::new(&mock_server.uri())
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

// ============================================================================
// Test Router Builder
// ============================================================================

/// Build a router for HTTP handler tests using the PRODUCTION router.
///
/// This uses the actual `build_router` from `auth9_core::server` with TestAppState,
/// which means these tests cover the real production handler code in `src/api/*.rs`.
pub fn build_test_router(state: TestAppState) -> Router {
    // Use the production router with TestAppState
    // This ensures we're testing the actual production handlers
    build_router(state)
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
