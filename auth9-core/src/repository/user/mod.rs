//! User repository

use crate::error::Result;
use crate::models::common::StringUuid;
use crate::models::user::{
    AddUserToTenantInput, CreateUserInput, TenantUser, TenantUserWithTenant, UpdateUserInput, User,
};
use async_trait::async_trait;
use sqlx::MySqlPool;

mod impl_repo;

#[cfg(test)]
mod tests;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, identity_subject: &str, input: &CreateUserInput) -> Result<User>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn find_by_identity_subject(&self, identity_subject: &str) -> Result<Option<User>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<User>>;
    async fn count(&self) -> Result<i64>;
    async fn search(&self, query: &str, offset: i64, limit: i64) -> Result<Vec<User>>;
    async fn search_count(&self, query: &str) -> Result<i64>;
    async fn update(&self, id: StringUuid, input: &UpdateUserInput) -> Result<User>;
    async fn update_mfa_enabled(&self, id: StringUuid, enabled: bool) -> Result<User>;
    async fn delete(&self, id: StringUuid) -> Result<()>;

    // Tenant-User relations
    async fn add_to_tenant(&self, input: &AddUserToTenantInput) -> Result<TenantUser>;
    async fn update_role_in_tenant(
        &self,
        user_id: StringUuid,
        tenant_id: StringUuid,
        role: &str,
    ) -> Result<TenantUser>;
    async fn remove_from_tenant(&self, user_id: StringUuid, tenant_id: StringUuid) -> Result<()>;
    async fn find_tenant_users(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<User>>;
    async fn search_tenant_users(
        &self,
        tenant_id: StringUuid,
        query: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<User>>;
    async fn count_tenant_users(&self, tenant_id: StringUuid) -> Result<i64>;
    async fn search_tenant_users_count(&self, tenant_id: StringUuid, query: &str) -> Result<i64>;
    async fn find_user_tenants(&self, user_id: StringUuid) -> Result<Vec<TenantUser>>;

    /// Find user's tenants with tenant data (for API responses)
    async fn find_user_tenants_with_tenant(
        &self,
        user_id: StringUuid,
    ) -> Result<Vec<TenantUserWithTenant>>;

    /// Delete all tenant memberships for a user (tenant_users records)
    async fn delete_all_tenant_memberships(&self, user_id: StringUuid) -> Result<u64>;

    /// List all tenant_user IDs for a user (for cascade delete)
    async fn list_tenant_user_ids(&self, user_id: StringUuid) -> Result<Vec<StringUuid>>;

    /// List all tenant_user IDs for a tenant (for cascade delete)
    async fn list_tenant_user_ids_by_tenant(
        &self,
        tenant_id: StringUuid,
    ) -> Result<Vec<StringUuid>>;

    /// Delete all tenant memberships for a tenant
    async fn delete_tenant_memberships_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;

    /// Update password_changed_at timestamp
    async fn update_password_changed_at(&self, id: StringUuid) -> Result<()>;

    /// Update locked_until timestamp (None to unlock)
    async fn update_locked_until(
        &self,
        id: StringUuid,
        locked_until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()>;

    // SCIM provisioning methods

    /// Find user by SCIM external ID
    async fn find_by_scim_external_id(&self, scim_external_id: String) -> Result<Option<User>>;

    /// Update SCIM tracking fields on a user
    async fn update_scim_fields(
        &self,
        id: StringUuid,
        scim_external_id: Option<String>,
        scim_provisioned_by: Option<StringUuid>,
    ) -> Result<()>;
}

pub struct UserRepositoryImpl {
    pub(crate) pool: MySqlPool,
}

impl UserRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
