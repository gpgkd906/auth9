use crate::state::{
    HasAnalytics, HasCache, HasDbPool, HasIdentityProviders, HasPasswordManagement, HasServices,
    HasSessionManagement, HasWebAuthn,
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
{
}
