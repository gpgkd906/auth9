//! Application state traits for dependency injection
//!
//! This module defines traits that abstract the application state,
//! enabling the same handler code to work with both production
//! and test implementations.

use crate::cache::CacheOperations;
use crate::config::Config;
use crate::domains::authorization::service::{ClientService, RbacService};
use crate::domains::identity::service::{
    IdentityProviderService, PasswordService, SessionService, WebAuthnService,
};
use crate::domains::integration::service::{ActionService, WebhookService};
use crate::domains::platform::service::{
    BrandingService, EmailService, EmailTemplateService, SystemSettingsService,
};
use crate::domains::provisioning::service::{ScimService, ScimTokenService};
use crate::domains::security_observability::service::{AnalyticsService, SecurityDetectionService};
use crate::domains::tenant_access::service::{InvitationService, TenantService, UserService};
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::audit::AuditRepository;
use crate::repository::scim_group_mapping::ScimGroupRoleMappingRepository;
use crate::repository::scim_log::ScimProvisioningLogRepository;
use crate::repository::scim_token::ScimTokenRepository;
use crate::repository::{
    ActionRepository, InvitationRepository, LinkedIdentityRepository, LoginEventRepository,
    PasswordResetRepository, RbacRepository, SecurityAlertRepository, ServiceBrandingRepository,
    ServiceRepository, SessionRepository, SystemSettingsRepository, TenantRepository,
    UserRepository, WebhookRepository,
};

// ============================================================
// Generic Service Type Aliases for Trait Bounds
// ============================================================

/// Generic TenantService type parameterized by trait associated types
pub type TenantServiceType<S> = TenantService<
    <S as HasServices>::TenantRepo,
    <S as HasServices>::ServiceRepo,
    <S as HasServices>::WebhookRepo,
    <S as HasServices>::CascadeInvitationRepo,
    <S as HasServices>::UserRepo,
    <S as HasServices>::RbacRepo,
    <S as HasServices>::LoginEventRepo,
    <S as HasServices>::SecurityAlertRepo,
    <S as HasServices>::ActionRepo,
>;

/// Generic UserService type parameterized by trait associated types
pub type UserServiceType<S> = UserService<
    <S as HasServices>::UserRepo,
    <S as HasServices>::SessionRepo,
    <S as HasServices>::PasswordResetRepo,
    <S as HasServices>::LinkedIdentityRepo,
    <S as HasServices>::LoginEventRepo,
    <S as HasServices>::SecurityAlertRepo,
    <S as HasServices>::AuditRepo,
    <S as HasServices>::RbacRepo,
>;

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
    /// The action repository type
    type ActionRepo: ActionRepository;

    /// Get the application configuration
    fn config(&self) -> &Config;

    /// Get the tenant service
    fn tenant_service(&self) -> &TenantServiceType<Self>;

    /// Get the user service
    fn user_service(&self) -> &UserServiceType<Self>;

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

    /// Get the action service
    fn action_service(&self) -> &ActionService<Self::ActionRepo>;

    /// Check if the system is ready (database and cache are healthy)
    /// Returns (db_ok, cache_ok) tuple
    fn check_ready(&self) -> impl std::future::Future<Output = (bool, bool)> + Send;

    /// Optional direct database pool access for cross-cutting policy logic.
    fn maybe_db_pool(&self) -> Option<&sqlx::MySqlPool> {
        None
    }
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
    /// The service branding repository type
    type ServiceBrandingRepo: ServiceBrandingRepository;

    /// Get the branding service
    fn branding_service(&self) -> &BrandingService<Self::BrandingRepo, Self::ServiceBrandingRepo>;
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

// ============================================================
// SCIM Service Type Aliases
// ============================================================

/// Generic ScimService type parameterized by trait associated types
pub type ScimServiceType<S> = ScimService<
    <S as HasServices>::UserRepo,
    <S as HasScimServices>::ScimGroupMappingRepo,
    <S as HasScimServices>::ScimLogRepo,
>;

/// Generic ScimTokenService type parameterized by trait associated types
pub type ScimTokenServiceType<S> = ScimTokenService<<S as HasScimServices>::ScimTokenRepo>;

/// Trait for states that provide SCIM provisioning services
pub trait HasScimServices: Clone + Send + Sync + 'static {
    /// The SCIM token repository type
    type ScimTokenRepo: ScimTokenRepository;
    /// The SCIM group-role mapping repository type
    type ScimGroupMappingRepo: ScimGroupRoleMappingRepository;
    /// The SCIM provisioning log repository type
    type ScimLogRepo: ScimProvisioningLogRepository;

    /// Get the SCIM service
    fn scim_service(&self) -> &ScimServiceType<Self>
    where
        Self: HasServices;

    /// Get the SCIM token service
    fn scim_token_service(&self) -> &ScimTokenServiceType<Self>;

    /// Get the SCIM group mapping repository (for admin API direct access)
    fn scim_group_mapping_repo(&self) -> &Self::ScimGroupMappingRepo;

    /// Get the SCIM log repository (for admin API direct access)
    fn scim_log_repo(&self) -> &Self::ScimLogRepo;
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
