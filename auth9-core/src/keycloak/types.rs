//! Keycloak type definitions
//!
//! This module contains all shared type definitions for interacting with
//! the Keycloak Admin API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Keycloak user representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUser {
    pub id: Option<String>,
    pub username: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub email_verified: bool,
    #[serde(default)]
    pub attributes: HashMap<String, Vec<String>>,
}

/// Keycloak user update input
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUserUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_actions: Option<Vec<String>>,
}

/// Input for creating a user in Keycloak
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKeycloakUserInput {
    pub username: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    pub email_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Vec<KeycloakCredential>>,
}

/// Keycloak credential for user creation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakCredential {
    #[serde(rename = "type")]
    pub credential_type: String,
    pub value: String,
    pub temporary: bool,
}

/// Keycloak user credential representation
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakUserCredential {
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: String,
}

/// Keycloak OIDC client representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakOidcClient {
    pub id: Option<String>,
    pub client_id: String,
    pub name: Option<String>,
    pub enabled: bool,
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_url: Option<String>,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub web_origins: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
    pub public_client: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

/// Keycloak session representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakSession {
    pub id: String,
    pub username: Option<String>,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub start: Option<i64>,
    pub last_access: Option<i64>,
    #[serde(default)]
    pub clients: HashMap<String, String>,
}

/// Keycloak identity provider representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakIdentityProvider {
    pub alias: String,
    pub display_name: Option<String>,
    pub provider_id: String,
    pub enabled: bool,
    #[serde(default)]
    pub trust_email: bool,
    #[serde(default)]
    pub store_token: bool,
    #[serde(default)]
    pub link_only: bool,
    pub first_broker_login_flow_alias: Option<String>,
    #[serde(default)]
    pub config: HashMap<String, String>,
}

/// Keycloak federated identity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakFederatedIdentity {
    pub identity_provider: String,
    pub user_id: String,
    pub user_name: Option<String>,
}

/// Keycloak credential representation for WebAuthn
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakCredentialRepresentation {
    pub id: String,
    #[serde(rename = "type")]
    pub credential_type: String,
    pub user_label: Option<String>,
    pub created_date: Option<i64>,
}

/// Keycloak realm representation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeycloakRealm {
    pub realm: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_allowed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_password_allowed: Option<bool>,
    /// SSL requirement: "none", "external", or "all"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_required: Option<String>,
    /// Login theme name (e.g., "auth9", "keycloak")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_theme: Option<String>,
}

