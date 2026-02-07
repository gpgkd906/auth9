//! Keycloak Seeder for initialization
//!
//! This module provides functionality to initialize and seed default data
//! in Keycloak, including realms, admin users, and clients.

use anyhow::Context;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::env;
use tracing::{info, warn};

use crate::config::KeycloakConfig;

use super::types::*;

/// Default admin user configuration
const DEFAULT_ADMIN_USERNAME: &str = "admin";
const DEFAULT_ADMIN_EMAIL: &str = "admin@auth9.local";
const DEFAULT_ADMIN_FIRST_NAME: &str = "Admin";
const DEFAULT_ADMIN_LAST_NAME: &str = "User";

/// Default portal client configuration
const DEFAULT_PORTAL_CLIENT_ID: &str = "auth9-portal";
const DEFAULT_PORTAL_CLIENT_NAME: &str = "Auth9 Admin Portal";

/// Default admin client configuration
const DEFAULT_ADMIN_CLIENT_ID: &str = "auth9-admin";
const DEFAULT_ADMIN_CLIENT_NAME: &str = "Auth9 Admin Client";

/// Get admin password from env var or generate a secure random one
fn get_admin_password() -> String {
    // Allow override via environment variable (useful for local development)
    if let Ok(password) = env::var("AUTH9_ADMIN_PASSWORD") {
        if !password.is_empty() {
            return password;
        }
    }
    generate_secure_password()
}

/// Generate a cryptographically secure random password
fn generate_secure_password() -> String {
    use rand::Rng;
    const CHARSET_LOWER: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
    const CHARSET_UPPER: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    const CHARSET_DIGIT: &[u8] = b"23456789";
    const CHARSET_SPECIAL: &[u8] = b"!@#$%^&*";

    let mut rng = rand::thread_rng();

    // Ensure at least one of each type
    let mut password: Vec<char> = Vec::with_capacity(16);
    password.push(CHARSET_LOWER[rng.gen_range(0..CHARSET_LOWER.len())] as char);
    password.push(CHARSET_UPPER[rng.gen_range(0..CHARSET_UPPER.len())] as char);
    password.push(CHARSET_DIGIT[rng.gen_range(0..CHARSET_DIGIT.len())] as char);
    password.push(CHARSET_SPECIAL[rng.gen_range(0..CHARSET_SPECIAL.len())] as char);

    // Fill remaining with mixed charset
    let all_chars: Vec<u8> =
        [CHARSET_LOWER, CHARSET_UPPER, CHARSET_DIGIT, CHARSET_SPECIAL].concat();
    for _ in 0..12 {
        password.push(all_chars[rng.gen_range(0..all_chars.len())] as char);
    }

    // Shuffle the password
    use rand::seq::SliceRandom;
    password.shuffle(&mut rng);

    password.into_iter().collect()
}

/// Build comprehensive redirect URIs list combining localhost and production URLs
fn build_redirect_uris(core_public_url: Option<&str>, portal_url: Option<&str>) -> Vec<String> {
    let mut uris = Vec::new();

    // Always include localhost URLs for local development
    uris.extend([
        "http://localhost:8080/api/v1/auth/callback".to_string(),
        "http://127.0.0.1:8080/api/v1/auth/callback".to_string(),
        "http://localhost:3000/*".to_string(),
        "http://127.0.0.1:3000/*".to_string(),
    ]);

    // Add production URLs if configured
    if let Some(core_url) = core_public_url {
        uris.push(format!("{}/api/v1/auth/callback", core_url));
    }
    if let Some(portal_url_str) = portal_url {
        uris.push(format!("{}/*", portal_url_str));
    }

    uris
}

/// Build comprehensive web origins list
fn build_web_origins(core_public_url: Option<&str>, portal_url: Option<&str>) -> Vec<String> {
    let mut origins = Vec::new();

    // Always include localhost
    origins.extend([
        "http://localhost:8080".to_string(),
        "http://127.0.0.1:8080".to_string(),
        "http://localhost:3000".to_string(),
        "http://127.0.0.1:3000".to_string(),
    ]);

    // Add production URLs
    if let Some(core_url) = core_public_url {
        origins.push(core_url.to_string());
    }
    if let Some(portal_url_str) = portal_url {
        origins.push(portal_url_str.to_string());
    }

    origins
}

