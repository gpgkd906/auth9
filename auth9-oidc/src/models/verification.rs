use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User email verification status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserVerificationStatus {
    pub user_id: String,
    pub email_verified: bool,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_verification_status() {
        let status = UserVerificationStatus {
            user_id: "user-1".to_string(),
            email_verified: false,
            email_verified_at: None,
            updated_at: Utc::now(),
        };
        assert!(!status.email_verified);
        assert!(status.email_verified_at.is_none());
    }

    #[test]
    fn verified_status_has_timestamp() {
        let now = Utc::now();
        let status = UserVerificationStatus {
            user_id: "user-1".to_string(),
            email_verified: true,
            email_verified_at: Some(now),
            updated_at: now,
        };
        assert!(status.email_verified);
        assert!(status.email_verified_at.is_some());
    }
}
