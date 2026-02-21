//! Service/Client business logic

use crate::cache::CacheManager;
use crate::domain::{
    Client, ClientWithSecret, CreateServiceInput, Service, ServiceWithClient, StringUuid,
    UpdateServiceInput,
};
use crate::error::{AppError, Result};
use crate::repository::action::ActionRepository;
use crate::repository::service_branding::ServiceBrandingRepository;
use crate::repository::{RbacRepository, ServiceRepository};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use rand::Rng;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;
use validator::Validate;

pub struct ClientService<R: ServiceRepository, RR: RbacRepository, AR: ActionRepository = crate::repository::action::ActionRepositoryImpl, BR: ServiceBrandingRepository = crate::repository::service_branding::ServiceBrandingRepositoryImpl> {
    repo: Arc<R>,
    rbac_repo: Arc<RR>,
    action_repo: Option<Arc<AR>>,
    branding_repo: Option<Arc<BR>>,
    cache_manager: Option<CacheManager>,
}

impl<R: ServiceRepository, RR: RbacRepository, AR: ActionRepository, BR: ServiceBrandingRepository> ClientService<R, RR, AR, BR> {
    pub fn new(repo: Arc<R>, rbac_repo: Arc<RR>, cache_manager: Option<CacheManager>) -> Self {
        Self {
            repo,
            rbac_repo,
            action_repo: None,
            branding_repo: None,
            cache_manager,
        }
    }

    pub fn with_cascade_repos(mut self, action_repo: Arc<AR>, branding_repo: Arc<BR>) -> Self {
        self.action_repo = Some(action_repo);
        self.branding_repo = Some(branding_repo);
        self
    }

