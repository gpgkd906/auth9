//! Service/Client business logic

use crate::cache::CacheManager;
use crate::domain::{
    Client, ClientWithSecret, CreateServiceInput, Service, ServiceWithClient, UpdateServiceInput,
};
use crate::error::{AppError, Result};
use crate::repository::ServiceRepository;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use rand::Rng;
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

pub struct ClientService<R: ServiceRepository> {
    repo: Arc<R>,
    cache_manager: Option<CacheManager>,
}

impl<R: ServiceRepository> ClientService<R> {
    pub fn new(repo: Arc<R>, cache_manager: Option<CacheManager>) -> Self {
        Self {
            repo,
            cache_manager,
        }
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
            .create_client(service.id.0, &input.client_id, &secret_hash, Some("Initial Key".to_string()))
            .await?;

        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }

        Ok(ServiceWithClient {
            service,
            client: ClientWithSecret {
                client,
                client_secret,
            }
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
            .create_client(service.id.0, &input.client_id, &secret_hash, Some("Initial Key".to_string()))
            .await?;

        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }

        Ok(ServiceWithClient {
            service,
            client: ClientWithSecret {
                client,
                client_secret,
            }
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

        let client = self.repo.create_client(service_id, &client_id, &secret_hash, name).await?;

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
        let client = self.repo.create_client(service_id, &client_id, &secret_hash, name).await?;

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
            .ok_or_else(|| AppError::NotFound(format!("Service for client '{}' not found", client_id)))?;
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
        let client = self.repo.find_client_by_client_id(client_id).await?
            .ok_or_else(|| AppError::NotFound(format!("Client {} not found", client_id)))?;
        
        // Generate new secret and hash
        let new_secret = generate_client_secret();
        let secret_hash = hash_secret(&new_secret)?;
        
        // Update in database
        self.repo.update_client_secret_hash(client_id, &secret_hash).await?;
        
        // Invalidate cache if applicable
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(client.service_id.0).await;
        }
        
        Ok(new_secret)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let _ = self.get(id).await?;
        self.repo.delete(id).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(id).await;
        }
        Ok(())
    }

    pub async fn delete_client(&self, service_id: Uuid, client_id: &str) -> Result<()> {
        self.repo.delete_client(service_id, client_id).await
    }

    /// Update a client's secret hash in the database
    pub async fn update_client_secret_hash(&self, client_id: &str, new_secret_hash: &str) -> Result<()> {
        self.repo.update_client_secret_hash(client_id, new_secret_hash).await
    }

    pub async fn verify_secret(&self, client_id: &str, secret: &str) -> Result<Service> {
        // 1. Find Client to get hash
        let client = self.repo.find_client_by_client_id(client_id).await?
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

    #[test]
    fn test_generate_client_secret() {
        let secret1 = generate_client_secret();
        let secret2 = generate_client_secret();

        // Should be 43 characters (32 bytes base64 encoded without padding)
        assert_eq!(secret1.len(), 43);
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn test_hash_and_verify_secret() {
        let secret = "test-secret-123";
        let hash = hash_secret(secret).unwrap();

        assert!(verify_secret(secret, &hash).unwrap());
        assert!(!verify_secret("wrong-secret", &hash).unwrap());
    }

    #[tokio::test]
    async fn test_create_client() {
        use crate::repository::service::MockServiceRepository;
        use crate::domain::StringUuid;
        
        let mut mock_repo = MockServiceRepository::new();
        let service_id = Uuid::new_v4();
        
        mock_repo
            .expect_find_by_id()
            .with(mockall::predicate::eq(service_id))
            .times(1)
            .returning(move |_| Ok(Some(Service {
                id: StringUuid(service_id),
                tenant_id: None,
                name: "Test Service".to_string(),
                base_url: None,
                redirect_uris: vec![],
                logout_uris: vec![],
                status: crate::domain::ServiceStatus::Active,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })));

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
            
        let service = ClientService::new(Arc::new(mock_repo), None);
        
        let result = service.create_client(service_id, Some("Client 1".to_string())).await;
        
        assert!(result.is_ok());
        let client_with_secret = result.unwrap();
        assert_eq!(client_with_secret.client.name, Some("Client 1".to_string()));
        assert!(!client_with_secret.client_secret.is_empty());
    }

    #[tokio::test]
    async fn test_regenerate_client_secret() {
        use crate::repository::service::MockServiceRepository;
        use crate::domain::StringUuid;
        
        let mut mock_repo = MockServiceRepository::new();
        let client_id = "test-client-id";
        
        mock_repo
            .expect_find_client_by_client_id()
            .with(mockall::predicate::eq(client_id))
            .times(1)
            .returning(move |_| Ok(Some(crate::domain::Client {
                id: StringUuid(Uuid::new_v4()),
                service_id: StringUuid(Uuid::new_v4()),
                client_id: client_id.to_string(),
                client_secret_hash: "old_hash".to_string(),
                name: None,
                created_at: chrono::Utc::now(),
            })));

        mock_repo
            .expect_update_client_secret_hash()
            .with(
                mockall::predicate::eq(client_id),
                mockall::predicate::always(), // We can't easily predict the new hash
            )
            .times(1)
            .returning(|_, _| Ok(()));
            
        let service = ClientService::new(Arc::new(mock_repo), None);
        
        let result = service.regenerate_client_secret(client_id).await;
        
        assert!(result.is_ok());
        let new_secret = result.unwrap();
        assert!(!new_secret.is_empty());
    }

}
