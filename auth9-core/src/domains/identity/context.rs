use crate::state::{
    HasAnalytics, HasBranding, HasCache, HasDbPool, HasEmailVerification, HasIdentityProviders,
    HasMfa, HasPasswordManagement, HasRequiredActions, HasServices, HasSessionManagement,
    HasSystemSettings, HasWebAuthn,
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
    + HasEmailVerification
    + HasRequiredActions
    + HasMfa
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
        + HasEmailVerification
        + HasRequiredActions
        + HasMfa
{
}
