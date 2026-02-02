//! Invitation repository

use crate::domain::{CreateInvitationInput, Invitation, InvitationStatus, StringUuid};
use crate::error::{AppError, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait InvitationRepository: Send + Sync {
    /// Create a new invitation
    async fn create(
        &self,
        tenant_id: StringUuid,
        invited_by: StringUuid,
        input: &CreateInvitationInput,
        token_hash: &str,
    ) -> Result<Invitation>;

    /// Find invitation by ID
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Invitation>>;

    /// Find invitation by email and tenant
    async fn find_by_email_and_tenant(
        &self,
        email: &str,
        tenant_id: StringUuid,
    ) -> Result<Option<Invitation>>;

    /// List invitations for a tenant with optional status filter
    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Invitation>>;

    /// Count invitations for a tenant with optional status filter
    async fn count_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
    ) -> Result<i64>;

    /// Update invitation status
    async fn update_status(&self, id: StringUuid, status: InvitationStatus) -> Result<Invitation>;

    /// Mark invitation as accepted
    async fn mark_accepted(&self, id: StringUuid) -> Result<Invitation>;

    /// Delete an invitation
    async fn delete(&self, id: StringUuid) -> Result<()>;

    /// Expire all pending invitations that have passed their expiration date
    async fn expire_pending(&self) -> Result<u64>;

    /// Delete all invitations for a tenant (for cascade delete)
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
}

pub struct InvitationRepositoryImpl {
    pool: MySqlPool,
}

impl InvitationRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl InvitationRepository for InvitationRepositoryImpl {
    async fn create(
        &self,
        tenant_id: StringUuid,
        invited_by: StringUuid,
        input: &CreateInvitationInput,
        token_hash: &str,
    ) -> Result<Invitation> {
        let id = StringUuid::new_v4();
        let expires_in = input.expires_in_hours.unwrap_or(72);
        let expires_at = Utc::now() + Duration::hours(expires_in);
        let role_ids_json =
            serde_json::to_string(&input.role_ids).map_err(|e| AppError::Internal(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO invitations (id, tenant_id, email, role_ids, invited_by, token_hash, status, expires_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&input.email)
        .bind(&role_ids_json)
        .bind(invited_by)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to create invitation")))
    }

    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as::<_, Invitation>(
            r#"
            SELECT id, tenant_id, email, role_ids, invited_by, token_hash, status, expires_at, accepted_at, created_at, updated_at
            FROM invitations
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(invitation)
    }

    async fn find_by_email_and_tenant(
        &self,
        email: &str,
        tenant_id: StringUuid,
    ) -> Result<Option<Invitation>> {
        let invitation = sqlx::query_as::<_, Invitation>(
            r#"
            SELECT id, tenant_id, email, role_ids, invited_by, token_hash, status, expires_at, accepted_at, created_at, updated_at
            FROM invitations
            WHERE email = ? AND tenant_id = ? AND status = 'pending'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(email)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(invitation)
    }

    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<Invitation>> {
        let invitations = if let Some(status) = status {
            sqlx::query_as::<_, Invitation>(
                r#"
                SELECT id, tenant_id, email, role_ids, invited_by, token_hash, status, expires_at, accepted_at, created_at, updated_at
                FROM invitations
                WHERE tenant_id = ? AND status = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(tenant_id)
            .bind(status.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, Invitation>(
                r#"
                SELECT id, tenant_id, email, role_ids, invited_by, token_hash, status, expires_at, accepted_at, created_at, updated_at
                FROM invitations
                WHERE tenant_id = ?
                ORDER BY created_at DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(invitations)
    }

    async fn count_by_tenant(
        &self,
        tenant_id: StringUuid,
        status: Option<InvitationStatus>,
    ) -> Result<i64> {
        let row: (i64,) = if let Some(status) = status {
            sqlx::query_as("SELECT COUNT(*) FROM invitations WHERE tenant_id = ? AND status = ?")
                .bind(tenant_id)
                .bind(status.to_string())
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM invitations WHERE tenant_id = ?")
                .bind(tenant_id)
                .fetch_one(&self.pool)
                .await?
        };
        Ok(row.0)
    }

    async fn update_status(&self, id: StringUuid, status: InvitationStatus) -> Result<Invitation> {
        sqlx::query(
            r#"
            UPDATE invitations
            SET status = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))
    }

    async fn mark_accepted(&self, id: StringUuid) -> Result<Invitation> {
        sqlx::query(
            r#"
            UPDATE invitations
            SET status = 'accepted', accepted_at = NOW(), updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Invitation {} not found", id)))
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM invitations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Invitation {} not found", id)));
        }

        Ok(())
    }

    async fn expire_pending(&self) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE invitations
            SET status = 'expired', updated_at = NOW()
            WHERE status = 'pending' AND expires_at <= NOW()
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM invitations WHERE tenant_id = ?")
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_find_by_id() {
        let mut mock = MockInvitationRepository::new();
        let id = StringUuid::new_v4();
        let id_clone = id;

        mock.expect_find_by_id().with(eq(id)).returning(move |_| {
            Ok(Some(Invitation {
                id: id_clone,
                email: "test@example.com".to_string(),
                ..Default::default()
            }))
        });

        let result = mock.find_by_id(id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_mock_find_by_id_not_found() {
        let mut mock = MockInvitationRepository::new();

        mock.expect_find_by_id().returning(|_| Ok(None));

        let result = mock.find_by_id(StringUuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_list_by_tenant() {
        let mut mock = MockInvitationRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id), eq(None), eq(0), eq(10))
            .returning(|_, _, _, _| {
                Ok(vec![
                    Invitation {
                        email: "user1@example.com".to_string(),
                        ..Default::default()
                    },
                    Invitation {
                        email: "user2@example.com".to_string(),
                        ..Default::default()
                    },
                ])
            });

        let result = mock.list_by_tenant(tenant_id, None, 0, 10).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_update_status() {
        let mut mock = MockInvitationRepository::new();
        let id = StringUuid::new_v4();
        let id_clone = id;

        mock.expect_update_status()
            .with(eq(id), eq(InvitationStatus::Revoked))
            .returning(move |_, status| {
                Ok(Invitation {
                    id: id_clone,
                    status,
                    ..Default::default()
                })
            });

        let result = mock
            .update_status(id, InvitationStatus::Revoked)
            .await
            .unwrap();
        assert_eq!(result.status, InvitationStatus::Revoked);
    }

    #[tokio::test]
    async fn test_mock_expire_pending() {
        let mut mock = MockInvitationRepository::new();

        mock.expect_expire_pending().returning(|| Ok(5));

        let result = mock.expire_pending().await.unwrap();
        assert_eq!(result, 5);
    }
}
