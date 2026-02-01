//! Invitation domain types

use super::common::StringUuid;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// Invitation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InvitationStatus {
    #[default]
    Pending,
    Accepted,
    Expired,
    Revoked,
}

impl std::str::FromStr for InvitationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "expired" => Ok(Self::Expired),
            "revoked" => Ok(Self::Revoked),
            _ => Err(format!("Unknown invitation status: {}", s)),
        }
    }
}

impl std::fmt::Display for InvitationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Accepted => write!(f, "accepted"),
            Self::Expired => write!(f, "expired"),
            Self::Revoked => write!(f, "revoked"),
        }
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for InvitationStatus {
    fn decode(value: sqlx::mysql::MySqlValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s: String = sqlx::Decode::<'r, sqlx::MySql>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Type<sqlx::MySql> for InvitationStatus {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for InvitationStatus {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.to_string();
        <&str as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&s.as_str(), buf)
    }
}

/// Invitation entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invitation {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub email: String,
    #[sqlx(json)]
    pub role_ids: Vec<StringUuid>,
    pub invited_by: StringUuid,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub status: InvitationStatus,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Invitation {
    /// Check if the invitation is still valid (pending and not expired)
    pub fn is_valid(&self) -> bool {
        self.status == InvitationStatus::Pending && self.expires_at > Utc::now()
    }

    /// Check if the invitation has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at <= Utc::now()
    }
}

impl Default for Invitation {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: StringUuid::new_v4(),
            tenant_id: StringUuid::new_v4(),
            email: String::new(),
            role_ids: Vec::new(),
            invited_by: StringUuid::new_v4(),
            token_hash: String::new(),
            status: InvitationStatus::default(),
            expires_at: now + Duration::hours(72),
            accepted_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for creating a new invitation
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateInvitationInput {
    /// Email address to invite
    #[validate(email)]
    pub email: String,

    /// Role IDs to assign when invitation is accepted
    #[validate(length(min = 1, message = "At least one role is required"))]
    pub role_ids: Vec<StringUuid>,

    /// Custom expiration in hours (default: 72)
    #[validate(range(min = 1, max = 720))]
    pub expires_in_hours: Option<i64>,
}

/// API response for invitation list (without sensitive token_hash)
#[derive(Debug, Clone, Serialize)]
pub struct InvitationResponse {
    pub id: StringUuid,
    pub tenant_id: StringUuid,
    pub email: String,
    pub role_ids: Vec<StringUuid>,
    pub invited_by: StringUuid,
    pub status: InvitationStatus,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Invitation> for InvitationResponse {
    fn from(inv: Invitation) -> Self {
        Self {
            id: inv.id,
            tenant_id: inv.tenant_id,
            email: inv.email,
            role_ids: inv.role_ids,
            invited_by: inv.invited_by,
            status: inv.status,
            expires_at: inv.expires_at,
            accepted_at: inv.accepted_at,
            created_at: inv.created_at,
        }
    }
}

/// Input for accepting an invitation
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AcceptInvitationInput {
    /// The invitation token received via email
    #[validate(length(min = 1))]
    pub token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitation_status_default() {
        let status = InvitationStatus::default();
        assert_eq!(status, InvitationStatus::Pending);
    }

    #[test]
    fn test_invitation_status_from_str() {
        assert_eq!(
            "pending".parse::<InvitationStatus>().unwrap(),
            InvitationStatus::Pending
        );
        assert_eq!(
            "ACCEPTED".parse::<InvitationStatus>().unwrap(),
            InvitationStatus::Accepted
        );
        assert_eq!(
            "expired".parse::<InvitationStatus>().unwrap(),
            InvitationStatus::Expired
        );
        assert_eq!(
            "revoked".parse::<InvitationStatus>().unwrap(),
            InvitationStatus::Revoked
        );
        assert!("invalid".parse::<InvitationStatus>().is_err());
    }

    #[test]
    fn test_invitation_status_display() {
        assert_eq!(format!("{}", InvitationStatus::Pending), "pending");
        assert_eq!(format!("{}", InvitationStatus::Accepted), "accepted");
        assert_eq!(format!("{}", InvitationStatus::Expired), "expired");
        assert_eq!(format!("{}", InvitationStatus::Revoked), "revoked");
    }

    #[test]
    fn test_invitation_default() {
        let inv = Invitation::default();
        assert!(!inv.id.is_nil());
        assert_eq!(inv.status, InvitationStatus::Pending);
        assert!(inv.role_ids.is_empty());
        assert!(inv.accepted_at.is_none());
    }

    #[test]
    fn test_invitation_is_valid() {
        let inv = Invitation {
            status: InvitationStatus::Pending,
            expires_at: Utc::now() + Duration::hours(1),
            ..Default::default()
        };
        assert!(inv.is_valid());
    }

    #[test]
    fn test_invitation_is_expired() {
        let inv = Invitation {
            status: InvitationStatus::Pending,
            expires_at: Utc::now() - Duration::hours(1),
            ..Default::default()
        };
        assert!(inv.is_expired());
        assert!(!inv.is_valid());
    }

    #[test]
    fn test_invitation_not_valid_if_accepted() {
        let inv = Invitation {
            status: InvitationStatus::Accepted,
            expires_at: Utc::now() + Duration::hours(1),
            ..Default::default()
        };
        assert!(!inv.is_valid());
    }

    #[test]
    fn test_create_invitation_input_validation() {
        let input = CreateInvitationInput {
            email: "valid@example.com".to_string(),
            role_ids: vec![StringUuid::new_v4()],
            expires_in_hours: Some(48),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_invitation_input_invalid_email() {
        let input = CreateInvitationInput {
            email: "not-an-email".to_string(),
            role_ids: vec![StringUuid::new_v4()],
            expires_in_hours: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_invitation_input_empty_roles() {
        let input = CreateInvitationInput {
            email: "valid@example.com".to_string(),
            role_ids: vec![],
            expires_in_hours: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_invitation_response_from_invitation() {
        let inv = Invitation {
            email: "test@example.com".to_string(),
            token_hash: "secret-hash".to_string(),
            ..Default::default()
        };

        let response: InvitationResponse = inv.into();
        assert_eq!(response.email, "test@example.com");
        // token_hash should not be in response (it's excluded via From impl)
    }

    #[test]
    fn test_invitation_status_serialization() {
        let status = InvitationStatus::Pending;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"pending\"");

        let parsed: InvitationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }
}
