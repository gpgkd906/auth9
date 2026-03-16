use crate::state::{
    HasAnalytics, HasBranding, HasCache, HasDbPool, HasIdentityProviders, HasPasswordManagement,
    HasServices, HasSessionManagement, HasSystemSettings, HasWebAuthn,
};

pub trait IdentityContext:
    HasServices
    + HasCache
    + HasPasswordManagement
    + HasSessionManagement
    + HasWebAuthn
    + HasIdentityProviders
    + HasAnalytics
    + HasDbPool
    + HasSystemSettings
    + HasBranding
{
}

impl<T> IdentityContext for T where
    T: HasServices
        + HasCache
        + HasPasswordManagement
        + HasSessionManagement
        + HasWebAuthn
        + HasIdentityProviders
        + HasAnalytics
        + HasDbPool
        + HasSystemSettings
        + HasBranding
{
}
