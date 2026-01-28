//! Data access layer (Repository pattern)

pub mod tenant;
pub mod user;
pub mod service;
pub mod rbac;
pub mod audit;

pub use tenant::TenantRepository;
pub use user::UserRepository;
pub use service::ServiceRepository;
pub use rbac::RbacRepository;
pub use audit::AuditRepository;

use sqlx::MySqlPool;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DbPool {
    pool: MySqlPool,
}

impl DbPool {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub fn inner(&self) -> &MySqlPool {
        &self.pool
    }
}

impl std::ops::Deref for DbPool {
    type Target = MySqlPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
