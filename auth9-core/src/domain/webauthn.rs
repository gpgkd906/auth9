//! WebAuthn/Passkey domain models
//!
//! Note: WebAuthn credentials are stored in Keycloak.
//! These models are for displaying credential info in Auth9 Portal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// WebAuthn credential info from Keycloak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredential {
    pub id: String,
    pub credential_type: String,
    pub user_label: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
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
