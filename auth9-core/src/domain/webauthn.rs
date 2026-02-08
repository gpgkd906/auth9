//! WebAuthn/Passkey domain models
//!
//! Native WebAuthn credentials are stored in TiDB.
//! Keycloak credentials are supported during migration period.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// WebAuthn credential info from Keycloak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredential {
    pub id: String,
    pub credential_type: String,
    pub user_label: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Stored passkey credential (TiDB entity)
#[derive(Debug, Clone, FromRow)]
pub struct StoredPasskey {
    pub id: String,
    pub user_id: String,
    pub credential_id: String,
    pub credential_data: serde_json::Value,
    pub user_label: Option<String>,
    pub aaguid: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Input for creating a new passkey credential
#[derive(Debug, Clone)]
pub struct CreatePasskeyInput {
    pub id: String,
    pub user_id: String,
    pub credential_id: String,
    pub credential_data: serde_json::Value,
    pub user_label: Option<String>,
    pub aaguid: Option<String>,
}

impl From<StoredPasskey> for WebAuthnCredential {
    fn from(stored: StoredPasskey) -> Self {
        Self {
            id: stored.id,
            credential_type: "webauthn".to_string(),
            user_label: stored.user_label,
            created_at: Some(stored.created_at),
        }
    }
}

/// Keycloak credential representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakCredential {
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: String,
    pub user_label: Option<String>,
    pub created_date: Option<i64>,
    #[serde(default)]
    pub credential_data: Option<String>,
}

impl From<KeycloakCredential> for WebAuthnCredential {
    fn from(cred: KeycloakCredential) -> Self {
        let created_at = cred
            .created_date
            .map(|ts| DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now));

        Self {
            id: cred.id,
            credential_type: cred.credential_type,
            user_label: cred.user_label,
            created_at,
        }
    }
}

impl WebAuthnCredential {
    /// Check if this is a WebAuthn credential
    pub fn is_webauthn(&self) -> bool {
        let ct = self.credential_type.to_lowercase();
        ct.contains("webauthn")
    }

    /// Check if this is a passwordless WebAuthn credential
    pub fn is_passwordless(&self) -> bool {
        self.credential_type
            .to_lowercase()
            .contains("webauthn-passwordless")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webauthn_credential_from_keycloak() {
        let kc_cred = KeycloakCredential {
            id: "cred-123".to_string(),
            credential_type: "webauthn".to_string(),
            user_label: Some("My YubiKey".to_string()),
            created_date: Some(1700000000000),
            credential_data: None,
        };

        let cred: WebAuthnCredential = kc_cred.into();
        assert_eq!(cred.id, "cred-123");
        assert_eq!(cred.credential_type, "webauthn");
        assert_eq!(cred.user_label, Some("My YubiKey".to_string()));
        assert!(cred.created_at.is_some());
    }

    #[test]
    fn test_webauthn_credential_is_webauthn() {
        let cred = WebAuthnCredential {
            id: "cred-123".to_string(),
            credential_type: "webauthn".to_string(),
            user_label: None,
            created_at: None,
        };
        assert!(cred.is_webauthn());
        assert!(!cred.is_passwordless());
    }

    #[test]
    fn test_webauthn_credential_is_passwordless() {
        let cred = WebAuthnCredential {
            id: "cred-123".to_string(),
            credential_type: "webauthn-passwordless".to_string(),
            user_label: None,
            created_at: None,
        };
        assert!(cred.is_webauthn());
        assert!(cred.is_passwordless());
    }

    #[test]
    fn test_keycloak_credential_deserialization() {
        let json = r#"{
            "id": "cred-456",
            "type": "webauthn",
            "userLabel": "TouchID",
            "createdDate": 1700000000000
        }"#;

        let cred: KeycloakCredential = serde_json::from_str(json).unwrap();
        assert_eq!(cred.id, "cred-456");
        assert_eq!(cred.credential_type, "webauthn");
        assert_eq!(cred.user_label, Some("TouchID".to_string()));
    }

    #[test]
    fn test_keycloak_credential_minimal() {
        let json = r#"{"id": "cred-789", "type": "password"}"#;

        let cred: KeycloakCredential = serde_json::from_str(json).unwrap();
        assert_eq!(cred.id, "cred-789");
        assert!(cred.user_label.is_none());
        assert!(cred.created_date.is_none());
    }

    #[test]
    fn test_stored_passkey_to_webauthn_credential() {
        let stored = StoredPasskey {
            id: "pk-123".to_string(),
            user_id: "user-456".to_string(),
            credential_id: "cred-id-base64url".to_string(),
            credential_data: serde_json::json!({"key": "data"}),
            user_label: Some("My MacBook".to_string()),
            aaguid: Some("aaguid-123".to_string()),
            created_at: Utc::now(),
            last_used_at: None,
        };

        let cred: WebAuthnCredential = stored.into();
        assert_eq!(cred.id, "pk-123");
        assert_eq!(cred.credential_type, "webauthn");
        assert_eq!(cred.user_label, Some("My MacBook".to_string()));
        assert!(cred.created_at.is_some());
    }

    #[test]
    fn test_stored_passkey_without_label() {
        let stored = StoredPasskey {
            id: "pk-789".to_string(),
            user_id: "user-456".to_string(),
            credential_id: "cred-id-2".to_string(),
            credential_data: serde_json::json!({}),
            user_label: None,
            aaguid: None,
            created_at: Utc::now(),
            last_used_at: Some(Utc::now()),
        };

        let cred: WebAuthnCredential = stored.into();
        assert_eq!(cred.id, "pk-789");
        assert!(cred.user_label.is_none());
    }

    #[test]
    fn test_create_passkey_input() {
        let input = CreatePasskeyInput {
            id: "pk-new".to_string(),
            user_id: "user-123".to_string(),
            credential_id: "cred-new".to_string(),
            credential_data: serde_json::json!({"type": "public-key"}),
            user_label: Some("Test Key".to_string()),
            aaguid: Some("aaguid-test".to_string()),
        };

        assert_eq!(input.id, "pk-new");
        assert_eq!(input.user_id, "user-123");
        assert_eq!(input.user_label, Some("Test Key".to_string()));
    }

    #[test]
    fn test_non_webauthn_credential() {
        let cred = WebAuthnCredential {
            id: "cred-123".to_string(),
            credential_type: "password".to_string(),
            user_label: None,
            created_at: None,
        };
        assert!(!cred.is_webauthn());
        assert!(!cred.is_passwordless());
    }
}
