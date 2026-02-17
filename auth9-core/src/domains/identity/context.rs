use crate::state::{
    HasAnalytics, HasCache, HasIdentityProviders, HasPasswordManagement, HasServices,
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
{
}
