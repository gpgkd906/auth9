//! Application state traits for dependency injection
//!
//! This module defines traits that abstract the application state,
//! enabling the same handler code to work with both production
//! and test implementations.

use crate::config::Config;
use crate::jwt::JwtManager;
use crate::keycloak::KeycloakClient;
use crate::repository::audit::AuditRepository;
use crate::repository::{RbacRepository, ServiceRepository, TenantRepository, UserRepository};
use crate::service::{ClientService, RbacService, TenantService, UserService};

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

    /// Get the application configuration
    fn config(&self) -> &Config;

    /// Get the tenant service
    fn tenant_service(&self) -> &TenantService<Self::TenantRepo>;

    /// Get the user service
    fn user_service(&self) -> &UserService<Self::UserRepo>;

    /// Get the client/service service
    fn client_service(&self) -> &ClientService<Self::ServiceRepo>;

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
