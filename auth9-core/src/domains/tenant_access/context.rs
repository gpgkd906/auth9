use crate::state::{
    HasBranding, HasDbPool, HasInvitations, HasLdapAuth, HasRequiredActions, HasServices,
};

pub trait TenantAccessContext:
    HasServices + HasInvitations + HasBranding + HasDbPool + HasRequiredActions + HasLdapAuth
{
}

impl<T> TenantAccessContext for T where
    T: HasServices + HasInvitations + HasBranding + HasDbPool + HasRequiredActions + HasLdapAuth
{
}
