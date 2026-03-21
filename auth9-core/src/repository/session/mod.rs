//! Session repository

use crate::error::Result;
use crate::models::common::StringUuid;
use crate::models::session::{CreateSessionInput, Session};
use async_trait::async_trait;
use sqlx::MySqlPool;

mod impl_repo;

#[cfg(test)]
mod tests;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create(&self, input: &CreateSessionInput) -> Result<Session>;
    async fn find_by_id(&self, id: StringUuid) -> Result<Option<Session>>;
    async fn find_by_provider_session_id(
        &self,
        provider_session_id: &str,
    ) -> Result<Option<Session>>;
    async fn list_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>>;
    async fn list_active_by_user(&self, user_id: StringUuid) -> Result<Vec<Session>>;
    async fn update_last_active(&self, id: StringUuid) -> Result<()>;
    async fn revoke(&self, id: StringUuid) -> Result<()>;
    async fn revoke_all_by_user(&self, user_id: StringUuid) -> Result<u64>;
    async fn revoke_all_except(&self, user_id: StringUuid, except_id: StringUuid) -> Result<u64>;
    async fn delete_old(&self, days: i64) -> Result<u64>;

    /// Delete all sessions for a user (for cascade delete)
    async fn delete_by_user(&self, user_id: StringUuid) -> Result<u64>;

    /// Count active sessions for a user (for session concurrency limit)
    async fn count_active_by_user(&self, user_id: StringUuid) -> Result<i64>;

    /// Find the oldest active session for a user (for evicting when limit exceeded)
    async fn find_oldest_active_by_user(&self, user_id: StringUuid) -> Result<Option<Session>>;
}

pub struct SessionRepositoryImpl {
    pool: MySqlPool,
}

impl SessionRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}