/// Realm update parameters
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RealmUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_allowed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_password_allowed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_required: Option<String>,
    /// Login theme name (e.g., "auth9", "keycloak")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_theme: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keycloak_user_serialization() {
        let user = KeycloakUser {
            id: Some("123".to_string()),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            enabled: true,
            email_verified: true,
            attributes: HashMap::new(),
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));
    }

    #[test]
    fn test_create_user_input_serialization() {
        let input = CreateKeycloakUserInput {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            first_name: Some("New".to_string()),
            last_name: Some("User".to_string()),
            enabled: true,
            email_verified: false,
            credentials: Some(vec![KeycloakCredential {
                credential_type: "password".to_string(),
                value: "secret123".to_string(),
                temporary: true,
            }]),
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("newuser"));
        assert!(json.contains("password"));
    }

    #[test]
    fn test_keycloak_user_deserialization() {
        let json = r#"{
            "id": "abc-123",
            "username": "john",
            "email": "john@example.com",
            "firstName": "John",
            "lastName": "Doe",
            "enabled": true,
            "emailVerified": true,
            "attributes": {}
        }"#;

        let user: KeycloakUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, Some("abc-123".to_string()));
        assert_eq!(user.username, "john");
        assert_eq!(user.email, Some("john@example.com".to_string()));
        assert_eq!(user.first_name, Some("John".to_string()));
        assert_eq!(user.last_name, Some("Doe".to_string()));
        assert!(user.enabled);
        assert!(user.email_verified);
    }

    #[test]
    fn test_keycloak_user_deserialization_minimal() {
        let json = r#"{
            "username": "minimal",
            "enabled": false
        }"#;

        let user: KeycloakUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.username, "minimal");
        assert!(user.id.is_none());
        assert!(user.email.is_none());
        assert!(!user.enabled);
        assert!(!user.email_verified); // default
    }

    #[test]
    fn test_keycloak_user_update_serialization_partial() {
        let update = KeycloakUserUpdate {
            username: None,
            email: Some("newemail@example.com".to_string()),
            first_name: None,
            last_name: None,
            enabled: None,
            email_verified: None,
            required_actions: None,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("newemail@example.com"));
        assert!(!json.contains("username"));
        assert!(!json.contains("firstName"));
    }

    #[test]
    fn test_keycloak_user_update_with_required_actions() {
        let update = KeycloakUserUpdate {
            username: None,
            email: None,
            first_name: None,
            last_name: None,
            enabled: None,
            email_verified: None,
            required_actions: Some(vec![
                "CONFIGURE_TOTP".to_string(),
                "UPDATE_PASSWORD".to_string(),
            ]),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("requiredActions"));
        assert!(json.contains("CONFIGURE_TOTP"));
        assert!(json.contains("UPDATE_PASSWORD"));
    }

    #[test]
    fn test_keycloak_credential_serialization() {
        let cred = KeycloakCredential {
            credential_type: "password".to_string(),
            value: "secret123".to_string(),
            temporary: false,
        };

        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains("\"type\":\"password\""));
        assert!(json.contains("\"value\":\"secret123\""));
        assert!(json.contains("\"temporary\":false"));
    }

    #[test]
    fn test_keycloak_user_credential_deserialization() {
        let json = r#"{
            "id": "cred-123",
            "type": "otp"
        }"#;

        let cred: KeycloakUserCredential = serde_json::from_str(json).unwrap();
        assert_eq!(cred.id, "cred-123");
        assert_eq!(cred.credential_type, "otp");
    }

    #[test]
    fn test_keycloak_oidc_client_serialization() {
        let client = KeycloakOidcClient {
            id: Some("client-uuid".to_string()),
            client_id: "my-app".to_string(),
            name: Some("My Application".to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: Some("https://myapp.example.com".to_string()),
            root_url: None,
            admin_url: None,
            redirect_uris: vec!["https://myapp.example.com/callback".to_string()],
            web_origins: vec!["https://myapp.example.com".to_string()],
            attributes: None,
            public_client: false,
            secret: Some("client-secret".to_string()),
        };

        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("\"clientId\":\"my-app\""));
        assert!(json.contains("\"protocol\":\"openid-connect\""));
        assert!(json.contains("\"redirectUris\""));
    }

    #[test]
    fn test_keycloak_oidc_client_deserialization() {
        let json = r#"{
            "id": "uuid-123",
            "clientId": "test-client",
            "name": "Test Client",
            "enabled": true,
            "protocol": "openid-connect",
            "redirectUris": ["http://localhost:3000/callback"],
            "webOrigins": ["http://localhost:3000"],
            "publicClient": true
        }"#;

        let client: KeycloakOidcClient = serde_json::from_str(json).unwrap();
        assert_eq!(client.id, Some("uuid-123".to_string()));
        assert_eq!(client.client_id, "test-client");
        assert_eq!(client.protocol, "openid-connect");
        assert!(client.public_client);
        assert_eq!(client.redirect_uris.len(), 1);
    }

    #[test]
    fn test_keycloak_oidc_client_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "post.logout.redirect.uris".to_string(),
            "https://app.com/logout".to_string(),
        );

        let client = KeycloakOidcClient {
            id: None,
            client_id: "attr-client".to_string(),
            name: None,
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: None,
            root_url: None,
            admin_url: None,
            redirect_uris: vec![],
            web_origins: vec![],
            attributes: Some(attrs),
            public_client: false,
            secret: None,
        };

        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("post.logout.redirect.uris"));
        assert!(json.contains("https://app.com/logout"));
    }

    #[test]
    fn test_keycloak_user_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("department".to_string(), vec!["Engineering".to_string()]);
        attrs.insert("phone".to_string(), vec!["+1234567890".to_string()]);

        let user = KeycloakUser {
            id: Some("user-123".to_string()),
            username: "attruser".to_string(),
            email: Some("attr@example.com".to_string()),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            attributes: attrs,
        };

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("department"));
        assert!(json.contains("Engineering"));
    }

    #[test]
    fn test_create_user_input_without_credentials() {
        let input = CreateKeycloakUserInput {
            username: "nopassword".to_string(),
            email: "nopass@example.com".to_string(),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            credentials: None,
        };

        let json = serde_json::to_string(&input).unwrap();
        assert!(!json.contains("credentials"));
    }

    #[test]
    fn test_keycloak_user_clone() {
        let user = KeycloakUser {
            id: Some("123".to_string()),
            username: "cloneuser".to_string(),
            email: Some("clone@example.com".to_string()),
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: true,
            attributes: HashMap::new(),
        };

        let cloned = user.clone();
        assert_eq!(user.id, cloned.id);
        assert_eq!(user.username, cloned.username);
    }

    #[test]
    fn test_keycloak_oidc_client_clone() {
        let client = KeycloakOidcClient {
            id: Some("id".to_string()),
            client_id: "client".to_string(),
            name: None,
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: None,
            root_url: None,
            admin_url: None,
            redirect_uris: vec!["http://localhost".to_string()],
            web_origins: vec![],
            attributes: None,
            public_client: false,
            secret: None,
        };

        let cloned = client.clone();
        assert_eq!(client.client_id, cloned.client_id);
        assert_eq!(client.redirect_uris, cloned.redirect_uris);
    }

    #[test]
    fn test_keycloak_user_debug() {
        let user = KeycloakUser {
            id: Some("debug-123".to_string()),
            username: "debuguser".to_string(),
            email: None,
            first_name: None,
            last_name: None,
            enabled: true,
            email_verified: false,
            attributes: HashMap::new(),
        };

        let debug_str = format!("{:?}", user);
        assert!(debug_str.contains("KeycloakUser"));
        assert!(debug_str.contains("debuguser"));
    }

    #[test]
    fn test_keycloak_user_update_debug() {
        let update = KeycloakUserUpdate {
            username: Some("newname".to_string()),
            email: None,
            first_name: None,
            last_name: None,
            enabled: Some(true),
            email_verified: None,
            required_actions: None,
        };

        let debug_str = format!("{:?}", update);
        assert!(debug_str.contains("KeycloakUserUpdate"));
        assert!(debug_str.contains("newname"));
    }

    #[test]
    fn test_realm_update_serialization() {
        let update = RealmUpdate {
            registration_allowed: Some(true),
            reset_password_allowed: Some(false),
            ssl_required: None,
            login_theme: None,
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("registrationAllowed"));
        assert!(json.contains("resetPasswordAllowed"));
        assert!(!json.contains("sslRequired"));
    }
}
