//! Enterprise SSO domain models (tenant-scoped connectors).

use crate::domain::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSsoConnector {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub keycloak_alias: String,
    pub config: HashMap<String, String>,
    pub domains: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "snake_case")]
pub struct CreateEnterpriseSsoConnectorInput {
    #[validate(length(min = 1, max = 100))]
    pub alias: String,
    pub display_name: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub provider_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default)]
    pub config: HashMap<String, String>,
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "snake_case")]
pub struct UpdateEnterpriseSsoConnectorInput {
    pub display_name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub config: Option<HashMap<String, String>>,
    pub domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSsoDiscoveryInput {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EnterpriseSsoDiscoveryResult {
    pub tenant_id: StringUuid,
    pub tenant_slug: String,
    pub connector_alias: String,
    pub keycloak_alias: String,
    pub provider_type: String,
}

fn default_true() -> bool {
    true
}

fn default_priority() -> i32 {
    100
}
