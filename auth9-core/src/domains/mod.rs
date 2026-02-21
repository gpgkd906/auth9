//! Domain-oriented modules grouping API, service, and route layers by bounded context.

pub mod authorization;
pub mod identity;
pub mod integration;
pub mod platform;
pub mod provisioning;
pub mod security_observability;
pub mod tenant_access;

/// Aggregate trait for building the full HTTP router from domain route modules.
///
/// This narrows server-level generics to a single domain-centric bound while
/// keeping compatibility with existing `Has*` traits underneath.
pub trait DomainRouterState:
    identity::context::IdentityContext
    + tenant_access::context::TenantAccessContext
    + authorization::context::AuthorizationContext
    + platform::context::PlatformContext
    + integration::context::IntegrationContext
    + security_observability::context::SecurityObservabilityContext
    + provisioning::context::ProvisioningContext
{
}

impl<T> DomainRouterState for T where
    T: identity::context::IdentityContext
        + tenant_access::context::TenantAccessContext
        + authorization::context::AuthorizationContext
        + platform::context::PlatformContext
        + integration::context::IntegrationContext
        + security_observability::context::SecurityObservabilityContext
        + provisioning::context::ProvisioningContext
{
}
