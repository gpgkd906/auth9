use crate::state::{HasAnalytics, HasCache, HasSecurityAlerts, HasServices, HasWebhooks};

pub trait IntegrationContext:
    HasServices + HasWebhooks + HasAnalytics + HasSecurityAlerts + HasCache
{
}

impl<T> IntegrationContext for T where
    T: HasServices + HasWebhooks + HasAnalytics + HasSecurityAlerts + HasCache
{
}
