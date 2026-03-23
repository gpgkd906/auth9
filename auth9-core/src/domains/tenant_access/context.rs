use crate::state::{HasBranding, HasDbPool, HasInvitations, HasRequiredActions, HasServices};

pub trait TenantAccessContext: HasServices + HasInvitations + HasBranding + HasDbPool + HasRequiredActions {}

impl<T> TenantAccessContext for T where T: HasServices + HasInvitations + HasBranding + HasDbPool + HasRequiredActions {}
