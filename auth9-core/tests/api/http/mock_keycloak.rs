//! Mock Keycloak Server for HTTP Handler Tests
//!
//! Provides a wiremock-based mock server that simulates Keycloak's Admin API.
//! This allows testing HTTP handlers that interact with Keycloak without
//! requiring a real Keycloak instance.

use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Mock Keycloak server for testing
pub struct MockKeycloakServer {
    server: MockServer,
}

impl MockKeycloakServer {
    /// Create and start a new mock Keycloak server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let mock = Self { server };
        // Always set up the token endpoint
        mock.mock_token_endpoint().await;
        mock
    }

    /// Get the base URI of the mock server
    pub fn uri(&self) -> String {
        self.server.uri()
    }

    /// Mock the token endpoint (required for all authenticated requests)
    async fn mock_token_endpoint(&self) {
        Mock::given(method("POST"))
            .and(path("/realms/master/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "mock-token",
                "expires_in": 300,
                "token_type": "Bearer"
            })))
            .mount(&self.server)
            .await;
    }

    // ========================================================================
    // OIDC Endpoints for Auth Flow Testing
    // ========================================================================

    /// Mock successful token exchange (authorization code → tokens)
    pub async fn mock_token_exchange_success(
        &self,
        access_token: &str,
        refresh_token: Option<&str>,
        id_token: Option<&str>,
    ) {
        let mut response = json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": 300
        });
        if let Some(rt) = refresh_token {
            response["refresh_token"] = json!(rt);
        }
        if let Some(it) = id_token {
            response["id_token"] = json!(it);
        }

        Mock::given(method("POST"))
            .and(path("/realms/test/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }

    /// Mock failed token exchange
    pub async fn mock_token_exchange_failure(&self, error: &str, error_description: &str) {
        Mock::given(method("POST"))
            .and(path("/realms/test/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": error,
                "error_description": error_description
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock userinfo endpoint
    pub async fn mock_userinfo_endpoint(&self, sub: &str, email: &str, name: Option<&str>) {
        let mut response = json!({
            "sub": sub,
            "email": email
        });
        if let Some(n) = name {
            response["name"] = json!(n);
        }

        Mock::given(method("GET"))
            .and(path("/realms/test/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&self.server)
            .await;
    }

    /// Mock userinfo endpoint failure (invalid token)
    pub async fn mock_userinfo_endpoint_failure(&self) {
        Mock::given(method("GET"))
            .and(path("/realms/test/protocol/openid-connect/userinfo"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_token",
                "error_description": "Token is invalid"
            })))
            .mount(&self.server)
            .await;
    }

    // ========================================================================
    // User Endpoints
    // ========================================================================

    /// Mock successful user creation, returning the given user ID
    pub async fn mock_create_user_success(&self, user_id: &str) {
        Mock::given(method("POST"))
            .and(path("/admin/realms/test/users"))
            .respond_with(ResponseTemplate::new(201).append_header(
                "Location",
                format!("{}/admin/realms/test/users/{}", self.server.uri(), user_id),
            ))
            .mount(&self.server)
            .await;
    }

    /// Mock user creation conflict (user already exists)
    pub async fn mock_create_user_conflict(&self) {
        Mock::given(method("POST"))
            .and(path("/admin/realms/test/users"))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({
                "errorMessage": "User exists with same username"
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock successful user retrieval
    pub async fn mock_get_user_success(&self, user_id: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/admin/realms/test/users/{}", user_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": user_id,
                "username": "testuser",
                "email": "test@example.com",
                "firstName": "Test",
                "lastName": "User",
                "enabled": true,
                "emailVerified": true
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock user not found
    pub async fn mock_get_user_not_found(&self) {
        Mock::given(method("GET"))
            .and(path_regex(r"/admin/realms/test/users/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&self.server)
            .await;
    }

    /// Mock successful user update
    pub async fn mock_update_user_success(&self, user_id: &str) {
        Mock::given(method("PUT"))
            .and(path(format!("/admin/realms/test/users/{}", user_id)))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    /// Mock user update for any user (wildcard)
    pub async fn mock_update_user_any_success(&self) {
        Mock::given(method("PUT"))
            .and(path_regex(r"/admin/realms/test/users/.*"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    /// Mock successful user deletion
    pub async fn mock_delete_user_success(&self, user_id: &str) {
        Mock::given(method("DELETE"))
            .and(path(format!("/admin/realms/test/users/{}", user_id)))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    /// Mock user deletion for any user (wildcard)
    pub async fn mock_delete_user_any_success(&self) {
        Mock::given(method("DELETE"))
            .and(path_regex(r"/admin/realms/test/users/[^/]+$"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    /// Mock user deletion not found
    pub async fn mock_delete_user_not_found(&self) {
        Mock::given(method("DELETE"))
            .and(path_regex(r"/admin/realms/test/users/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&self.server)
            .await;
    }

    // ========================================================================
    // User Credentials Endpoints
    // ========================================================================

    /// Mock list user credentials (for MFA operations)
    pub async fn mock_list_user_credentials(&self, user_id: &str, credentials: Vec<(&str, &str)>) {
        let creds_json: Vec<serde_json::Value> = credentials
            .into_iter()
            .map(|(id, cred_type)| {
                json!({
                    "id": id,
                    "type": cred_type
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path(format!(
                "/admin/realms/test/users/{}/credentials",
                user_id
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(creds_json))
            .mount(&self.server)
            .await;
    }

    /// Mock list user credentials for any user
    pub async fn mock_list_user_credentials_any(&self, credentials: Vec<(&str, &str)>) {
        let creds_json: Vec<serde_json::Value> = credentials
            .into_iter()
            .map(|(id, cred_type)| {
                json!({
                    "id": id,
                    "type": cred_type
                })
            })
            .collect();

        Mock::given(method("GET"))
            .and(path_regex(r"/admin/realms/test/users/[^/]+/credentials$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(creds_json))
            .mount(&self.server)
            .await;
    }

    /// Mock delete user credential (for MFA disable)
    pub async fn mock_delete_user_credential_success(&self) {
        Mock::given(method("DELETE"))
            .and(path_regex(r"/admin/realms/test/users/.*/credentials/.*"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    // ========================================================================
    // OIDC Client Endpoints
    // ========================================================================

    /// Mock successful OIDC client creation
    pub async fn mock_create_oidc_client_success(&self, client_uuid: &str) {
        Mock::given(method("POST"))
            .and(path("/admin/realms/test/clients"))
            .respond_with(ResponseTemplate::new(201).append_header(
                "Location",
                format!(
                    "{}/admin/realms/test/clients/{}",
                    self.server.uri(),
                    client_uuid
                ),
            ))
            .mount(&self.server)
            .await;
    }

    /// Mock OIDC client creation conflict
    pub async fn mock_create_oidc_client_conflict(&self) {
        Mock::given(method("POST"))
            .and(path("/admin/realms/test/clients"))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({
                "errorMessage": "Client already exists"
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock get client secret
    pub async fn mock_get_client_secret(&self, client_uuid: &str, secret: &str) {
        Mock::given(method("GET"))
            .and(path(format!(
                "/admin/realms/test/clients/{}/client-secret",
                client_uuid
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "secret",
                "value": secret
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock get client secret for any client
    pub async fn mock_get_client_secret_any(&self, secret: &str) {
        Mock::given(method("GET"))
            .and(path_regex(r"/admin/realms/test/clients/[^/]+/client-secret$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "secret",
                "value": secret
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock regenerate client secret
    pub async fn mock_regenerate_client_secret(&self, client_uuid: &str, new_secret: &str) {
        Mock::given(method("POST"))
            .and(path(format!(
                "/admin/realms/test/clients/{}/client-secret",
                client_uuid
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "secret",
                "value": new_secret
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock regenerate client secret for any client
    pub async fn mock_regenerate_client_secret_any(&self, new_secret: &str) {
        Mock::given(method("POST"))
            .and(path_regex(r"/admin/realms/test/clients/[^/]+/client-secret$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "secret",
                "value": new_secret
            })))
            .mount(&self.server)
            .await;
    }

    /// Mock get client UUID by client_id
    pub async fn mock_get_client_uuid_by_client_id(&self, client_id: &str, client_uuid: &str) {
        Mock::given(method("GET"))
            .and(path_regex(r"/admin/realms/test/clients.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": client_uuid,
                    "clientId": client_id,
                    "name": "Test Client",
                    "enabled": true,
                    "protocol": "openid-connect",
                    "publicClient": false,
                    "redirectUris": [],
                    "webOrigins": []
                }
            ])))
            .mount(&self.server)
            .await;
    }

    /// Mock get client UUID not found (empty response)
    pub async fn mock_get_client_uuid_not_found(&self) {
        Mock::given(method("GET"))
            .and(path_regex(r"/admin/realms/test/clients\?.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&self.server)
            .await;
    }

    /// Mock successful OIDC client update
    pub async fn mock_update_oidc_client_success(&self) {
        Mock::given(method("PUT"))
            .and(path_regex(r"/admin/realms/test/clients/.*"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    /// Mock successful OIDC client deletion
    pub async fn mock_delete_oidc_client_success(&self) {
        Mock::given(method("DELETE"))
            .and(path_regex(r"/admin/realms/test/clients/.*"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&self.server)
            .await;
    }

    /// Mock OIDC client deletion not found
    pub async fn mock_delete_oidc_client_not_found(&self) {
        Mock::given(method("DELETE"))
            .and(path_regex(r"/admin/realms/test/clients/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&self.server)
            .await;
    }

    // ========================================================================
    // Convenience Methods for Common Test Scenarios
    // ========================================================================

    /// Set up all mocks needed for a typical user creation test
    pub async fn setup_for_user_creation(&self, user_id: &str) {
        self.mock_create_user_success(user_id).await;
    }

    /// Set up all mocks needed for a typical user update test
    pub async fn setup_for_user_update(&self, user_id: &str) {
        self.mock_update_user_success(user_id).await;
    }

    /// Set up all mocks needed for a typical user deletion test
    pub async fn setup_for_user_deletion(&self, user_id: &str) {
        self.mock_delete_user_success(user_id).await;
    }

    /// Set up all mocks needed for MFA enable test
    pub async fn setup_for_mfa_enable(&self, user_id: &str) {
        self.mock_update_user_success(user_id).await;
    }

    /// Set up all mocks needed for MFA disable test
    pub async fn setup_for_mfa_disable(&self, user_id: &str) {
        self.mock_list_user_credentials(user_id, vec![("cred-1", "totp")]).await;
        self.mock_delete_user_credential_success().await;
        self.mock_update_user_success(user_id).await;
    }

    /// Set up all mocks needed for service/client creation test
    pub async fn setup_for_service_creation(&self, client_uuid: &str, client_secret: &str) {
        self.mock_create_oidc_client_success(client_uuid).await;
        self.mock_get_client_secret(client_uuid, client_secret).await;
    }

    /// Set up mocks for service deletion (needs to delete associated Keycloak clients)
    pub async fn setup_for_service_deletion(&self) {
        self.mock_get_client_uuid_not_found().await;
        self.mock_delete_oidc_client_success().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_keycloak_server_creation() {
        let mock = MockKeycloakServer::new().await;
        assert!(mock.uri().starts_with("http://"));
    }

    #[tokio::test]
    async fn test_mock_keycloak_uri() {
        let mock = MockKeycloakServer::new().await;
        let uri = mock.uri();
        assert!(uri.contains("127.0.0.1"));
    }
}
