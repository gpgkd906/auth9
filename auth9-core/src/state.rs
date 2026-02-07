//! Application state traits for dependency injection
//!
//! This module defines traits that abstract the application state,
//! enabling the same handler code to work with both production
//! and test implementations.

use crate::cache::CacheOperations;
use crate::config::Config;
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::audit::AuditRepository;
use crate::repository::{
    InvitationRepository, LinkedIdentityRepository, LoginEventRepository, PasswordResetRepository,
    RbacRepository, SecurityAlertRepository, ServiceRepository, SessionRepository,
    SystemSettingsRepository, TenantRepository, UserRepository, WebhookRepository,
};
use crate::service::{
    AnalyticsService, BrandingService, ClientService, EmailService, EmailTemplateService,
    IdentityProviderService, InvitationService, PasswordService, RbacService,
    SecurityDetectionService, SessionService, SystemSettingsService, TenantService, UserService,
    WebAuthnService, WebhookService,
};

/// Trait for application state that provides access to all services.
///
/// This trait enables dependency injection by allowing handlers to work
/// with any type that provides the required services, whether that's
/// the production `AppState` or a test implementation.
pub trait HasServices: Clone + Send + Sync + 'static {
    /// The tenant repository type
    type TenantRepo: TenantRepository;
    /// The user repository type
    type UserRepo: UserRepository;
    /// The service repository type
    type ServiceRepo: ServiceRepository;
    /// The RBAC repository type
    type RbacRepo: RbacRepository;
    /// The audit repository type
    type AuditRepo: AuditRepository;
    /// The session repository type (for cascade delete)
    type SessionRepo: SessionRepository;
    /// The password reset repository type (for cascade delete)
    type PasswordResetRepo: PasswordResetRepository;
    /// The linked identity repository type (for cascade delete)
    type LinkedIdentityRepo: LinkedIdentityRepository;
    /// The login event repository type (for cascade delete)
    type LoginEventRepo: LoginEventRepository;
    /// The security alert repository type (for cascade delete)
    type SecurityAlertRepo: SecurityAlertRepository;
    /// The webhook repository type (for cascade delete)
    type WebhookRepo: WebhookRepository;
    /// The invitation repository type (for cascade delete) - Note: HasInvitations also has this
    type CascadeInvitationRepo: InvitationRepository;

    /// Get the application configuration
    fn config(&self) -> &Config;

    /// Get the tenant service
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
    >;

    /// Get the user service
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
    >;

    /// Get the client/service service
    fn client_service(&self) -> &ClientService<Self::ServiceRepo, Self::RbacRepo>;

    /// Get the RBAC service
    fn rbac_service(&self) -> &RbacService<Self::RbacRepo>;

    /// Get the audit repository
    fn audit_repo(&self) -> &Self::AuditRepo;

    /// Get the JWT manager
    fn jwt_manager(&self) -> &JwtManager;

    /// Get the Keycloak client
    fn keycloak_client(&self) -> &KeycloakClient;

    /// Check if the system is ready (database and cache are healthy)
    /// Returns (db_ok, cache_ok) tuple
    fn check_ready(&self) -> impl std::future::Future<Output = (bool, bool)> + Send;
}

/// Extension trait for writing audit logs
pub trait HasAuditLog: HasServices {
    /// Write an audit log entry
    fn write_audit_log(
        &self,
        headers: &axum::http::HeaderMap,
        action: &str,
        resource_type: &str,
        resource_id: Option<uuid::Uuid>,
        before: Option<serde_json::Value>,
        after: Option<serde_json::Value>,
    ) -> impl std::future::Future<Output = ()> + Send;
}

/// Trait for states that provide system settings and email services
pub trait HasSystemSettings: Clone + Send + Sync + 'static {
    /// The system settings repository type
    type SystemSettingsRepo: SystemSettingsRepository;

    /// Get the system settings service
    fn system_settings_service(&self) -> &SystemSettingsService<Self::SystemSettingsRepo>;

    /// Get the email service
    fn email_service(&self) -> &EmailService<Self::SystemSettingsRepo>;
}

/// Trait for states that provide invitation services
pub trait HasInvitations: HasServices + HasSystemSettings {
    /// The invitation repository type
    type InvitationRepo: InvitationRepository;

    /// Get the invitation service
    fn invitation_service(
        &self,
    ) -> &InvitationService<Self::InvitationRepo, Self::TenantRepo, Self::SystemSettingsRepo>;
}

/// Trait for states that provide email template services
pub trait HasEmailTemplates: HasSystemSettings {
    /// Get the email template service
    fn email_template_service(&self) -> &EmailTemplateService<Self::SystemSettingsRepo>;
}

