use crate::state::{HasAnalytics, HasSecurityAlerts, HasServices};

pub trait SecurityObservabilityContext: HasServices + HasAnalytics + HasSecurityAlerts {}

impl<T> SecurityObservabilityContext for T where T: HasServices + HasAnalytics + HasSecurityAlerts {}
