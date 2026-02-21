//! Provisioning domain context trait

use crate::state::{HasScimServices, HasServices, HasWebhooks};

/// Context trait for SCIM provisioning domain.
/// Combines core services with SCIM-specific services.
pub trait ProvisioningContext: HasServices + HasScimServices + HasWebhooks {}

impl<T> ProvisioningContext for T where T: HasServices + HasScimServices + HasWebhooks {}
