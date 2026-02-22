//! ABAC policy domain models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AbacMode {
    #[default]
    Disabled,
    Shadow,
    Enforce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AbacEffect {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AbacPolicyDocument {
    #[validate(length(min = 1, message = "Policy must contain at least one rule"))]
    pub rules: Vec<AbacRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AbacRule {
    pub id: String,
    pub effect: AbacEffect,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub resource_types: Vec<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub condition: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AbacPolicySetSummary {
    pub policy_set_id: String,
    pub tenant_id: String,
    pub mode: AbacMode,
    pub published_version_id: Option<String>,
    pub published_version_no: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AbacPolicyVersionSummary {
    pub id: String,
    pub policy_set_id: String,
    pub version_no: i32,
    pub status: String,
    pub change_note: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AbacSimulationInput {
    pub action: String,
    pub resource_type: String,
    #[serde(default)]
    pub subject: serde_json::Value,
    #[serde(default)]
    pub resource: serde_json::Value,
    #[serde(default)]
    pub request: serde_json::Value,
    #[serde(default)]
    pub env: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AbacSimulationResult {
    pub decision: String,
    pub matched_allow_rule_ids: Vec<String>,
    pub matched_deny_rule_ids: Vec<String>,
}
