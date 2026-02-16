use crate::state::{HasBranding, HasEmailTemplates, HasServices, HasSystemSettings};

pub trait PlatformContext:
    HasServices + HasSystemSettings + HasEmailTemplates + HasBranding
{
}

impl<T> PlatformContext for T where
    T: HasServices + HasSystemSettings + HasEmailTemplates + HasBranding
{
}
