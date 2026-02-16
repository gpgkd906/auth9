use crate::state::{HasBranding, HasInvitations, HasServices};

pub trait TenantAccessContext: HasServices + HasInvitations + HasBranding {}

impl<T> TenantAccessContext for T where T: HasServices + HasInvitations + HasBranding {}
