use crate::domain::{AbacEffect, AbacPolicyDocument};
use crate::error::AppError;
use crate::middleware::auth::AuthUser;
use crate::policy::{PolicyAction, PolicyInput, ResourceScope};
use crate::state::HasServices;
use chrono::{Datelike, Timelike, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::FromRow;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbacDecisionMode {
    Disabled,
    Shadow,
    Enforce,
}

#[derive(Debug, Clone)]
pub struct AbacEvaluationOutcome {
    pub mode: AbacDecisionMode,
    pub denied: bool,
    pub matched_allow_rule_ids: Vec<String>,
    pub matched_deny_rule_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AbacSimulationOutcome {
    pub denied: bool,
    pub matched_allow_rule_ids: Vec<String>,
    pub matched_deny_rule_ids: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
struct ActivePolicyRow {
    mode: String,
    policy_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ConditionNode {
    All {
        all: Vec<ConditionNode>,
    },
    Any {
        any: Vec<ConditionNode>,
    },
    Not {
        not: Box<ConditionNode>,
    },
    Predicate {
        var: String,
        op: String,
        #[serde(default)]
        value: Value,
    },
}

fn action_to_key(action: PolicyAction) -> &'static str {
    match action {
        PolicyAction::UserTenantRead => "user_tenant_read",
        PolicyAction::UserManage => "user_manage",
        PolicyAction::InvitationRead => "invitation_read",
        PolicyAction::InvitationWrite => "invitation_write",
        PolicyAction::RbacWrite => "rbac_write",
        PolicyAction::RbacAssignSelf => "rbac_assign_self",
        _ => "unknown",
    }
}

fn scope_to_resource_type(scope: &ResourceScope) -> &'static str {
    match scope {
        ResourceScope::Global => "global",
        ResourceScope::Tenant(_) => "tenant",
        ResourceScope::User(_) => "user",
    }
}

fn mode_from_str(mode: &str) -> AbacDecisionMode {
    match mode {
        "shadow" => AbacDecisionMode::Shadow,
        "enforce" => AbacDecisionMode::Enforce,
        _ => AbacDecisionMode::Disabled,
    }
}

fn value_to_vec(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(v) => v.clone(),
        _ => vec![value.clone()],
    }
}

fn build_context(auth: &AuthUser, input: &PolicyInput) -> HashMap<String, Value> {
    let now = Utc::now();
    let mut out = HashMap::new();
    out.insert("subject.user_id".to_string(), json!(auth.user_id));
    out.insert("subject.email".to_string(), json!(auth.email));
    out.insert(
        "subject.token_type".to_string(),
        json!(format!("{:?}", auth.token_type)),
    );
    out.insert("subject.tenant_id".to_string(), json!(auth.tenant_id));
    out.insert("subject.roles".to_string(), json!(auth.roles));
    out.insert("subject.permissions".to_string(), json!(auth.permissions));
    if let Some(domain) = auth.email.split('@').nth(1) {
        out.insert("subject.email_domain".to_string(), json!(domain));
    }

    match &input.scope {
        ResourceScope::Tenant(tenant_id) => {
            out.insert(
                "resource.tenant_id".to_string(),
                json!(tenant_id.to_string()),
            );
        }
        ResourceScope::User(user_id) => {
            out.insert(
                "resource.target_user_id".to_string(),
                json!(user_id.to_string()),
            );
        }
        ResourceScope::Global => {}
    }

    out.insert("env.now_utc".to_string(), json!(now.to_rfc3339()));
    out.insert(
        "env.weekday".to_string(),
        json!(now.weekday().number_from_monday()),
    );
    out.insert("env.hour".to_string(), json!(now.hour()));
    out
}

fn matches_action(rule_actions: &[String], action_key: &str) -> bool {
    rule_actions.is_empty()
        || rule_actions
            .iter()
            .any(|a| a == "*" || a.eq_ignore_ascii_case(action_key))
}

fn matches_resource_type(rule_resource_types: &[String], resource_type: &str) -> bool {
    rule_resource_types.is_empty()
        || rule_resource_types
            .iter()
            .any(|t| t == "*" || t.eq_ignore_ascii_case(resource_type))
}

pub fn simulate_document(
    policy_doc: &AbacPolicyDocument,
    action_key: &str,
    resource_type: &str,
    ctx: &HashMap<String, Value>,
) -> AbacSimulationOutcome {
    let mut matched_allow_rule_ids = vec![];
    let mut matched_deny_rule_ids = vec![];
    let has_allow_rules = policy_doc
        .rules
        .iter()
        .any(|r| matches!(r.effect, AbacEffect::Allow));

    let mut rules = policy_doc.rules.clone();
    rules.sort_by_key(|r| Reverse(r.priority));

    for rule in rules {
        if !matches_action(&rule.actions, action_key) {
            continue;
        }
        if !matches_resource_type(&rule.resource_types, resource_type) {
            continue;
        }

        let matched = match rule.condition {
            None => true,
            Some(raw) => {
                let node: ConditionNode = match serde_json::from_value(raw) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                eval_condition(&node, ctx)
            }
        };

        if !matched {
            continue;
        }

        match rule.effect {
            AbacEffect::Allow => matched_allow_rule_ids.push(rule.id),
            AbacEffect::Deny => matched_deny_rule_ids.push(rule.id),
        }
    }

    let denied =
        !matched_deny_rule_ids.is_empty() || (has_allow_rules && matched_allow_rule_ids.is_empty());

    AbacSimulationOutcome {
        denied,
        matched_allow_rule_ids,
        matched_deny_rule_ids,
    }
}

fn compare_numbers(left: &Value, right: &Value, op: &str) -> bool {
    let l = left.as_f64();
    let r = right.as_f64();
    match (l, r) {
        (Some(a), Some(b)) => match op {
            "gt" => a > b,
            "gte" => a >= b,
            "lt" => a < b,
            "lte" => a <= b,
            _ => false,
        },
        _ => false,
    }
}

fn parse_time_hhmm(raw: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = raw.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let h = parts[0].parse::<u32>().ok()?;
    let m = parts[1].parse::<u32>().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

fn eval_predicate(var: &str, op: &str, expected: &Value, ctx: &HashMap<String, Value>) -> bool {
    let left = match ctx.get(var) {
        Some(v) => v,
        None => return op == "exists" && expected == &json!(false),
    };

    match op {
        "exists" => expected.as_bool().unwrap_or(true),
        "eq" => left == expected,
        "neq" => left != expected,
        "contains" => match left {
            Value::Array(arr) => arr.contains(expected),
            Value::String(s) => expected
                .as_str()
                .map(|needle| s.contains(needle))
                .unwrap_or(false),
            _ => false,
        },
        "starts_with" => left
            .as_str()
            .and_then(|s| expected.as_str().map(|p| s.starts_with(p)))
            .unwrap_or(false),
        "in" => value_to_vec(expected).contains(left),
        "not_in" => !value_to_vec(expected).contains(left),
        "gt" | "gte" | "lt" | "lte" => compare_numbers(left, expected, op),
        "ip_in_cidr" => {
            let ip = left
                .as_str()
                .and_then(|raw| IpAddr::from_str(raw).ok())
                .unwrap_or(IpAddr::from([0, 0, 0, 0]));
            let cidr = expected.as_str().unwrap_or_default();
            if let Some((base, prefix)) = cidr.split_once('/') {
                if let (Ok(base_ip), Ok(prefix_len)) =
                    (IpAddr::from_str(base), prefix.parse::<u8>())
                {
                    return match (ip, base_ip) {
                        (IpAddr::V4(ipv4), IpAddr::V4(basev4)) => {
                            if prefix_len > 32 {
                                return false;
                            }
                            let mask = if prefix_len == 0 {
                                0
                            } else {
                                u32::MAX << (32 - prefix_len)
                            };
                            (u32::from(ipv4) & mask) == (u32::from(basev4) & mask)
                        }
                        _ => false,
                    };
                }
            }
            false
        }
        "time_between" => {
            let raw = expected.as_str().unwrap_or_default();
            let parts: Vec<&str> = raw.split('-').collect();
            if parts.len() != 2 {
                return false;
            }
            let (start_h, start_m) = match parse_time_hhmm(parts[0]) {
                Some(v) => v,
                None => return false,
            };
            let (end_h, end_m) = match parse_time_hhmm(parts[1]) {
                Some(v) => v,
                None => return false,
            };
            let current = left.as_u64().unwrap_or(0) as u32;
            let start = start_h * 60 + start_m;
            let end = end_h * 60 + end_m;
            let now_minutes = current * 60;
            if start <= end {
                now_minutes >= start && now_minutes <= end
            } else {
                now_minutes >= start || now_minutes <= end
            }
        }
        _ => false,
    }
}

fn eval_condition(node: &ConditionNode, ctx: &HashMap<String, Value>) -> bool {
    match node {
        ConditionNode::All { all } => all.iter().all(|n| eval_condition(n, ctx)),
        ConditionNode::Any { any } => any.iter().any(|n| eval_condition(n, ctx)),
        ConditionNode::Not { not } => !eval_condition(not, ctx),
        ConditionNode::Predicate { var, op, value } => eval_predicate(var, op, value, ctx),
    }
}

pub async fn evaluate_with_state<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    input: &PolicyInput,
) -> Result<AbacEvaluationOutcome, AppError> {
    let tenant_id = match input.scope {
        ResourceScope::Tenant(tenant_id) => tenant_id,
        _ => {
            return Ok(AbacEvaluationOutcome {
                mode: AbacDecisionMode::Disabled,
                denied: false,
                matched_allow_rule_ids: vec![],
                matched_deny_rule_ids: vec![],
            });
        }
    };

    let Some(pool) = state.maybe_db_pool() else {
        return Ok(AbacEvaluationOutcome {
            mode: AbacDecisionMode::Disabled,
            denied: false,
            matched_allow_rule_ids: vec![],
            matched_deny_rule_ids: vec![],
        });
    };

    let row = sqlx::query_as::<_, ActivePolicyRow>(
        r#"
        SELECT ps.mode as mode, CAST(psv.policy_json AS CHAR) as policy_json
        FROM abac_policy_sets ps
        LEFT JOIN abac_policy_set_versions psv ON ps.published_version_id = psv.id
        WHERE ps.tenant_id = ?
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await;

    let row = match row {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!("ABAC lookup failed, falling back to RBAC only: {}", err);
            return Ok(AbacEvaluationOutcome {
                mode: AbacDecisionMode::Disabled,
                denied: false,
                matched_allow_rule_ids: vec![],
                matched_deny_rule_ids: vec![],
            });
        }
    };

    let Some(row) = row else {
        return Ok(AbacEvaluationOutcome {
            mode: AbacDecisionMode::Disabled,
            denied: false,
            matched_allow_rule_ids: vec![],
            matched_deny_rule_ids: vec![],
        });
    };

    let mode = mode_from_str(&row.mode);
    let Some(policy_json) = row.policy_json else {
        return Ok(AbacEvaluationOutcome {
            mode,
            denied: false,
            matched_allow_rule_ids: vec![],
            matched_deny_rule_ids: vec![],
        });
    };

    let policy_doc: AbacPolicyDocument = match serde_json::from_str(&policy_json) {
        Ok(v) => v,
        Err(err) => {
            tracing::warn!(
                "ABAC policy parse failed, falling back to RBAC only: {}",
                err
            );
            return Ok(AbacEvaluationOutcome {
                mode: AbacDecisionMode::Disabled,
                denied: false,
                matched_allow_rule_ids: vec![],
                matched_deny_rule_ids: vec![],
            });
        }
    };

    let action_key = action_to_key(input.action);
    let resource_type = scope_to_resource_type(&input.scope);
    let mut ctx = build_context(auth, input);
    ctx.insert("request.action".to_string(), json!(action_key));
    ctx.insert("resource.type".to_string(), json!(resource_type));

    let sim = simulate_document(&policy_doc, action_key, resource_type, &ctx);

    Ok(AbacEvaluationOutcome {
        mode,
        denied: sim.denied,
        matched_allow_rule_ids: sim.matched_allow_rule_ids,
        matched_deny_rule_ids: sim.matched_deny_rule_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_condition_all_any_not() {
        let mut ctx = HashMap::new();
        ctx.insert("subject.email_domain".to_string(), json!("auth9.local"));
        ctx.insert("subject.roles".to_string(), json!(["owner"]));

        let cond: ConditionNode = serde_json::from_value(json!({
            "all": [
                { "var": "subject.email_domain", "op": "eq", "value": "auth9.local" },
                { "any": [
                    { "var": "subject.roles", "op": "contains", "value": "owner" },
                    { "var": "subject.roles", "op": "contains", "value": "admin" }
                ]}
            ]
        }))
        .unwrap();

        assert!(eval_condition(&cond, &ctx));
    }

    #[test]
    fn test_simulate_document_deny_overrides_allow() {
        let policy = AbacPolicyDocument {
            rules: vec![
                crate::domain::AbacRule {
                    id: "allow_admin".to_string(),
                    effect: AbacEffect::Allow,
                    actions: vec!["user_manage".to_string()],
                    resource_types: vec!["tenant".to_string()],
                    priority: 10,
                    condition: Some(json!({
                        "var": "subject.roles",
                        "op": "contains",
                        "value": "admin"
                    })),
                },
                crate::domain::AbacRule {
                    id: "deny_off_hours".to_string(),
                    effect: AbacEffect::Deny,
                    actions: vec!["user_manage".to_string()],
                    resource_types: vec!["tenant".to_string()],
                    priority: 100,
                    condition: Some(json!({
                        "var": "env.hour",
                        "op": "gte",
                        "value": 19
                    })),
                },
            ],
        };

        let mut ctx = HashMap::new();
        ctx.insert("subject.roles".to_string(), json!(["admin"]));
        ctx.insert("env.hour".to_string(), json!(20));

        let out = simulate_document(&policy, "user_manage", "tenant", &ctx);
        assert!(out.denied);
        assert_eq!(out.matched_allow_rule_ids, vec!["allow_admin".to_string()]);
        assert_eq!(
            out.matched_deny_rule_ids,
            vec!["deny_off_hours".to_string()]
        );
    }

    #[test]
    fn test_simulate_document_default_deny_when_allow_exists_but_not_matched() {
        let policy = AbacPolicyDocument {
            rules: vec![crate::domain::AbacRule {
                id: "allow_owner".to_string(),
                effect: AbacEffect::Allow,
                actions: vec!["user_manage".to_string()],
                resource_types: vec!["tenant".to_string()],
                priority: 1,
                condition: Some(json!({
                    "var": "subject.roles",
                    "op": "contains",
                    "value": "owner"
                })),
            }],
        };

        let mut ctx = HashMap::new();
        ctx.insert("subject.roles".to_string(), json!(["member"]));

        let out = simulate_document(&policy, "user_manage", "tenant", &ctx);
        assert!(out.denied);
        assert!(out.matched_allow_rule_ids.is_empty());
        assert!(out.matched_deny_rule_ids.is_empty());
    }

    #[test]
    fn test_simulate_document_allow_when_only_deny_rules_and_none_match() {
        let policy = AbacPolicyDocument {
            rules: vec![crate::domain::AbacRule {
                id: "deny_external".to_string(),
                effect: AbacEffect::Deny,
                actions: vec!["user_manage".to_string()],
                resource_types: vec!["tenant".to_string()],
                priority: 1,
                condition: Some(json!({
                    "var": "request.ip",
                    "op": "ip_in_cidr",
                    "value": "10.0.0.0/8"
                })),
            }],
        };

        let mut ctx = HashMap::new();
        ctx.insert("request.ip".to_string(), json!("203.0.113.10"));

        let out = simulate_document(&policy, "user_manage", "tenant", &ctx);
        assert!(!out.denied);
        assert!(out.matched_allow_rule_ids.is_empty());
        assert!(out.matched_deny_rule_ids.is_empty());
    }

    #[test]
    fn test_simulate_document_ignores_invalid_condition_json() {
        let policy = AbacPolicyDocument {
            rules: vec![crate::domain::AbacRule {
                id: "bad_rule".to_string(),
                effect: AbacEffect::Deny,
                actions: vec!["user_manage".to_string()],
                resource_types: vec!["tenant".to_string()],
                priority: 1,
                condition: Some(json!({ "unexpected": true })),
            }],
        };
        let out = simulate_document(&policy, "user_manage", "tenant", &HashMap::new());
        assert!(!out.denied);
    }

    #[test]
    fn test_eval_predicate_exists_in_not_in() {
        let mut ctx = HashMap::new();
        ctx.insert("subject.roles".to_string(), json!(["admin", "member"]));
        ctx.insert("subject.region".to_string(), json!("us-east"));

        assert!(eval_predicate(
            "subject.roles",
            "exists",
            &json!(true),
            &ctx
        ));
        assert!(eval_predicate(
            "subject.region",
            "in",
            &json!(["us-east", "eu"]),
            &ctx
        ));
        assert!(eval_predicate(
            "subject.region",
            "not_in",
            &json!(["ap-south", "eu"]),
            &ctx
        ));
        assert!(eval_predicate(
            "subject.missing",
            "exists",
            &json!(false),
            &ctx
        ));
    }

    #[test]
    fn test_eval_predicate_number_and_string_ops() {
        let mut ctx = HashMap::new();
        ctx.insert("env.hour".to_string(), json!(10));
        ctx.insert("subject.email".to_string(), json!("admin@example.com"));

        assert!(eval_predicate("env.hour", "gt", &json!(9), &ctx));
        assert!(eval_predicate("env.hour", "lte", &json!(10), &ctx));
        assert!(eval_predicate(
            "subject.email",
            "starts_with",
            &json!("admin@"),
            &ctx
        ));
        assert!(eval_predicate(
            "subject.email",
            "contains",
            &json!("example"),
            &ctx
        ));
    }

    #[test]
    fn test_eval_predicate_ip_and_time_between() {
        let mut ctx = HashMap::new();
        ctx.insert("request.ip".to_string(), json!("10.1.2.3"));
        ctx.insert("env.hour".to_string(), json!(23));

        assert!(eval_predicate(
            "request.ip",
            "ip_in_cidr",
            &json!("10.0.0.0/8"),
            &ctx
        ));
        assert!(!eval_predicate(
            "request.ip",
            "ip_in_cidr",
            &json!("10.0.0.0/40"),
            &ctx
        ));
        assert!(eval_predicate(
            "env.hour",
            "time_between",
            &json!("22:00-06:00"),
            &ctx
        ));
        assert!(!eval_predicate(
            "env.hour",
            "time_between",
            &json!("09:00-18:00"),
            &ctx
        ));
    }
}
