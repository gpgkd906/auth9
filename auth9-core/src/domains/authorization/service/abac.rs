use crate::domain::{
    AbacMode, AbacPolicyDocument, AbacPolicySetSummary, AbacPolicyVersionSummary,
    AbacSimulationInput, AbacSimulationResult, StringUuid,
};
use crate::error::{AppError, Result};
use crate::policy::abac::simulate_document;
use crate::repository::abac::{
    AbacDraftCreateResult, AbacPolicySetRecord, AbacPolicyVersionRecord, AbacVersionMutationOutcome,
};
use crate::repository::AbacRepository;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AbacPolicyListPayload {
    pub policy_set: Option<AbacPolicySetSummary>,
    pub versions: Vec<AbacPolicyVersionSummary>,
}

pub struct AbacPolicyService<R: AbacRepository> {
    repo: Arc<R>,
}

impl<R: AbacRepository> AbacPolicyService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn list_policies(&self, tenant_id: StringUuid) -> Result<AbacPolicyListPayload> {
        let set = self.repo.fetch_policy_set_by_tenant(tenant_id).await?;
        let Some(set) = set else {
            return Ok(AbacPolicyListPayload {
                policy_set: None,
                versions: vec![],
            });
        };
        let versions = self.repo.fetch_versions_by_policy_set(set.id).await?;
        Ok(AbacPolicyListPayload {
            policy_set: Some(build_policy_set_summary(&set, &versions)),
            versions: versions.into_iter().map(map_version_summary).collect(),
        })
    }

    pub async fn create_policy(
        &self,
        tenant_id: StringUuid,
        policy: AbacPolicyDocument,
        change_note: Option<String>,
        created_by: StringUuid,
    ) -> Result<AbacDraftCreateResult> {
        let policy_json =
            serde_json::to_string(&policy).map_err(|e| AppError::Internal(e.into()))?;
        self.repo
            .create_draft_for_tenant(tenant_id, policy_json, change_note, created_by)
            .await
    }

    pub async fn update_policy(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        policy: AbacPolicyDocument,
        change_note: Option<String>,
    ) -> Result<()> {
        let policy_json =
            serde_json::to_string(&policy).map_err(|e| AppError::Internal(e.into()))?;
        let updated = self
            .repo
            .update_draft_for_tenant(tenant_id, version_id, policy_json, change_note)
            .await?;
        if !updated {
            return Err(AppError::BadRequest(
                "Draft policy version not found or already published".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn publish_policy(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        mode: AbacMode,
    ) -> Result<()> {
        match self
            .repo
            .publish_for_tenant(tenant_id, version_id, mode_to_str(mode))
            .await?
        {
            AbacVersionMutationOutcome::Applied => Ok(()),
            AbacVersionMutationOutcome::PolicySetNotFound => {
                Err(AppError::NotFound("ABAC policy set not found".to_string()))
            }
            AbacVersionMutationOutcome::VersionNotFound => Err(AppError::NotFound(
                "ABAC policy version not found".to_string(),
            )),
        }
    }

    pub async fn rollback_policy(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        mode: AbacMode,
    ) -> Result<()> {
        match self
            .repo
            .rollback_for_tenant(tenant_id, version_id, mode_to_str(mode))
            .await?
        {
            AbacVersionMutationOutcome::Applied => Ok(()),
            AbacVersionMutationOutcome::PolicySetNotFound => {
                Err(AppError::NotFound("ABAC policy set not found".to_string()))
            }
            AbacVersionMutationOutcome::VersionNotFound => Err(AppError::NotFound(
                "ABAC policy version not found".to_string(),
            )),
        }
    }

    pub async fn simulate_policy(
        &self,
        tenant_id: StringUuid,
        policy: Option<AbacPolicyDocument>,
        simulation: AbacSimulationInput,
    ) -> Result<AbacSimulationResult> {
        let policy_doc = if let Some(doc) = policy {
            doc
        } else {
            let Some(raw) = self.repo.fetch_published_policy_json(tenant_id).await? else {
                return Err(AppError::BadRequest(
                    "No published ABAC policy found; provide policy in request".to_string(),
                ));
            };
            serde_json::from_str(&raw)
                .map_err(|e| AppError::BadRequest(format!("Invalid policy JSON: {e}")))?
        };

        let ctx = build_flattened_context(&simulation);
        let sim = simulate_document(
            &policy_doc,
            &simulation.action,
            &simulation.resource_type,
            &ctx,
        );
        Ok(AbacSimulationResult {
            decision: if sim.denied { "deny" } else { "allow" }.to_string(),
            matched_allow_rule_ids: sim.matched_allow_rule_ids,
            matched_deny_rule_ids: sim.matched_deny_rule_ids,
        })
    }
}

fn mode_to_str(mode: AbacMode) -> &'static str {
    match mode {
        AbacMode::Disabled => "disabled",
        AbacMode::Shadow => "shadow",
        AbacMode::Enforce => "enforce",
    }
}

fn mode_from_str(mode: &str) -> AbacMode {
    match mode {
        "shadow" => AbacMode::Shadow,
        "enforce" => AbacMode::Enforce,
        _ => AbacMode::Disabled,
    }
}

fn build_policy_set_summary(
    set: &AbacPolicySetRecord,
    versions: &[AbacPolicyVersionRecord],
) -> AbacPolicySetSummary {
    AbacPolicySetSummary {
        policy_set_id: set.id.to_string(),
        tenant_id: set.tenant_id.to_string(),
        mode: mode_from_str(&set.mode),
        published_version_id: set.published_version_id.map(|v| v.to_string()),
        published_version_no: versions
            .iter()
            .find(|v| Some(v.id) == set.published_version_id)
            .map(|v| v.version_no),
    }
}

fn map_version_summary(v: AbacPolicyVersionRecord) -> AbacPolicyVersionSummary {
    AbacPolicyVersionSummary {
        id: v.id.to_string(),
        policy_set_id: v.policy_set_id.to_string(),
        version_no: v.version_no,
        status: v.status,
        change_note: v.change_note,
        created_by: v.created_by.map(|x| x.to_string()),
        created_at: v.created_at.to_rfc3339(),
        published_at: v.published_at.map(|t| t.to_rfc3339()),
    }
}

fn build_flattened_context(input: &AbacSimulationInput) -> HashMap<String, Value> {
    let mut ctx = HashMap::new();
    if let Value::Object(map) = &input.subject {
        for (k, v) in map {
            ctx.insert(format!("subject.{k}"), v.clone());
        }
    }
    if let Value::Object(map) = &input.resource {
        for (k, v) in map {
            ctx.insert(format!("resource.{k}"), v.clone());
        }
    }
    if let Value::Object(map) = &input.request {
        for (k, v) in map {
            ctx.insert(format!("request.{k}"), v.clone());
        }
    }
    if let Value::Object(map) = &input.env {
        for (k, v) in map {
            ctx.insert(format!("env.{k}"), v.clone());
        }
    }
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::AbacEffect;
    use crate::domain::AbacRule;
    use crate::repository::abac::MockAbacRepository;
    use mockall::predicate::eq;

    fn sample_policy() -> AbacPolicyDocument {
        AbacPolicyDocument {
            rules: vec![AbacRule {
                id: "allow_admin".to_string(),
                effect: AbacEffect::Allow,
                actions: vec!["user_manage".to_string()],
                resource_types: vec!["tenant".to_string()],
                priority: 10,
                condition: Some(serde_json::json!({
                    "var": "subject.roles",
                    "op": "contains",
                    "value": "admin"
                })),
            }],
        }
    }

    #[tokio::test]
    async fn test_list_policies_maps_set_and_versions() {
        let mut repo = MockAbacRepository::new();
        let tenant_id = StringUuid::new_v4();
        let set_id = StringUuid::new_v4();
        let version_id = StringUuid::new_v4();

        repo.expect_fetch_policy_set_by_tenant()
            .with(eq(tenant_id))
            .return_once(move |_| {
                Ok(Some(AbacPolicySetRecord {
                    id: set_id,
                    tenant_id,
                    mode: "shadow".to_string(),
                    published_version_id: Some(version_id),
                }))
            });
        repo.expect_fetch_versions_by_policy_set()
            .with(eq(set_id))
            .return_once(move |_| {
                Ok(vec![AbacPolicyVersionRecord {
                    id: version_id,
                    policy_set_id: set_id,
                    version_no: 2,
                    status: "published".to_string(),
                    change_note: Some("note".to_string()),
                    created_by: None,
                    created_at: chrono::Utc::now(),
                    published_at: None,
                }])
            });

        let svc = AbacPolicyService::new(Arc::new(repo));
        let out = svc.list_policies(tenant_id).await.unwrap();
        assert_eq!(out.policy_set.as_ref().unwrap().mode, AbacMode::Shadow);
        assert_eq!(
            out.policy_set.as_ref().unwrap().published_version_no,
            Some(2)
        );
        assert_eq!(out.versions.len(), 1);
    }

    #[tokio::test]
    async fn test_create_policy_calls_repo() {
        let mut repo = MockAbacRepository::new();
        let tenant_id = StringUuid::new_v4();
        let created_by = StringUuid::new_v4();
        repo.expect_create_draft_for_tenant()
            .withf(
                move |tid: &StringUuid, _raw: &String, note: &Option<String>, by: &StringUuid| {
                    *tid == tenant_id && note.as_deref() == Some("note") && *by == created_by
                },
            )
            .return_once(move |_, _, _, _| {
                Ok(AbacDraftCreateResult {
                    id: StringUuid::new_v4(),
                    policy_set_id: StringUuid::new_v4(),
                    version_no: 1,
                    status: "draft".to_string(),
                })
            });
        let svc = AbacPolicyService::new(Arc::new(repo));
        let out = svc
            .create_policy(
                tenant_id,
                sample_policy(),
                Some("note".to_string()),
                created_by,
            )
            .await
            .unwrap();
        assert_eq!(out.status, "draft");
    }

    #[tokio::test]
    async fn test_publish_policy_maps_not_found() {
        let mut repo = MockAbacRepository::new();
        let tenant_id = StringUuid::new_v4();
        let version_id = StringUuid::new_v4();
        repo.expect_publish_for_tenant()
            .with(eq(tenant_id), eq(version_id), eq("enforce"))
            .return_once(|_, _, _| Ok(AbacVersionMutationOutcome::VersionNotFound));
        let svc = AbacPolicyService::new(Arc::new(repo));
        let err = svc
            .publish_policy(tenant_id, version_id, AbacMode::Enforce)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_rollback_policy_maps_applied() {
        let mut repo = MockAbacRepository::new();
        let tenant_id = StringUuid::new_v4();
        let version_id = StringUuid::new_v4();
        repo.expect_rollback_for_tenant()
            .with(eq(tenant_id), eq(version_id), eq("shadow"))
            .return_once(|_, _, _| Ok(AbacVersionMutationOutcome::Applied));
        let svc = AbacPolicyService::new(Arc::new(repo));
        svc.rollback_policy(tenant_id, version_id, AbacMode::Shadow)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_simulate_policy_with_inline_doc() {
        let repo = MockAbacRepository::new();
        let svc = AbacPolicyService::new(Arc::new(repo));
        let tenant_id = StringUuid::new_v4();
        let out = svc
            .simulate_policy(
                tenant_id,
                Some(sample_policy()),
                AbacSimulationInput {
                    action: "user_manage".to_string(),
                    resource_type: "tenant".to_string(),
                    subject: serde_json::json!({"roles": ["admin"]}),
                    resource: serde_json::json!({}),
                    request: serde_json::json!({}),
                    env: serde_json::json!({}),
                },
            )
            .await
            .unwrap();
        assert_eq!(out.decision, "allow");
        assert_eq!(out.matched_allow_rule_ids, vec!["allow_admin".to_string()]);
    }

    #[tokio::test]
    async fn test_simulate_policy_without_published_doc_fails() {
        let mut repo = MockAbacRepository::new();
        let tenant_id = StringUuid::new_v4();
        repo.expect_fetch_published_policy_json()
            .with(eq(tenant_id))
            .return_once(|_| Ok(None));
        let svc = AbacPolicyService::new(Arc::new(repo));
        let err = svc
            .simulate_policy(
                tenant_id,
                None,
                AbacSimulationInput {
                    action: "user_manage".to_string(),
                    resource_type: "tenant".to_string(),
                    subject: serde_json::json!({}),
                    resource: serde_json::json!({}),
                    request: serde_json::json!({}),
                    env: serde_json::json!({}),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
