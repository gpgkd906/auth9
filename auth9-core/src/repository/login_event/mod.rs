//! Login event repository

use crate::error::Result;
#[allow(unused_imports)]
use crate::models::analytics::{
    CreateLoginEventInput, DailyTrendPoint, LoginEvent, LoginEventType, LoginStats,
};
use crate::models::common::StringUuid;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::MySqlPool;

mod impl_repo;

#[cfg(test)]
mod tests;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LoginEventRepository: Send + Sync {
    async fn create(&self, input: &CreateLoginEventInput) -> Result<i64>;
    async fn find_by_id(&self, id: i64) -> Result<Option<LoginEvent>>;
    async fn list(&self, offset: i64, limit: i64) -> Result<Vec<LoginEvent>>;
    async fn list_by_user(
        &self,
        user_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>>;
    async fn list_by_tenant(
        &self,
        tenant_id: StringUuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<LoginEvent>>;
    async fn list_by_email(&self, email: &str, offset: i64, limit: i64) -> Result<Vec<LoginEvent>>;
    async fn count(&self) -> Result<i64>;
    async fn count_by_user(&self, user_id: StringUuid) -> Result<i64>;
    async fn count_by_tenant(&self, tenant_id: StringUuid) -> Result<i64>;
    async fn count_by_email(&self, email: &str) -> Result<i64>;
    async fn get_stats(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<LoginStats>;
    async fn count_failed_by_ip(&self, ip_address: &str, since: DateTime<Utc>) -> Result<i64>;
    async fn count_failed_by_ip_multi_user(
        &self,
        ip_address: &str,
        since: DateTime<Utc>,
    ) -> Result<i64>;
    /// Count failed login attempts for a specific user/email across all IPs (account-level detection)
    async fn count_failed_by_user(&self, email: &str, since: DateTime<Utc>) -> Result<i64>;
    async fn delete_old(&self, days: i64) -> Result<u64>;

    /// Nullify user_id for login events (preserve audit trail when user is deleted)
    async fn nullify_user_id(&self, user_id: StringUuid) -> Result<u64>;

    /// Delete all login events for a tenant (when tenant is deleted)
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;

    /// Count failed federation login events for a specific provider
    async fn count_federation_failed_by_provider(
        &self,
        provider_alias: &str,
        since: DateTime<Utc>,
    ) -> Result<i64>;

    /// Get daily trend data (per-day breakdown of logins)
    async fn get_daily_trend(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DailyTrendPoint>>;
}

pub struct LoginEventRepositoryImpl {
    pool: MySqlPool,
}

impl LoginEventRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
