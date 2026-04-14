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

/// Email verification token row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerificationToken {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Input for creating an email verification token.
#[derive(Debug, Clone)]
pub struct CreateVerificationTokenInput {
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
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

    #[test]
    fn email_verification_token_construction() {
        let now = Utc::now();
        let token = EmailVerificationToken {
            id: "tok-1".to_string(),
            user_id: "user-1".to_string(),
            token_hash: "sha256hash".to_string(),
            expires_at: now + chrono::Duration::hours(24),
            used_at: None,
            created_at: now,
        };
        assert!(token.used_at.is_none());
        assert!(token.expires_at > now);
    }

    #[test]
    fn create_verification_token_input() {
        let input = CreateVerificationTokenInput {
            user_id: "user-1".to_string(),
            token_hash: "hash".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        };
        assert_eq!(input.user_id, "user-1");
    }
}
