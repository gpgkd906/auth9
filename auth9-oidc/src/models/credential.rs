use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Credential type discriminator — neutral naming, no Keycloak semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    Password,
    Totp,
    RecoveryCode,
    WebAuthn,
}

impl CredentialType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Password => "password", // pragma: allowlist secret
            Self::Totp => "totp",
            Self::RecoveryCode => "recovery_code",
            Self::WebAuthn => "webauthn",
        }
    }

    pub fn from_str_value(s: &str) -> Option<Self> {
        match s {
            "password" => Some(Self::Password),
            "totp" => Some(Self::Totp),
            "recovery_code" => Some(Self::RecoveryCode),
            "webauthn" => Some(Self::WebAuthn),
            _ => None,
        }
    }
}

impl fmt::Display for CredentialType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stored credential row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub user_id: String,
    pub credential_type: CredentialType,
    pub credential_data: serde_json::Value,
    pub user_label: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a new credential.
#[derive(Debug, Clone)]
pub struct CreateCredentialInput {
    pub user_id: String,
    pub credential_type: CredentialType,
    pub credential_data: serde_json::Value,
    pub user_label: Option<String>,
}

// --- Typed credential payloads ---

/// Password credential data stored in `credential_data` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordCredentialData {
    pub hash: String,
    pub algorithm: String,
    pub temporary: bool,
}

/// TOTP credential data stored in `credential_data` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpCredentialData {
    pub secret_encrypted: String,
    pub algorithm: String,
    pub digits: u8,
    pub period: u32,
}

/// Recovery code data stored in `credential_data` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCodeData {
    pub code_hash: String,
    pub used: bool,
}

/// WebAuthn credential data stored in `credential_data` JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredentialData {
    pub credential_id: String,
    pub public_key: String,
    pub aaguid: Option<String>,
    pub sign_count: u32,
}

impl Credential {
    /// Parse `credential_data` into a typed payload.
    pub fn parse_password_data(&self) -> Result<PasswordCredentialData, serde_json::Error> {
        serde_json::from_value(self.credential_data.clone())
    }

    pub fn parse_totp_data(&self) -> Result<TotpCredentialData, serde_json::Error> {
        serde_json::from_value(self.credential_data.clone())
    }

    pub fn parse_recovery_code_data(&self) -> Result<RecoveryCodeData, serde_json::Error> {
        serde_json::from_value(self.credential_data.clone())
    }

    pub fn parse_webauthn_data(&self) -> Result<WebAuthnCredentialData, serde_json::Error> {
        serde_json::from_value(self.credential_data.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_type_roundtrip() {
        for ct in [
            CredentialType::Password,
            CredentialType::Totp,
            CredentialType::RecoveryCode,
            CredentialType::WebAuthn,
        ] {
            let s = ct.as_str();
            assert_eq!(CredentialType::from_str_value(s), Some(ct));
            assert_eq!(ct.to_string(), s);
        }
    }

    #[test]
    fn credential_type_unknown_returns_none() {
        assert_eq!(CredentialType::from_str_value("unknown"), None);
    }

    #[test]
    fn password_credential_data_serde() {
        let data = PasswordCredentialData {
            hash: "argon2id$v19$m=65536$hash".to_string(),
            algorithm: "argon2id".to_string(),
            temporary: false,
        };
        let json = serde_json::to_value(&data).unwrap();
        let parsed: PasswordCredentialData = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.hash, data.hash);
        assert_eq!(parsed.algorithm, "argon2id");
        assert!(!parsed.temporary);
    }

    #[test]
    fn totp_credential_data_serde() {
        let data = TotpCredentialData {
            secret_encrypted: "encrypted-secret".to_string(), // pragma: allowlist secret
            algorithm: "SHA1".to_string(),
            digits: 6,
            period: 30,
        };
        let json = serde_json::to_value(&data).unwrap();
        let parsed: TotpCredentialData = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.digits, 6);
        assert_eq!(parsed.period, 30);
    }

    #[test]
    fn recovery_code_data_serde() {
        let data = RecoveryCodeData {
            code_hash: "sha256$abcdef".to_string(),
            used: false,
        };
        let json = serde_json::to_value(&data).unwrap();
        let parsed: RecoveryCodeData = serde_json::from_value(json).unwrap();
        assert!(!parsed.used);
    }

    #[test]
    fn webauthn_credential_data_serde() {
        let data = WebAuthnCredentialData {
            credential_id: "cred-base64url".to_string(),
            public_key: "pk-base64url".to_string(),
            aaguid: Some("aaguid-123".to_string()),
            sign_count: 0,
        };
        let json = serde_json::to_value(&data).unwrap();
        let parsed: WebAuthnCredentialData = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.sign_count, 0);
        assert_eq!(parsed.aaguid, Some("aaguid-123".to_string()));
    }

    #[test]
    fn credential_parse_typed_data() {
        let cred = Credential {
            id: "cred-1".to_string(),
            user_id: "user-1".to_string(),
            credential_type: CredentialType::Password,
            credential_data: serde_json::json!({
                "hash": "argon2id$hash",
                "algorithm": "argon2id",
                "temporary": false
            }),
            user_label: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let pwd = cred.parse_password_data().unwrap();
        assert_eq!(pwd.algorithm, "argon2id");
        assert!(!pwd.temporary);
    }
}