/// Keycloak Seeder for initialization
pub struct KeycloakSeeder {
    config: KeycloakConfig,
    http_client: Client,
}

impl KeycloakSeeder {
    /// Create a new Keycloak seeder
    pub fn new(config: &KeycloakConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config: config.clone(),
            http_client,
        }
    }

    /// Get admin token using master realm password grant
    /// Uses KEYCLOAK_ADMIN and KEYCLOAK_ADMIN_PASSWORD environment variables
    /// Uses admin-cli client (default public client in master realm)
    async fn get_master_admin_token(&self) -> anyhow::Result<String> {
        let admin_username = env::var("KEYCLOAK_ADMIN").unwrap_or_else(|_| "admin".to_string());
        let admin_password =
            env::var("KEYCLOAK_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".to_string());

        let token_url = format!(
            "{}/realms/master/protocol/openid-connect/token",
            self.config.url
        );

        // Use admin-cli (Keycloak's default public client in master realm)
        // This is necessary during initialization when auth9-admin client doesn't exist yet
        let admin_cli = "admin-cli";

        let params = vec![
            ("grant_type", "password"),
            ("client_id", admin_cli),
            ("username", &admin_username),
            ("password", &admin_password),
        ];

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .context("Failed to connect to Keycloak")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to get admin token from Keycloak: {} - {}",
                status,
                body
            );
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token_response.access_token)
    }

    /// Check if a realm exists
    async fn realm_exists(&self, token: &str) -> anyhow::Result<bool> {
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to check realm")?;

        Ok(response.status().is_success())
    }

    /// Create a new realm
    async fn create_realm(&self, token: &str) -> anyhow::Result<()> {
        let url = format!("{}/admin/realms", self.config.url);

        let realm = KeycloakRealm {
            realm: self.config.realm.clone(),
            enabled: true,
            display_name: Some("Auth9".to_string()),
            // Default to false - Auth9 controls this via BrandingConfig
            registration_allowed: Some(false),
            reset_password_allowed: Some(true),
            ssl_required: Some(self.config.ssl_required.clone()),
            // Use auth9 custom login theme
            login_theme: Some("auth9".to_string()),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&realm)
            .send()
            .await
            .context("Failed to create realm")?;

        if response.status() == StatusCode::CONFLICT {
            info!("Realm '{}' already exists", self.config.realm);
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create realm: {} - {}", status, body);
        }

        info!("Created realm '{}'", self.config.realm);
        Ok(())
    }

    /// Update realm settings (SSL, login theme, registration, and events) based on configuration
    ///
    /// Note: registrationAllowed is set to false because Auth9 controls this
    /// via BrandingConfig.allow_registration, which syncs to Keycloak when updated.
    async fn update_realm_settings(&self, token: &str) -> anyhow::Result<()> {
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        // Build events listeners list
        // Always include jboss-logging for server logs
        // Add ext-event-http if webhook secret is configured (p2-inc/keycloak-events HTTP forwarder)
        let mut events_listeners = vec!["jboss-logging"];
        if self.config.webhook_secret.is_some() {
            events_listeners.push("ext-event-http");
        }

        let update = serde_json::json!({
            "sslRequired": self.config.ssl_required,
            "loginTheme": "auth9",
            // Default to false - Auth9 controls this via BrandingConfig
            "registrationAllowed": false,
            // Enable event storage and listeners for login event tracking
            "eventsEnabled": true,
            "eventsListeners": events_listeners,
            // Track login-related events for security monitoring
            "enabledEventTypes": [
                "LOGIN",
                "LOGIN_ERROR",
                "LOGOUT",
                "LOGOUT_ERROR",
                "CODE_TO_TOKEN",
                "CODE_TO_TOKEN_ERROR",
                "REFRESH_TOKEN",
                "REFRESH_TOKEN_ERROR",
                "IDENTITY_PROVIDER_LOGIN",
                "IDENTITY_PROVIDER_LOGIN_ERROR",
                "USER_DISABLED_BY_PERMANENT_LOCKOUT",
                "USER_DISABLED_BY_TEMPORARY_LOCKOUT"
            ],
            // Retain events for 30 days (for troubleshooting)
            "eventsExpiration": 2592000
        });

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(token)
            .json(&update)
            .send()
            .await
            .context("Failed to update realm settings")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update realm settings: {} - {}", status, body);
        }

        info!(
            "Updated realm '{}': SSL='{}', loginTheme='auth9', registrationAllowed=false, eventsEnabled=true, eventsListeners={:?}",
            self.config.realm, self.config.ssl_required, events_listeners
        );

        // Configure ext-event-http provider via realm attributes
        // p2-inc/keycloak-events reads config from realm attribute: _providerConfig.ext-event-http
        if let Some(ref webhook_secret) = self.config.webhook_secret {
            self.configure_event_http_provider(token, webhook_secret)
                .await?;
        }

        // IMPORTANT: configure_realm_security must be called LAST because it uses GET-merge-PUT
        // to apply brute force protection and password policy. Earlier partial PUTs (events config,
        // ext-event-http attributes) can reset boolean fields like bruteForceProtected to false
        // when those fields are omitted from the partial JSON payload.
        self.configure_realm_security(token).await?;

        Ok(())
    }

    /// Configure brute force protection and password policy via GET-merge-PUT
    async fn configure_realm_security(&self, token: &str) -> anyhow::Result<()> {
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        // GET current realm representation
        let current: serde_json::Value = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to get realm for security config")?
            .json()
            .await
            .context("Failed to parse realm JSON")?;

        // Merge security settings into full representation
        let mut updated = current;
        if let Some(obj) = updated.as_object_mut() {
            obj.insert(
                "bruteForceProtected".to_string(),
                serde_json::json!(true),
            );
            obj.insert("failureFactor".to_string(), serde_json::json!(5));
            obj.insert("maxDeltaTimeSeconds".to_string(), serde_json::json!(600));
            obj.insert("waitIncrementSeconds".to_string(), serde_json::json!(60));
            obj.insert(
                "maxFailureWaitSeconds".to_string(),
                serde_json::json!(900),
            );
            obj.insert(
                "minimumQuickLoginWaitSeconds".to_string(),
                serde_json::json!(60),
            );
            obj.insert(
                "quickLoginCheckMilliSeconds".to_string(),
                serde_json::json!(1000),
            );
            obj.insert(
                "passwordPolicy".to_string(),
                serde_json::json!(
                    "length(8) and upperCase(1) and lowerCase(1) and digits(1) and notUsername()"
                ),
            );
        }

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(token)
            .json(&updated)
            .send()
            .await
            .context("Failed to update realm security settings")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to update realm security settings: {} - {}",
                status,
                body
            );
        }

        info!(
            "Configured realm '{}' security: bruteForceProtected=true, failureFactor=5, passwordPolicy=enabled",
            self.config.realm
        );

        Ok(())
    }

    /// Configure ext-event-http provider via realm attributes
    ///
    /// The p2-inc/keycloak-events plugin reads its configuration from realm attributes,
    /// not from Keycloak's standard SPI config. The attribute key is:
    /// `_providerConfig.ext-event-http` with a JSON value containing targetUri and sharedSecret.
    async fn configure_event_http_provider(
        &self,
        token: &str,
        webhook_secret: &str,
    ) -> anyhow::Result<()> {
        let url = format!("{}/admin/realms/{}", self.config.url, self.config.realm);

        // Build the webhook target URL - use the internal Docker network URL
        // auth9-core listens on port 8080 inside the network
        let target_uri = std::env::var("KC_SPI_EVENTS_LISTENER_EXT_EVENT_HTTP_TARGET_URI")
            .unwrap_or_else(|_| "http://auth9-core:8080/api/v1/keycloak/events".to_string());

        let provider_config = serde_json::json!({
            "targetUri": target_uri,
            "sharedSecret": webhook_secret,
            "hmacAlgorithm": "HmacSHA256"
        });

        let update = serde_json::json!({
            "attributes": {
                "_providerConfig.ext-event-http": provider_config.to_string()
            }
        });

        let response = self
            .http_client
            .put(&url)
            .bearer_auth(token)
            .json(&update)
            .send()
            .await
            .context("Failed to configure ext-event-http provider")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!("Failed to configure ext-event-http: {} - {}", status, body);
        } else {
            info!(
                "Configured ext-event-http provider: targetUri={}",
                target_uri
            );
        }

        Ok(())
    }

    /// Ensure realm exists (create if not) and configure settings
    pub async fn ensure_realm_exists(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        if self.realm_exists(&token).await? {
            info!("Realm '{}' already exists", self.config.realm);
        } else {
            self.create_realm(&token).await?;
        }

        // Always update settings (SSL, login theme, event listeners) for both new and existing realms
        self.update_realm_settings(&token).await?;
        Ok(())
    }

    /// Check if admin user exists by email
    async fn admin_user_exists(&self, token: &str) -> anyhow::Result<bool> {
        let url = format!(
            "{}/admin/realms/{}/users?username={}&exact=true",
            self.config.url, self.config.realm, DEFAULT_ADMIN_USERNAME
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to check admin user")?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let users: Vec<serde_json::Value> = response.json().await.unwrap_or_default();

        Ok(!users.is_empty())
    }

    /// Create default admin user with a randomly generated password
    /// Returns the generated password if user was created
    async fn create_admin_user(&self, token: &str) -> anyhow::Result<Option<String>> {
        let url = format!(
            "{}/admin/realms/{}/users",
            self.config.url, self.config.realm
        );

        let password = get_admin_password();

        let user = CreateKeycloakUserInput {
            username: DEFAULT_ADMIN_USERNAME.to_string(),
            email: DEFAULT_ADMIN_EMAIL.to_string(),
            first_name: Some(DEFAULT_ADMIN_FIRST_NAME.to_string()),
            last_name: Some(DEFAULT_ADMIN_LAST_NAME.to_string()),
            enabled: true,
            email_verified: true,
            credentials: Some(vec![KeycloakCredential {
                credential_type: "password".to_string(),
                value: password.clone(),
                temporary: false,
            }]),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&user)
            .send()
            .await
            .context("Failed to create admin user")?;

        if response.status() == StatusCode::CONFLICT {
            info!("Admin user '{}' already exists", DEFAULT_ADMIN_USERNAME);
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create admin user: {} - {}", status, body);
        }

        info!("Created admin user '{}'", DEFAULT_ADMIN_USERNAME);
        Ok(Some(password))
    }

    /// Seed default admin user (idempotent)
    pub async fn seed_admin_user(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        if self.admin_user_exists(&token).await? {
            info!(
                "Admin user '{}' already exists, skipping",
                DEFAULT_ADMIN_USERNAME
            );
            return Ok(());
        }

        if let Some(password) = self.create_admin_user(&token).await? {
            // Print credentials in machine-parseable format for deploy script
            println!("========================================");
            println!("AUTH9_ADMIN_USERNAME={}", DEFAULT_ADMIN_USERNAME);
            println!("AUTH9_ADMIN_PASSWORD={}", password);
            println!("========================================");

            warn!(
                "Created admin user '{}' with generated password",
                DEFAULT_ADMIN_USERNAME
            );
            warn!("Please save the admin credentials shown above!");
        }

        Ok(())
    }

    /// Check if portal client exists
    async fn portal_client_exists(&self, token: &str) -> anyhow::Result<bool> {
        let url = format!(
            "{}/admin/realms/{}/clients?clientId={}",
            self.config.url, self.config.realm, DEFAULT_PORTAL_CLIENT_ID
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to check portal client")?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let clients: Vec<serde_json::Value> = response.json().await.unwrap_or_default();

        Ok(!clients.is_empty())
    }

    /// Create portal client
    async fn create_portal_client(&self, token: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/admin/realms/{}/clients",
            self.config.url, self.config.realm
        );

        let client = KeycloakOidcClient {
            id: None,
            client_id: DEFAULT_PORTAL_CLIENT_ID.to_string(),
            name: Some(DEFAULT_PORTAL_CLIENT_NAME.to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            root_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            admin_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            redirect_uris: build_redirect_uris(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            web_origins: build_web_origins(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            attributes: None,
            public_client: false, // Confidential client - Keycloak will generate a secret
            secret: None,
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&client)
            .send()
            .await
            .context("Failed to create portal client")?;

        if response.status() == StatusCode::CONFLICT {
            info!(
                "Portal client '{}' already exists",
                DEFAULT_PORTAL_CLIENT_ID
            );
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create portal client: {} - {}", status, body);
        }

        info!("Created portal client '{}'", DEFAULT_PORTAL_CLIENT_ID);
        Ok(())
    }

    /// Update existing portal client configuration
    async fn update_portal_client(&self, token: &str) -> anyhow::Result<()> {
        // 1. Query existing client to get UUID
        let url = format!(
            "{}/admin/realms/{}/clients?clientId={}",
            self.config.url, self.config.realm, DEFAULT_PORTAL_CLIENT_ID
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to query portal client")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to query portal client for update");
        }

        let clients: Vec<KeycloakOidcClient> = response.json().await?;
        let existing_client = clients
            .into_iter()
            .next()
            .context("Portal client not found for update")?;

        let client_uuid = existing_client.id.context("Portal client UUID missing")?;

        // 2. Build updated client configuration
        let updated_client = KeycloakOidcClient {
            id: Some(client_uuid.clone()),
            client_id: DEFAULT_PORTAL_CLIENT_ID.to_string(),
            name: Some(DEFAULT_PORTAL_CLIENT_NAME.to_string()),
            enabled: true,
            protocol: "openid-connect".to_string(),
            base_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            root_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            admin_url: Some(
                self.config
                    .portal_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
                    .to_string(),
            ),
            redirect_uris: build_redirect_uris(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            web_origins: build_web_origins(
                self.config.core_public_url.as_deref(),
                self.config.portal_url.as_deref(),
            ),
            attributes: existing_client.attributes,
            public_client: false,
            secret: existing_client.secret,
        };

        // 3. Update client
        let update_url = format!(
            "{}/admin/realms/{}/clients/{}",
            self.config.url, self.config.realm, client_uuid
        );

        let response = self
            .http_client
            .put(&update_url)
            .bearer_auth(token)
            .json(&updated_client)
            .send()
            .await
            .context("Failed to update portal client")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update portal client: {} - {}", status, body);
        }

        info!(
            "Updated portal client '{}' configuration",
            DEFAULT_PORTAL_CLIENT_ID
        );
        Ok(())
    }

    /// Seed portal client (idempotent - creates or updates)
    pub async fn seed_portal_client(&self) -> anyhow::Result<()> {
        let token = self.get_master_admin_token().await?;

        // Log configuration being used
        match (&self.config.core_public_url, &self.config.portal_url) {
            (Some(core), Some(portal)) => {
                info!(
                    "Configuring portal client with production URLs: core={}, portal={}",
                    core, portal
                );
            }
            _ => {
                warn!(
                    "Production URLs not configured (AUTH9_CORE_PUBLIC_URL, AUTH9_PORTAL_URL). \
                    Using localhost URLs only. This is OK for local development but not for production."
                );
            }
        }

        if self.portal_client_exists(&token).await? {
            info!(
                "Portal client '{}' already exists, updating configuration...",
                DEFAULT_PORTAL_CLIENT_ID
            );

            // Update existing client to ensure production URLs are configured
            self.update_portal_client(&token).await?;

            return Ok(());
        }

        // Create new client
        self.create_portal_client(&token).await?;
        Ok(())
    }

    /// Check if admin client exists in a specific realm
    async fn admin_client_exists_in_realm(&self, token: &str, realm: &str) -> anyhow::Result<bool> {
        let client_id = DEFAULT_ADMIN_CLIENT_ID;
        let url = format!("{}/admin/realms/{}/clients", self.config.url, realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .query(&[("clientId", client_id)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let clients: Vec<serde_json::Value> = response.json().await?;
        Ok(!clients.is_empty())
    }

    /// Create auth9-admin client in a specific realm
    async fn create_admin_client_in_realm(&self, token: &str, realm: &str) -> anyhow::Result<()> {
        info!(
            "Creating auth9-admin client in realm '{}' (Confidential Client)",
            realm
        );

        // Read secret from environment
        let client_secret =
            env::var("KEYCLOAK_ADMIN_CLIENT_SECRET").unwrap_or_else(|_| String::new());

        // Build client configuration
        let mut client = serde_json::json!({
            "clientId": DEFAULT_ADMIN_CLIENT_ID,
            "name": DEFAULT_ADMIN_CLIENT_NAME,
            "enabled": true,
            "protocol": "openid-connect",
            "publicClient": false,  // Confidential client
            "serviceAccountsEnabled": false,  // Use password grant, not client credentials
            "standardFlowEnabled": false,
            "directAccessGrantsEnabled": true,  // Enable password grant
            "redirectUris": [],
            "webOrigins": [],
        });

        // If secret is provided, set it explicitly
        if !client_secret.is_empty() {
            client["secret"] = serde_json::json!(client_secret);
        }

        let url = format!("{}/admin/realms/{}/clients", self.config.url, realm);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .json(&client)
            .send()
            .await?;

        // Handle conflict (client already exists) as success for idempotency
        if response.status() == StatusCode::CONFLICT {
            info!("auth9-admin client already exists in realm '{}'", realm);
            return Ok(());
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!(
                "Failed to create auth9-admin client in realm '{}': {} - {}",
                realm,
                status,
                body
            );
        }

        // If no secret was provided, retrieve the auto-generated one
        if client_secret.is_empty() {
            info!("Retrieving auto-generated client secret...");
            match self.get_admin_client_secret_in_realm(token, realm).await {
                Ok(secret) => {
                    info!("auth9-admin client created in realm '{}'!", realm);
                    info!("Copy this secret to your secrets configuration:");
                    info!("   KEYCLOAK_ADMIN_CLIENT_SECRET={}", secret);
                }
                Err(e) => {
                    warn!("Client created but failed to retrieve secret: {}", e);
                    info!("You can retrieve it manually from Keycloak Admin Console:");
                    info!("  Clients -> auth9-admin -> Credentials tab");
                }
            }
        } else {
            info!(
                "auth9-admin client created in realm '{}' with preset secret",
                realm
            );
        }

        Ok(())
    }

    /// Get the client secret for auth9-admin in a specific realm
    async fn get_admin_client_secret_in_realm(
        &self,
        token: &str,
        realm: &str,
    ) -> anyhow::Result<String> {
        // Get client UUID
        let client_uuid = self
            .get_client_uuid_by_client_id_in_realm(token, realm, DEFAULT_ADMIN_CLIENT_ID)
            .await?;

        // Get client secret
        let secret = self
            .get_client_secret_in_realm(token, realm, &client_uuid)
            .await?;
        Ok(secret)
    }

    /// Get client UUID by client ID in a specific realm
    async fn get_client_uuid_by_client_id_in_realm(
        &self,
        token: &str,
        realm: &str,
        client_id: &str,
    ) -> anyhow::Result<String> {
        let url = format!("{}/admin/realms/{}/clients", self.config.url, realm);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(token)
            .query(&[("clientId", client_id)])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Failed to get client by clientId: {} - {}", status, body);
        }

        let clients: Vec<serde_json::Value> = response.json().await?;
        if clients.is_empty() {
            anyhow::bail!("Client '{}' not found in realm '{}'", client_id, realm);
        }

        let client_uuid = clients[0]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Client UUID not found in response"))?
            .to_string();

        Ok(client_uuid)
    }

    /// Get client secret in a specific realm
    async fn get_client_secret_in_realm(
        &self,
        token: &str,
        realm: &str,
        client_uuid: &str,
    ) -> anyhow::Result<String> {
        let url = format!(
            "{}/admin/realms/{}/clients/{}/client-secret",
            self.config.url, realm, client_uuid
        );

        let response = self.http_client.get(&url).bearer_auth(token).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Failed to get client secret: {} - {}", status, body);
        }

        let secret_response: serde_json::Value = response.json().await?;
        let secret = secret_response["value"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Client secret value not found in response"))?
            .to_string();

        Ok(secret)
    }

    /// Seed auth9-admin client in master realm (idempotent)
    pub async fn seed_master_admin_client(&self) -> anyhow::Result<()> {
        info!("Seeding auth9-admin client in master realm...");

        // Get admin token using admin-cli
        let token = self.get_master_admin_token().await?;

        // Check if client already exists in master realm
        if self.admin_client_exists_in_realm(&token, "master").await? {
            info!("auth9-admin client already exists in master realm, skipping");
            return Ok(());
        }

        // Create client in master realm
        self.create_admin_client_in_realm(&token, "master").await?;
        Ok(())
    }

    /// Seed auth9-admin client in configured realm (idempotent)
    pub async fn seed_admin_client(&self) -> anyhow::Result<()> {
        info!(
            "Seeding auth9-admin client in realm '{}'...",
            self.config.realm
        );

        // Get admin token using current configuration (admin-cli or auth9-admin)
        let token = self.get_master_admin_token().await?;

        // Check if client already exists
        if self
            .admin_client_exists_in_realm(&token, &self.config.realm)
            .await?
        {
            info!(
                "auth9-admin client already exists in realm '{}', skipping",
                self.config.realm
            );
            return Ok(());
        }

        // Create client in configured realm
        self.create_admin_client_in_realm(&token, &self.config.realm)
            .await?;
        Ok(())
    }
}
