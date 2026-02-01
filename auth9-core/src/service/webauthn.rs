//! WebAuthn/Passkey service
//!
//! This is a lightweight service that proxies to Keycloak for WebAuthn management.
//! All actual WebAuthn logic is handled by Keycloak.

use crate::domain::WebAuthnCredential;
use crate::error::Result;
use crate::keycloak::KeycloakClient;
use std::sync::Arc;

pub struct WebAuthnService {
    keycloak: Arc<KeycloakClient>,
}

impl WebAuthnService {
    pub fn new(keycloak: Arc<KeycloakClient>) -> Self {
        Self { keycloak }
    }

    /// List WebAuthn credentials for a user
    pub async fn list_credentials(&self, keycloak_user_id: &str) -> Result<Vec<WebAuthnCredential>> {
        let credentials = self.keycloak.list_webauthn_credentials(keycloak_user_id).await?;

        let webauthn_creds: Vec<WebAuthnCredential> = credentials
            .into_iter()
            .map(|c| WebAuthnCredential {
                id: c.id,
                credential_type: c.credential_type,
                user_label: c.user_label,
                created_at: c.created_date.map(|ts| {
                    chrono::DateTime::from_timestamp_millis(ts).unwrap_or_else(chrono::Utc::now)
                }),
            })
            .collect();

        Ok(webauthn_creds)
    }

    /// Delete a WebAuthn credential
    pub async fn delete_credential(
        &self,
        keycloak_user_id: &str,
        credential_id: &str,
    ) -> Result<()> {
        self.keycloak
            .delete_user_credential(keycloak_user_id, credential_id)
            .await
    }

    /// Build the URL to redirect user to Keycloak's WebAuthn registration
    ///
    /// The user should be redirected to this URL to register a new passkey.
    /// After registration, Keycloak will redirect back to the redirect_uri.
    pub fn build_register_url(&self, redirect_uri: &str) -> String {
        // Keycloak handles WebAuthn registration through required actions
        // The client needs to trigger the CONFIGURE_TOTP or WEBAUTHN_REGISTER action
        format!(
            "{}?redirect_uri={}&kc_action=WEBAUTHN_REGISTER",
            std::env::var("KEYCLOAK_ACCOUNT_URL").unwrap_or_else(|_| {
                format!(
                    "{}/realms/{}/account",
                    std::env::var("KEYCLOAK_URL").unwrap_or_else(|_| "http://localhost:8081".to_string()),
                    std::env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "auth9".to_string())
                )
            }),
            urlencoding::encode(redirect_uri)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeycloakConfig;

    #[test]
    fn test_build_register_url() {
        let redirect = "http://localhost:3000/settings/passkeys";
        let encoded = urlencoding::encode(redirect);
        assert!(encoded.contains("localhost"));
        // Should encode special characters
        assert!(encoded.contains("%3A")); // : character
        assert!(encoded.contains("%2F")); // / character
    }

    #[test]
    fn test_build_register_url_with_query_params() {
        let redirect = "http://localhost:3000/settings/passkeys?tab=security&user=123";
        let encoded = urlencoding::encode(redirect);
        assert!(encoded.contains("tab%3Dsecurity"));
        assert!(encoded.contains("user%3D123"));
    }

    #[test]
    fn test_service_new() {
        let keycloak = create_test_keycloak_client();
        let service = WebAuthnService::new(Arc::new(keycloak));

        // Test that service is created successfully
        let url = service.build_register_url("http://localhost:3000/callback");
        assert!(url.contains("redirect_uri"));
        assert!(url.contains("kc_action=WEBAUTHN_REGISTER"));
    }

    #[test]
    fn test_build_register_url_contains_keycloak_action() {
        let keycloak = create_test_keycloak_client();
        let service = WebAuthnService::new(Arc::new(keycloak));

        let url = service.build_register_url("http://localhost:3000/passkeys");

        // URL should contain the WebAuthn register action
        assert!(url.contains("kc_action=WEBAUTHN_REGISTER"));
    }

    #[test]
    fn test_build_register_url_encodes_redirect() {
        let keycloak = create_test_keycloak_client();
        let service = WebAuthnService::new(Arc::new(keycloak));

        let redirect = "http://localhost:3000/settings?tab=security";
        let url = service.build_register_url(redirect);

        // Redirect URI should be URL encoded
        assert!(url.contains("redirect_uri="));
        assert!(!url.contains("?tab=security&")); // Should be encoded
    }

    #[test]
    fn test_webauthn_credential_creation() {
        let cred = WebAuthnCredential {
            id: "cred-123".to_string(),
            credential_type: "webauthn".to_string(),
            user_label: Some("My Passkey".to_string()),
            created_at: Some(chrono::Utc::now()),
        };

        assert_eq!(cred.id, "cred-123");
        assert_eq!(cred.credential_type, "webauthn");
        assert_eq!(cred.user_label, Some("My Passkey".to_string()));
        assert!(cred.created_at.is_some());
    }

    #[test]
    fn test_webauthn_credential_without_label() {
        let cred = WebAuthnCredential {
            id: "cred-456".to_string(),
            credential_type: "webauthn-passwordless".to_string(),
            user_label: None,
            created_at: None,
        };

        assert_eq!(cred.id, "cred-456");
        assert_eq!(cred.credential_type, "webauthn-passwordless");
        assert!(cred.user_label.is_none());
        assert!(cred.created_at.is_none());
    }

    // Helper to create a test KeycloakClient
    fn create_test_keycloak_client() -> KeycloakClient {
        KeycloakClient::new(KeycloakConfig {
            url: "http://localhost:8081".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
        })
    }
}
