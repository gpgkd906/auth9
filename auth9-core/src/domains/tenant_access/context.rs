use crate::state::{HasBranding, HasDbPool, HasInvitations, HasServices};

pub trait TenantAccessContext: HasServices + HasInvitations + HasBranding + HasDbPool {}

impl<T> TenantAccessContext for T where T: HasServices + HasInvitations + HasBranding + HasDbPool {}
