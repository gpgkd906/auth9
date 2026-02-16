//! Security and observability domain service facade.

pub use crate::domains::security_observability::service::analytics::AnalyticsService;
pub use crate::domains::security_observability::service::security_detection::{
    SecurityDetectionConfig, SecurityDetectionService,
};
