//! Data access layer (Repository pattern)

pub mod audit;
pub mod invitation;
pub mod linked_identity;
pub mod login_event;
pub mod password_reset;
pub mod rbac;
pub mod security_alert;
pub mod service;
pub mod session;
pub mod system_settings;
pub mod tenant;
pub mod user;
pub mod webhook;

pub use audit::AuditRepository;
pub use invitation::InvitationRepository;
pub use linked_identity::LinkedIdentityRepository;
pub use login_event::LoginEventRepository;
pub use password_reset::PasswordResetRepository;
pub use rbac::RbacRepository;
pub use security_alert::SecurityAlertRepository;
pub use service::ServiceRepository;
pub use session::SessionRepository;
pub use system_settings::SystemSettingsRepository;
pub use tenant::TenantRepository;
pub use user::UserRepository;
pub use webhook::WebhookRepository;

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
