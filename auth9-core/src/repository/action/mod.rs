//! Action repository

use crate::error::Result;
use crate::models::action::{
    Action, ActionExecution, CreateActionInput, LogQueryFilter, UpdateActionInput,
};
use crate::models::common::StringUuid;
use async_trait::async_trait;
use sqlx::MySqlPool;

mod impl_repo;

#[cfg(test)]
mod tests;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(clippy::too_many_arguments)]
pub trait ActionRepository: Send + Sync {
    async fn create(
        &self,
        tenant_id: Option<StringUuid>,
        service_id: StringUuid,
        input: &CreateActionInput,
    ) -> Result<Action>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Action>>;
    async fn list_by_service(&self, service_id: StringUuid) -> Result<Vec<Action>>;
    async fn list_by_trigger(
        &self,
        service_id: StringUuid,
        trigger_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<Action>>;
    /// List actions by tenant and trigger (for PostChangePassword fallback where no service_id is available)
    async fn list_by_tenant_trigger(
        &self,
        tenant_id: StringUuid,
        trigger_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<Action>>;
    async fn update(&self, id: StringUuid, input: &UpdateActionInput) -> Result<Action>;
    async fn delete(&self, id: StringUuid) -> Result<()>;
    async fn delete_by_service(&self, service_id: StringUuid) -> Result<u64>;
    async fn delete_by_tenant(&self, tenant_id: StringUuid) -> Result<u64>;
    async fn record_execution(
        &self,
        action_id: StringUuid,
        tenant_id: Option<StringUuid>,
        service_id: StringUuid,
        trigger_id: String,
        user_id: Option<StringUuid>,
        success: bool,
        duration_ms: i32,
        error: Option<String>,
    ) -> Result<()>;
    async fn update_execution_stats(
        &self,
        id: StringUuid,
        success: bool,
        error: Option<String>,
    ) -> Result<()>;
    async fn find_execution_by_id(&self, id: StringUuid) -> Result<Option<ActionExecution>>;
    async fn query_logs(&self, filter: &LogQueryFilter) -> Result<Vec<ActionExecution>>;
    async fn count_logs(&self, filter: &LogQueryFilter) -> Result<i64>;
    async fn get_stats(&self, action_id: StringUuid) -> Result<Option<(i64, i64, f64, i64)>>;
}

pub struct ActionRepositoryImpl {
    pool: MySqlPool,
}

impl ActionRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
