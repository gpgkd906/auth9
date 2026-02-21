//! ABAC policy repository.

use crate::domain::StringUuid;
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, MySqlPool};

#[derive(Debug, Clone, FromRow)]
pub struct AbacPolicySetRecord {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub mode: String,
    pub published_version_id: Option<StringUuid>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AbacPolicyVersionRecord {
    pub id: StringUuid,
    pub policy_set_id: StringUuid,
    pub version_no: i32,
    pub status: String,
    pub change_note: Option<String>,
    pub created_by: Option<StringUuid>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AbacDraftCreateResult {
    pub id: StringUuid,
    pub policy_set_id: StringUuid,
    pub version_no: i32,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbacVersionMutationOutcome {
    Applied,
    PolicySetNotFound,
    VersionNotFound,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AbacRepository: Send + Sync {
    async fn fetch_policy_set_by_tenant(
        &self,
        tenant_id: StringUuid,
    ) -> Result<Option<AbacPolicySetRecord>>;
    async fn fetch_versions_by_policy_set(
        &self,
        policy_set_id: StringUuid,
    ) -> Result<Vec<AbacPolicyVersionRecord>>;
    async fn create_draft_for_tenant(
        &self,
        tenant_id: StringUuid,
        policy_json: String,
        change_note: Option<String>,
        created_by: StringUuid,
    ) -> Result<AbacDraftCreateResult>;
    async fn update_draft_for_tenant(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        policy_json: String,
        change_note: Option<String>,
    ) -> Result<bool>;
    async fn publish_for_tenant(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        mode: &str,
    ) -> Result<AbacVersionMutationOutcome>;
    async fn rollback_for_tenant(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        mode: &str,
    ) -> Result<AbacVersionMutationOutcome>;
    async fn fetch_published_policy_json(&self, tenant_id: StringUuid) -> Result<Option<String>>;
}

pub struct AbacRepositoryImpl {
    pool: MySqlPool,
}

impl AbacRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AbacRepository for AbacRepositoryImpl {
    async fn fetch_policy_set_by_tenant(
        &self,
        tenant_id: StringUuid,
    ) -> Result<Option<AbacPolicySetRecord>> {
        sqlx::query_as::<_, AbacPolicySetRecord>(
            "SELECT id, tenant_id, mode, published_version_id FROM abac_policy_sets WHERE tenant_id = ?",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn fetch_versions_by_policy_set(
        &self,
        policy_set_id: StringUuid,
    ) -> Result<Vec<AbacPolicyVersionRecord>> {
        sqlx::query_as::<_, AbacPolicyVersionRecord>(
            r#"
            SELECT id, policy_set_id, version_no, status, change_note, created_by, created_at, published_at
            FROM abac_policy_set_versions
            WHERE policy_set_id = ?
            ORDER BY version_no DESC
            "#,
        )
        .bind(policy_set_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    async fn create_draft_for_tenant(
        &self,
        tenant_id: StringUuid,
        policy_json: String,
        change_note: Option<String>,
        created_by: StringUuid,
    ) -> Result<AbacDraftCreateResult> {
        let mut tx = self.pool.begin().await?;
        let policy_set = sqlx::query_as::<_, AbacPolicySetRecord>(
            "SELECT id, tenant_id, mode, published_version_id FROM abac_policy_sets WHERE tenant_id = ?",
        )
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?;

        let policy_set_id = if let Some(existing) = policy_set {
            existing.id
        } else {
            let id = StringUuid::new_v4();
            sqlx::query(
                "INSERT INTO abac_policy_sets (id, tenant_id, mode, published_version_id, created_at, updated_at) VALUES (?, ?, 'disabled', NULL, NOW(), NOW())",
            )
            .bind(id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;
            id
        };

        let next_version_no: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_no), 0) + 1 FROM abac_policy_set_versions WHERE policy_set_id = ?",
        )
        .bind(policy_set_id)
        .fetch_one(&mut *tx)
        .await?;

        let version_id = StringUuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO abac_policy_set_versions (id, policy_set_id, version_no, status, policy_json, change_note, created_by, created_at, published_at)
            VALUES (?, ?, ?, 'draft', ?, ?, ?, NOW(), NULL)
            "#,
        )
        .bind(version_id)
        .bind(policy_set_id)
        .bind(next_version_no)
        .bind(policy_json)
        .bind(change_note)
        .bind(created_by)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(AbacDraftCreateResult {
            id: version_id,
            policy_set_id,
            version_no: next_version_no,
            status: "draft".to_string(),
        })
    }

    async fn update_draft_for_tenant(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        policy_json: String,
        change_note: Option<String>,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE abac_policy_set_versions psv
            JOIN abac_policy_sets ps ON ps.id = psv.policy_set_id
            SET psv.policy_json = ?, psv.change_note = ?
            WHERE psv.id = ? AND ps.tenant_id = ? AND psv.status = 'draft'
            "#,
        )
        .bind(policy_json)
        .bind(change_note)
        .bind(version_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn publish_for_tenant(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        mode: &str,
    ) -> Result<AbacVersionMutationOutcome> {
        let mut tx = self.pool.begin().await?;
        let Some(set) = sqlx::query_as::<_, AbacPolicySetRecord>(
            "SELECT id, tenant_id, mode, published_version_id FROM abac_policy_sets WHERE tenant_id = ?",
        )
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(AbacVersionMutationOutcome::PolicySetNotFound);
        };

        let result = sqlx::query(
            "UPDATE abac_policy_set_versions SET status = 'published', published_at = NOW() WHERE id = ? AND policy_set_id = ?",
        )
        .bind(version_id)
        .bind(set.id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(AbacVersionMutationOutcome::VersionNotFound);
        }

        sqlx::query(
            "UPDATE abac_policy_set_versions SET status = 'archived' WHERE policy_set_id = ? AND id <> ? AND status = 'published'",
        )
        .bind(set.id)
        .bind(version_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE abac_policy_sets SET published_version_id = ?, mode = ?, updated_at = NOW() WHERE id = ?",
        )
        .bind(version_id)
        .bind(mode)
        .bind(set.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(AbacVersionMutationOutcome::Applied)
    }

    async fn rollback_for_tenant(
        &self,
        tenant_id: StringUuid,
        version_id: StringUuid,
        mode: &str,
    ) -> Result<AbacVersionMutationOutcome> {
        let mut tx = self.pool.begin().await?;
        let Some(set) = sqlx::query_as::<_, AbacPolicySetRecord>(
            "SELECT id, tenant_id, mode, published_version_id FROM abac_policy_sets WHERE tenant_id = ?",
        )
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(AbacVersionMutationOutcome::PolicySetNotFound);
        };

        let exists: Option<StringUuid> = sqlx::query_scalar(
            "SELECT id FROM abac_policy_set_versions WHERE id = ? AND policy_set_id = ?",
        )
        .bind(version_id)
        .bind(set.id)
        .fetch_optional(&mut *tx)
        .await?;
        if exists.is_none() {
            return Ok(AbacVersionMutationOutcome::VersionNotFound);
        }

        sqlx::query("UPDATE abac_policy_set_versions SET status = 'archived' WHERE policy_set_id = ? AND status = 'published'")
            .bind(set.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE abac_policy_set_versions SET status = 'published', published_at = NOW() WHERE id = ?",
        )
        .bind(version_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE abac_policy_sets SET published_version_id = ?, mode = ?, updated_at = NOW() WHERE id = ?")
            .bind(version_id)
            .bind(mode)
            .bind(set.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(AbacVersionMutationOutcome::Applied)
    }

    async fn fetch_published_policy_json(&self, tenant_id: StringUuid) -> Result<Option<String>> {
        sqlx::query_scalar(
            r#"
            SELECT CAST(psv.policy_json AS CHAR) as policy_json
            FROM abac_policy_sets ps
            JOIN abac_policy_set_versions psv ON psv.id = ps.published_version_id
            WHERE ps.tenant_id = ?
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(Into::into)
    }
}