    pub async fn create(&self, input: CreateServiceInput) -> Result<ServiceWithClient> {
        input.validate()?;

        // Check for duplicate client_id
        if self
            .repo
            .find_client_by_client_id(&input.client_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Client with client_id '{}' already exists",
                input.client_id
            )));
        }

        // Generate client secret
        let client_secret = generate_client_secret();
        let secret_hash = hash_secret(&client_secret)?;

        // Create Service (without client credentials)
        let service = self.repo.create(&input).await?;

        // Create Client
        let client = self
            .repo
            .create_client(
                service.id.0,
                &input.client_id,
                &secret_hash,
                Some("Initial Key".to_string()),
            )
            .await?;

        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }

        Ok(ServiceWithClient {
            service,
            client: ClientWithSecret {
                client,
                client_secret,
            },
        })
    }

    pub async fn create_with_secret(
        &self,
        input: CreateServiceInput,
        client_secret: String,
    ) -> Result<ServiceWithClient> {
        input.validate()?;

        if self
            .repo
            .find_client_by_client_id(&input.client_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Client with client_id '{}' already exists",
                input.client_id
            )));
        }

        let secret_hash = hash_secret(&client_secret)?;

        let service = self.repo.create(&input).await?;
        let client = self
            .repo
            .create_client(
                service.id.0,
                &input.client_id,
                &secret_hash,
                Some("Initial Key".to_string()),
            )
            .await?;

        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }

        Ok(ServiceWithClient {
            service,
            client: ClientWithSecret {
                client,
                client_secret,
            },
        })
    }

    pub async fn create_client(
        &self,
        service_id: Uuid,
        name: Option<String>,
    ) -> Result<ClientWithSecret> {
        // Verify service exists
        let _ = self.get(service_id).await?;

        // Generate ID and Secret
        let client_id = Uuid::new_v4().to_string();
        let client_secret = generate_client_secret();
        let secret_hash = hash_secret(&client_secret)?;

        let client = self
            .repo
            .create_client(service_id, &client_id, &secret_hash, name)
            .await?;

        Ok(ClientWithSecret {
            client,
            client_secret,
        })
    }

    pub async fn create_client_with_secret(
        &self,
        service_id: Uuid,
        client_id: String,
        client_secret: String,
        name: Option<String>,
    ) -> Result<ClientWithSecret> {
        let _ = self.get(service_id).await?;

        // Check duplicate
        if self
            .repo
            .find_client_by_client_id(&client_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Client with client_id '{}' already exists",
                client_id
            )));
        }

        let secret_hash = hash_secret(&client_secret)?;
        let client = self
            .repo
            .create_client(service_id, &client_id, &secret_hash, name)
            .await?;

        Ok(ClientWithSecret {
            client,
            client_secret,
        })
    }

    pub async fn get(&self, id: Uuid) -> Result<Service> {
        // Cache logic for Service config (domain/base_url etc)
        // Note: Clients are not cached in service config usually, or if they are, cache needs update.
        // Assuming service config cache is only for Service struct.
        if let Some(cache) = &self.cache_manager {
            if let Ok(Some(service)) = cache.get_service_config(id).await {
                return Ok(service);
            }
        }
        let service = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Service {} not found", id)))?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }
        Ok(service)
    }

    pub async fn get_by_client_id(&self, client_id: &str) -> Result<Service> {
        let service = self
            .repo
            .find_by_client_id(client_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Service for client '{}' not found", client_id))
            })?;
        // Cache could be set here too
        Ok(service)
    }

    pub async fn list(
        &self,
        tenant_id: Option<Uuid>,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<Service>, i64)> {
        let offset = (page - 1) * per_page;
        let services = self.repo.list(tenant_id, offset, per_page).await?;
        let total = self.repo.count(tenant_id).await?;
        Ok((services, total))
    }

    pub async fn list_clients(&self, service_id: Uuid) -> Result<Vec<Client>> {
        self.repo.list_clients(service_id).await
    }

    pub async fn update(&self, id: Uuid, input: UpdateServiceInput) -> Result<Service> {
        input.validate()?;
        let _ = self.get(id).await?;
        let service = self.repo.update(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(id).await;
        }
        Ok(service)
    }

    /// Regenerate the client secret for a specific client.
    /// Returns the new plaintext secret (only shown once).
    pub async fn regenerate_client_secret(&self, client_id: &str) -> Result<String> {
        // Verify client exists
        let client = self
            .repo
            .find_client_by_client_id(client_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Client {} not found", client_id)))?;

        // Generate new secret and hash
        let new_secret = generate_client_secret();
        let secret_hash = hash_secret(&new_secret)?;

        // Update in database
        self.repo
            .update_client_secret_hash(client_id, &secret_hash)
            .await?;

        // Invalidate cache if applicable
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(client.service_id.0).await;
        }

        Ok(new_secret)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let _ = self.get(id).await?;
        let service_id = StringUuid(id);

        // CASCADE DELETE:
        // 1. Delete all clients (API keys) for this service
        let deleted_clients = self.repo.delete_clients_by_service(id).await?;
        warn!(
            service_id = %id,
            deleted_clients = deleted_clients,
            "Deleted clients for service"
        );

        // 2. Clear parent_role_id references before deleting roles
        let cleared_refs = self
            .rbac_repo
            .clear_parent_role_references(service_id)
            .await?;
        warn!(
            service_id = %id,
            cleared_refs = cleared_refs,
            "Cleared parent role references"
        );

        // 3. Delete role_permissions, user_tenant_roles, and roles for this service
        let deleted_roles = self.rbac_repo.delete_roles_by_service(service_id).await?;
        warn!(
            service_id = %id,
            deleted_roles = deleted_roles,
            "Deleted roles for service"
        );

        // 4. Delete permissions for this service
        let deleted_perms = self
            .rbac_repo
            .delete_permissions_by_service(service_id)
            .await?;
        warn!(
            service_id = %id,
            deleted_perms = deleted_perms,
            "Deleted permissions for service"
        );

        // 5. Delete actions for this service
        if let Some(action_repo) = &self.action_repo {
            let deleted_actions = action_repo.delete_by_service(service_id).await?;
            warn!(
                service_id = %id,
                deleted_actions = deleted_actions,
                "Deleted actions for service"
            );
        }

        // 6. Delete service branding
        if let Some(branding_repo) = &self.branding_repo {
            branding_repo.delete_by_service_id(service_id).await?;
            warn!(
                service_id = %id,
                "Deleted service branding"
            );
        }

        // 7. Delete the service itself
        self.repo.delete(id).await?;

        // 8. Clear cache
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(id).await;
        }
        Ok(())
    }

    pub async fn delete_client(&self, service_id: Uuid, client_id: &str) -> Result<()> {
        self.repo.delete_client(service_id, client_id).await
    }

    /// Update a client's secret hash in the database
    pub async fn update_client_secret_hash(
        &self,
        client_id: &str,
        new_secret_hash: &str,
    ) -> Result<()> {
        self.repo
            .update_client_secret_hash(client_id, new_secret_hash)
            .await
    }

    pub async fn verify_secret(&self, client_id: &str, secret: &str) -> Result<Service> {
        // 1. Find Client to get hash
        let client = self
            .repo
            .find_client_by_client_id(client_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid client credentials".to_string()))?;

        // 2. Verify Secret
        if !verify_secret(secret, &client.client_secret_hash)? {
            return Err(AppError::Unauthorized(
                "Invalid client credentials".to_string(),
            ));
        }

        // 3. Get Service
        // We can use get_by_client_id or fetch by client.service_id
        let service = self.get(client.service_id.0).await?;

        // Ensure service is active? (Service struct has status, checked in use cases usually?)
        // Existing code checked status in api/service.rs? No, domain/service.rs defines Active/Inactive
        // Logic might want to check if service.status is Active.
        if service.status != crate::domain::ServiceStatus::Active {
            return Err(AppError::Unauthorized("Service is inactive".to_string()));
        }

        Ok(service)
    }
}