/// Trait for states that provide branding services
pub trait HasBranding: Clone + Send + Sync + 'static {
    /// The system settings repository type used for branding storage
    type BrandingRepo: SystemSettingsRepository;

    /// Get the branding service
    fn branding_service(&self) -> &BrandingService<Self::BrandingRepo>;
}

/// Trait for states that provide password management services
pub trait HasPasswordManagement: Clone + Send + Sync + 'static {
    /// The password reset repository type
    type PasswordResetRepo: PasswordResetRepository;
    /// The user repository type
    type PasswordUserRepo: UserRepository;
    /// The system settings repository type
    type PasswordSystemSettingsRepo: SystemSettingsRepository;
    /// The tenant repository type for password policy
    type PasswordTenantRepo: TenantRepository;

    /// Get the password service
    fn password_service(
        &self,
    ) -> &PasswordService<
        Self::PasswordResetRepo,
        Self::PasswordUserRepo,
        Self::PasswordSystemSettingsRepo,
        Self::PasswordTenantRepo,
    >;

    /// Get the JWT manager for token verification
    fn jwt_manager(&self) -> &JwtManager;
}

/// Trait for states that provide session management services
pub trait HasSessionManagement: Clone + Send + Sync + 'static {
    /// The session repository type
    type SessionRepo: SessionRepository;
    /// The user repository type
    type SessionUserRepo: UserRepository;

    /// Get the session service
    fn session_service(&self) -> &SessionService<Self::SessionRepo, Self::SessionUserRepo>;

    /// Get the JWT manager for token verification
    fn jwt_manager(&self) -> &JwtManager;
}

/// Trait for states that provide WebAuthn services
pub trait HasWebAuthn: Clone + Send + Sync + 'static {
    /// Get the WebAuthn service
    fn webauthn_service(&self) -> &WebAuthnService;

    /// Get the JWT manager for token verification
    fn jwt_manager(&self) -> &JwtManager;
}

/// Trait for states that provide identity provider services
pub trait HasIdentityProviders: Clone + Send + Sync + 'static {
    /// The linked identity repository type
    type LinkedIdentityRepo: LinkedIdentityRepository;
    /// The user repository type
    type IdpUserRepo: UserRepository;

    /// Get the identity provider service
    fn identity_provider_service(
        &self,
    ) -> &IdentityProviderService<Self::LinkedIdentityRepo, Self::IdpUserRepo>;

    /// Get the JWT manager for token verification
    fn jwt_manager(&self) -> &JwtManager;
}

/// Trait for states that provide analytics services
pub trait HasAnalytics: Clone + Send + Sync + 'static {
    /// The login event repository type
    type LoginEventRepo: LoginEventRepository;

    /// Get the analytics service
    fn analytics_service(&self) -> &AnalyticsService<Self::LoginEventRepo>;
}

/// Trait for states that provide webhook services
pub trait HasWebhooks: Clone + Send + Sync + 'static {
    /// The webhook repository type
    type WebhookRepo: WebhookRepository;

    /// Get the webhook service
    fn webhook_service(&self) -> &WebhookService<Self::WebhookRepo>;
}

/// Trait for states that provide security alert services
pub trait HasSecurityAlerts: Clone + Send + Sync + 'static {
    /// The login event repository type
    type SecurityLoginEventRepo: LoginEventRepository;
    /// The security alert repository type
    type SecurityAlertRepo: SecurityAlertRepository;
    /// The webhook repository type
    type SecurityWebhookRepo: WebhookRepository;

    /// Get the security detection service
    fn security_detection_service(
        &self,
    ) -> &SecurityDetectionService<
        Self::SecurityLoginEventRepo,
        Self::SecurityAlertRepo,
        Self::SecurityWebhookRepo,
    >;

    /// Get the JWT manager for token verification
    fn jwt_manager(&self) -> &JwtManager;
}

/// Trait for states that provide direct database access
/// Used for tenant-service toggles and other direct queries
pub trait HasDbPool: Clone + Send + Sync + 'static {
    /// Get the database pool
    fn db_pool(&self) -> &sqlx::MySqlPool;
}

/// Trait for states that provide cache access
/// Used for token blacklisting and other caching operations
pub trait HasCache: Clone + Send + Sync + 'static {
    /// The cache type (CacheManager or NoOpCacheManager)
    type Cache: CacheOperations + Clone + 'static;

    /// Get the cache manager
    fn cache(&self) -> &Self::Cache;
}
