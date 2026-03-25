//! LDAP Group-Role mapping repository

use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use crate::models::ldap::{CreateLdapGroupRoleMappingInput, LdapGroupRoleMapping};
use async_trait::async_trait;
use sqlx::MySqlPool;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LdapGroupRoleMappingRepository: Send + Sync {
    async fn list_by_connector(
        &self,
        connector_id: StringUuid,
    ) -> Result<Vec<LdapGroupRoleMapping>>;
    async fn create(
        &self,
        tenant_id: StringUuid,
        connector_id: StringUuid,
        input: &CreateLdapGroupRoleMappingInput,
    ) -> Result<LdapGroupRoleMapping>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_connector(&self, connector_id: StringUuid) -> Result<u64>;
    async fn find_roles_for_groups(
        &self,
        connector_id: StringUuid,
        group_dns: &[String],
    ) -> Result<Vec<StringUuid>>;
}

pub struct LdapGroupRoleMappingRepositoryImpl {
    pool: MySqlPool,
}

impl LdapGroupRoleMappingRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LdapGroupRoleMappingRepository for LdapGroupRoleMappingRepositoryImpl {
    async fn list_by_connector(
        &self,
        connector_id: StringUuid,
    ) -> Result<Vec<LdapGroupRoleMapping>> {
        let mappings = sqlx::query_as::<_, LdapGroupRoleMapping>(
            r#"
            SELECT id, tenant_id, connector_id, ldap_group_dn, ldap_group_display_name,
                   role_id, created_at, updated_at
            FROM ldap_group_role_mappings
            WHERE connector_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(connector_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(mappings)
    }

    async fn create(
        &self,
        tenant_id: StringUuid,
        connector_id: StringUuid,
        input: &CreateLdapGroupRoleMappingInput,
    ) -> Result<LdapGroupRoleMapping> {
        let id = StringUuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO ldap_group_role_mappings
                (id, tenant_id, connector_id, ldap_group_dn, ldap_group_display_name, role_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(connector_id)
        .bind(&input.ldap_group_dn)
        .bind(&input.ldap_group_display_name)
        .bind(input.role_id)
        .execute(&self.pool)
        .await?;

        let mapping = sqlx::query_as::<_, LdapGroupRoleMapping>(
            r#"
            SELECT id, tenant_id, connector_id, ldap_group_dn, ldap_group_display_name,
                   role_id, created_at, updated_at
            FROM ldap_group_role_mappings
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(mapping)
    }

    async fn delete(&self, id: StringUuid) -> Result<()> {
        let result = sqlx::query("DELETE FROM ldap_group_role_mappings WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "LDAP group mapping {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn delete_by_connector(&self, connector_id: StringUuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM ldap_group_role_mappings WHERE connector_id = ?")
            .bind(connector_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn find_roles_for_groups(
        &self,
        connector_id: StringUuid,
        group_dns: &[String],
    ) -> Result<Vec<StringUuid>> {
        if group_dns.is_empty() {
            return Ok(vec![]);
        }

        // Build dynamic IN clause
        let placeholders: Vec<&str> = group_dns.iter().map(|_| "?").collect();
        let query = format!(
            "SELECT DISTINCT role_id FROM ldap_group_role_mappings WHERE connector_id = ? AND ldap_group_dn IN ({})",
            placeholders.join(",")
        );

        let mut q = sqlx::query_scalar::<_, StringUuid>(&query).bind(connector_id);
        for dn in group_dns {
            q = q.bind(dn);
        }

        let role_ids = q.fetch_all(&self.pool).await?;
        Ok(role_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[tokio::test]
    async fn test_mock_list_by_connector_empty() {
        let mut mock = MockLdapGroupRoleMappingRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_list_by_connector()
            .with(eq(cid))
            .returning(|_| Ok(vec![]));

        let result = mock.list_by_connector(cid).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_find_roles_for_groups_empty_input() {
        let mut mock = MockLdapGroupRoleMappingRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_find_roles_for_groups()
            .returning(|_, _| Ok(vec![]));

        let result = mock
            .find_roles_for_groups(cid, &[])
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_mock_delete_by_connector() {
        let mut mock = MockLdapGroupRoleMappingRepository::new();
        let cid = StringUuid::new_v4();
        mock.expect_delete_by_connector()
            .with(eq(cid))
            .returning(|_| Ok(3));

        let count = mock.delete_by_connector(cid).await.unwrap();
        assert_eq!(count, 3);
    }
}