/// Generate a cryptographically secure client secret
fn generate_client_secret() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Hash a client secret using Argon2
fn hash_secret(secret: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to hash secret: {}", e)))?;
    Ok(hash.to_string())
}

/// Verify a client secret against its hash
fn verify_secret(secret: &str, hash: &str) -> Result<bool> {
    use argon2::{PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(secret.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::rbac::MockRbacRepository;
    use crate::repository::service::MockServiceRepository;
    use mockall::predicate::*;

    /// Helper function to create a ClientService with mock repositories
    fn create_test_service(
        service_repo: MockServiceRepository,
    ) -> ClientService<MockServiceRepository, MockRbacRepository> {
        ClientService::new(
            Arc::new(service_repo),
            Arc::new(MockRbacRepository::new()),
            None,
        )
    }

    /// Helper function to create a ClientService with all mock repositories customizable
    fn create_test_service_full(
        service_repo: MockServiceRepository,
        rbac_repo: MockRbacRepository,
    ) -> ClientService<MockServiceRepository, MockRbacRepository> {
        ClientService::new(Arc::new(service_repo), Arc::new(rbac_repo), None)
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_generate_client_secret() {
        let secret1 = generate_client_secret();
        let secret2 = generate_client_secret();

        // Should be 43 characters (32 bytes base64 encoded without padding)
        assert_eq!(secret1.len(), 43);
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn test_generate_client_secret_multiple() {
        // Generate multiple secrets and ensure they're all unique
        let secrets: Vec<String> = (0..10).map(|_| generate_client_secret()).collect();
        let unique: std::collections::HashSet<_> = secrets.iter().collect();
        assert_eq!(unique.len(), secrets.len());
    }

    #[test]
    fn test_hash_and_verify_secret() {
        let secret = "test-secret-123";
        let hash = hash_secret(secret).unwrap();

        assert!(verify_secret(secret, &hash).unwrap());
        assert!(!verify_secret("wrong-secret", &hash).unwrap());
    }

    #[test]
    fn test_hash_secret_different_hashes() {
        // Same secret should produce different hashes (due to random salt)
        let secret = "my-secret";
        let hash1 = hash_secret(secret).unwrap();
        let hash2 = hash_secret(secret).unwrap();

        assert_ne!(hash1, hash2);
        // But both should verify correctly
        assert!(verify_secret(secret, &hash1).unwrap());
        assert!(verify_secret(secret, &hash2).unwrap());
    }

    #[test]
    fn test_verify_secret_invalid_hash() {
        let result = verify_secret("secret", "invalid-hash-format");
        assert!(result.is_err());
    }

    // ==================== Create Service Tests ====================

    #[tokio::test]
    async fn test_create_service_success() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_client_by_client_id()
            .with(eq("new-client-id"))
            .returning(|_| Ok(None));

        mock.expect_create().returning(move |input| {
            Ok(Service {
                id: StringUuid(service_id),
                tenant_id: input.tenant_id.map(StringUuid::from),
                name: input.name.clone(),
                base_url: input.base_url.clone(),
                redirect_uris: input.redirect_uris.clone(),
                logout_uris: input.logout_uris.clone().unwrap_or_default(),
                status: crate::domain::ServiceStatus::Active,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        });

        mock.expect_create_client()
            .returning(move |sid, cid, _hash, name| {
                Ok(crate::domain::Client {
                    id: StringUuid(Uuid::new_v4()),
                    service_id: StringUuid(sid),
                    client_id: cid.to_string(),
                    client_secret_hash: "hashed".to_string(),
                    name,
                    created_at: chrono::Utc::now(),
                })
            });

        let service = create_test_service(mock);

        let input = CreateServiceInput {
            tenant_id: Some(Uuid::new_v4()),
            name: "My Service".to_string(),
            client_id: "new-client-id".to_string(),
            base_url: Some("https://example.com".to_string()),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            logout_uris: None,
        };

        let result = service.create(input).await;
        assert!(result.is_ok());
        let swc = result.unwrap();
        assert_eq!(swc.service.name, "My Service");
        assert!(!swc.client.client_secret.is_empty());
    }

    #[tokio::test]
    async fn test_create_service_duplicate_client_id() {
        let mut mock = MockServiceRepository::new();

        mock.expect_find_client_by_client_id()
            .with(eq("existing-client"))
            .returning(|_| {
                Ok(Some(crate::domain::Client {
                    id: StringUuid::new_v4(),
                    service_id: StringUuid::new_v4(),
                    client_id: "existing-client".to_string(),
                    client_secret_hash: "hash".to_string(),
                    name: None,
                    created_at: chrono::Utc::now(),
                }))
            });

        let service = create_test_service(mock);

        let input = CreateServiceInput {
            tenant_id: None,
            name: "New Service".to_string(),
            client_id: "existing-client".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        let result = service.create(input).await;
        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    #[tokio::test]
    async fn test_create_service_invalid_input() {
        let mock = MockServiceRepository::new();
        let service = create_test_service(mock);

        let input = CreateServiceInput {
            tenant_id: None,
            name: "".to_string(), // Empty name is invalid
            client_id: "client".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        let result = service.create(input).await;
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[tokio::test]
    async fn test_create_with_secret() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_client_by_client_id()
            .returning(|_| Ok(None));

        mock.expect_create().returning(move |_| {
            Ok(Service {
                id: StringUuid(service_id),
                ..Default::default()
            })
        });

        mock.expect_create_client()
            .returning(move |sid, cid, _hash, _| {
                Ok(crate::domain::Client {
                    id: StringUuid::new_v4(),
                    service_id: StringUuid(sid),
                    client_id: cid.to_string(),
                    client_secret_hash: "hash".to_string(),
                    name: None,
                    created_at: chrono::Utc::now(),
                })
            });

        let service = create_test_service(mock);

        let input = CreateServiceInput {
            tenant_id: None,
            name: "Test".to_string(),
            client_id: "my-client".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        let result = service
            .create_with_secret(input, "my-custom-secret".to_string())
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().client.client_secret, "my-custom-secret");
    }

    // ==================== Create Client Tests ====================

    #[tokio::test]
    async fn test_create_client() {
        let mut mock_repo = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock_repo
            .expect_find_by_id()
            .with(eq(service_id))
            .times(1)
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    tenant_id: None,
                    name: "Test Service".to_string(),
                    base_url: None,
                    redirect_uris: vec![],
                    logout_uris: vec![],
                    status: crate::domain::ServiceStatus::Active,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            });

        mock_repo
            .expect_create_client()
            .times(1)
            .returning(move |sid, cid, _hash, name| {
                Ok(crate::domain::Client {
                    id: StringUuid(Uuid::new_v4()),
                    service_id: StringUuid(sid),
                    client_id: cid.to_string(),
                    client_secret_hash: "hashed_secret".to_string(),
                    name,
                    created_at: chrono::Utc::now(),
                })
            });

        let service = create_test_service(mock_repo);

        let result = service
            .create_client(service_id, Some("Client 1".to_string()))
            .await;

        assert!(result.is_ok());
        let client_with_secret = result.unwrap();
        assert_eq!(client_with_secret.client.name, Some("Client 1".to_string()));
        assert!(!client_with_secret.client_secret.is_empty());
    }

    #[tokio::test]
    async fn test_create_client_service_not_found() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.create_client(service_id, None).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_client_with_secret() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    status: crate::domain::ServiceStatus::Active,
                    ..Default::default()
                }))
            });

        mock.expect_find_client_by_client_id()
            .with(eq("custom-client-id"))
            .returning(|_| Ok(None));

        mock.expect_create_client().returning(|sid, cid, _, name| {
            Ok(crate::domain::Client {
                id: StringUuid::new_v4(),
                service_id: StringUuid(sid),
                client_id: cid.to_string(),
                client_secret_hash: "hash".to_string(),
                name,
                created_at: chrono::Utc::now(),
            })
        });

        let service = create_test_service(mock);

        let result = service
            .create_client_with_secret(
                service_id,
                "custom-client-id".to_string(),
                "custom-secret".to_string(),
                Some("Custom Client".to_string()),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().client_secret, "custom-secret");
    }

    #[tokio::test]
    async fn test_create_client_with_secret_duplicate_id() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id().returning(move |_| {
            Ok(Some(Service {
                id: StringUuid(service_id),
                status: crate::domain::ServiceStatus::Active,
                ..Default::default()
            }))
        });

        mock.expect_find_client_by_client_id()
            .with(eq("existing-id"))
            .returning(|_| {
                Ok(Some(crate::domain::Client {
                    id: StringUuid::new_v4(),
                    service_id: StringUuid::new_v4(),
                    client_id: "existing-id".to_string(),
                    client_secret_hash: "hash".to_string(),
                    name: None,
                    created_at: chrono::Utc::now(),
                }))
            });

        let service = create_test_service(mock);

        let result = service
            .create_client_with_secret(
                service_id,
                "existing-id".to_string(),
                "secret".to_string(),
                None,
            )
            .await;

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }

    // ==================== Get Service Tests ====================

    #[tokio::test]
    async fn test_get_service_success() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    name: "My Service".to_string(),
                    status: crate::domain::ServiceStatus::Active,
                    ..Default::default()
                }))
            });

        let service = create_test_service(mock);

        let result = service.get(service_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "My Service");
    }

    #[tokio::test]
    async fn test_get_service_not_found() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.get(service_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_by_client_id_success() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_client_id()
            .with(eq("my-client"))
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    name: "Found Service".to_string(),
                    ..Default::default()
                }))
            });

        let service = create_test_service(mock);

        let result = service.get_by_client_id("my-client").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Found Service");
    }

    #[tokio::test]
    async fn test_get_by_client_id_not_found() {
        let mut mock = MockServiceRepository::new();

        mock.expect_find_by_client_id()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.get_by_client_id("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ==================== List Tests ====================

    #[tokio::test]
    async fn test_list_services() {
        let mut mock = MockServiceRepository::new();

        mock.expect_list()
            .with(eq(None), eq(0), eq(10))
            .returning(|_, _, _| {
                Ok(vec![
                    Service {
                        name: "Service 1".to_string(),
                        ..Default::default()
                    },
                    Service {
                        name: "Service 2".to_string(),
                        ..Default::default()
                    },
                ])
            });

        mock.expect_count().with(eq(None)).returning(|_| Ok(2));

        let service = create_test_service(mock);

        let result = service.list(None, 1, 10).await;
        assert!(result.is_ok());
        let (services, total) = result.unwrap();
        assert_eq!(services.len(), 2);
        assert_eq!(total, 2);
    }

    #[tokio::test]
    async fn test_list_services_with_tenant() {
        let mut mock = MockServiceRepository::new();
        let tenant_id = Uuid::new_v4();

        mock.expect_list()
            .with(eq(Some(tenant_id)), eq(10), eq(10))
            .returning(|_, _, _| {
                Ok(vec![Service {
                    name: "Tenant Service".to_string(),
                    ..Default::default()
                }])
            });

        mock.expect_count()
            .with(eq(Some(tenant_id)))
            .returning(|_| Ok(11));

        let service = create_test_service(mock);

        let result = service.list(Some(tenant_id), 2, 10).await;
        assert!(result.is_ok());
        let (services, total) = result.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(total, 11);
    }

    #[tokio::test]
    async fn test_list_clients() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_list_clients()
            .with(eq(service_id))
            .returning(|_| {
                Ok(vec![crate::domain::Client {
                    id: StringUuid::new_v4(),
                    service_id: StringUuid::new_v4(),
                    client_id: "client-1".to_string(),
                    client_secret_hash: "hash".to_string(),
                    name: Some("Client 1".to_string()),
                    created_at: chrono::Utc::now(),
                }])
            });

        let service = create_test_service(mock);

        let result = service.list_clients(service_id).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    // ==================== Update Tests ====================

    #[tokio::test]
    async fn test_update_service_success() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    name: "Old Name".to_string(),
                    status: crate::domain::ServiceStatus::Active,
                    ..Default::default()
                }))
            });

        mock.expect_update().returning(|_, input| {
            Ok(Service {
                name: input.name.clone().unwrap_or_default(),
                status: input
                    .status
                    .clone()
                    .unwrap_or(crate::domain::ServiceStatus::Active),
                ..Default::default()
            })
        });

        let service = create_test_service(mock);

        let input = UpdateServiceInput {
            name: Some("New Name".to_string()),
            base_url: None,
            redirect_uris: None,
            logout_uris: None,
            status: None,
        };

        let result = service.update(service_id, input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "New Name");
    }

    #[tokio::test]
    async fn test_update_service_not_found() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let input = UpdateServiceInput {
            name: Some("New".to_string()),
            base_url: None,
            redirect_uris: None,
            logout_uris: None,
            status: None,
        };

        let result = service.update(service_id, input).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ==================== Regenerate Secret Tests ====================

    #[tokio::test]
    async fn test_regenerate_client_secret() {
        let mut mock_repo = MockServiceRepository::new();
        let client_id = "test-client-id";

        mock_repo
            .expect_find_client_by_client_id()
            .with(eq(client_id))
            .times(1)
            .returning(move |_| {
                Ok(Some(crate::domain::Client {
                    id: StringUuid(Uuid::new_v4()),
                    service_id: StringUuid(Uuid::new_v4()),
                    client_id: client_id.to_string(),
                    client_secret_hash: "old_hash".to_string(),
                    name: None,
                    created_at: chrono::Utc::now(),
                }))
            });

        mock_repo
            .expect_update_client_secret_hash()
            .with(eq(client_id), always())
            .times(1)
            .returning(|_, _| Ok(()));

        let service = create_test_service(mock_repo);

        let result = service.regenerate_client_secret(client_id).await;

        assert!(result.is_ok());
        let new_secret = result.unwrap();
        assert!(!new_secret.is_empty());
        assert_eq!(new_secret.len(), 43); // Base64 encoded 32 bytes
    }

    #[tokio::test]
    async fn test_regenerate_client_secret_not_found() {
        let mut mock = MockServiceRepository::new();

        mock.expect_find_client_by_client_id()
            .with(eq("nonexistent"))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.regenerate_client_secret("nonexistent").await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    // ==================== Delete Tests ====================

    #[tokio::test]
    async fn test_delete_service_cascade_success() {
        let mut service_repo = MockServiceRepository::new();
        let mut rbac_repo = MockRbacRepository::new();
        let service_id = Uuid::new_v4();

        service_repo
            .expect_find_by_id()
            .with(eq(service_id))
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    status: crate::domain::ServiceStatus::Active,
                    ..Default::default()
                }))
            });

        service_repo
            .expect_delete_clients_by_service()
            .returning(|_| Ok(2));

        rbac_repo
            .expect_clear_parent_role_references()
            .returning(|_| Ok(0));

        rbac_repo
            .expect_delete_roles_by_service()
            .returning(|_| Ok(3));

        rbac_repo
            .expect_delete_permissions_by_service()
            .returning(|_| Ok(5));

        service_repo
            .expect_delete()
            .with(eq(service_id))
            .returning(|_| Ok(()));

        let service = create_test_service_full(service_repo, rbac_repo);

        let result = service.delete(service_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_service_not_found() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.delete(service_id).await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_client() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();

        mock.expect_delete_client()
            .with(eq(service_id), eq("client-to-delete"))
            .returning(|_, _| Ok(()));

        let service = create_test_service(mock);

        let result = service.delete_client(service_id, "client-to-delete").await;
        assert!(result.is_ok());
    }

    // ==================== Verify Secret Tests ====================

    #[tokio::test]
    async fn test_verify_secret_success() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();
        let client_secret = "test-secret";
        let secret_hash = hash_secret(client_secret).unwrap();

        mock.expect_find_client_by_client_id()
            .with(eq("valid-client"))
            .returning(move |_| {
                Ok(Some(crate::domain::Client {
                    id: StringUuid::new_v4(),
                    service_id: StringUuid(service_id),
                    client_id: "valid-client".to_string(),
                    client_secret_hash: secret_hash.clone(),
                    name: None,
                    created_at: chrono::Utc::now(),
                }))
            });

        mock.expect_find_by_id()
            .with(eq(service_id))
            .returning(move |_| {
                Ok(Some(Service {
                    id: StringUuid(service_id),
                    status: crate::domain::ServiceStatus::Active,
                    ..Default::default()
                }))
            });

        let service = create_test_service(mock);

        let result = service.verify_secret("valid-client", client_secret).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_secret_invalid_client() {
        let mut mock = MockServiceRepository::new();

        mock.expect_find_client_by_client_id()
            .with(eq("invalid-client"))
            .returning(|_| Ok(None));

        let service = create_test_service(mock);

        let result = service.verify_secret("invalid-client", "secret").await;
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn test_verify_secret_wrong_secret() {
        let mut mock = MockServiceRepository::new();
        let secret_hash = hash_secret("correct-secret").unwrap();

        mock.expect_find_client_by_client_id()
            .with(eq("my-client"))
            .returning(move |_| {
                Ok(Some(crate::domain::Client {
                    id: StringUuid::new_v4(),
                    service_id: StringUuid::new_v4(),
                    client_id: "my-client".to_string(),
                    client_secret_hash: secret_hash.clone(),
                    name: None,
                    created_at: chrono::Utc::now(),
                }))
            });

        let service = create_test_service(mock);

        let result = service.verify_secret("my-client", "wrong-secret").await;
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn test_verify_secret_inactive_service() {
        let mut mock = MockServiceRepository::new();
        let service_id = Uuid::new_v4();
        let client_secret = "test-secret";
        let secret_hash = hash_secret(client_secret).unwrap();

        mock.expect_find_client_by_client_id().returning(move |_| {
            Ok(Some(crate::domain::Client {
                id: StringUuid::new_v4(),
                service_id: StringUuid(service_id),
                client_id: "client".to_string(),
                client_secret_hash: secret_hash.clone(),
                name: None,
                created_at: chrono::Utc::now(),
            }))
        });

        mock.expect_find_by_id().returning(move |_| {
            Ok(Some(Service {
                id: StringUuid(service_id),
                status: crate::domain::ServiceStatus::Inactive, // Inactive service
                ..Default::default()
            }))
        });

        let service = create_test_service(mock);

        let result = service.verify_secret("client", client_secret).await;
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    // ==================== Update Client Secret Hash Test ====================

    #[tokio::test]
    async fn test_update_client_secret_hash() {
        let mut mock = MockServiceRepository::new();

        mock.expect_update_client_secret_hash()
            .with(eq("my-client"), eq("new-hash"))
            .returning(|_, _| Ok(()));

        let service = create_test_service(mock);

        let result = service
            .update_client_secret_hash("my-client", "new-hash")
            .await;
        assert!(result.is_ok());
    }
}
