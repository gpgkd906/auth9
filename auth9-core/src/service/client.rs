//! Service/Client business logic

use crate::cache::CacheManager;
use crate::domain::{CreateServiceInput, Service, ServiceWithSecret, UpdateServiceInput};
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

    pub async fn create(&self, input: CreateServiceInput) -> Result<ServiceWithSecret> {
        input.validate()?;

        // Check for duplicate client_id
        if self
            .repo
            .find_by_client_id(&input.client_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Service with client_id '{}' already exists",
                input.client_id
            )));
        }

        // Generate client secret
        let client_secret = generate_client_secret();
        let secret_hash = hash_secret(&client_secret)?;

        let service = self.repo.create(&input, &secret_hash).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }

        Ok(ServiceWithSecret {
            service,
            client_secret: Some(client_secret),
        })
    }

    pub async fn create_with_secret(
        &self,
        input: CreateServiceInput,
        client_secret: String,
    ) -> Result<ServiceWithSecret> {
        input.validate()?;

        if self
            .repo
            .find_by_client_id(&input.client_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "Service with client_id '{}' already exists",
                input.client_id
            )));
        }

        let secret_hash = hash_secret(&client_secret)?;
        let service = self.repo.create(&input, &secret_hash).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }

        Ok(ServiceWithSecret {
            service,
            client_secret: Some(client_secret),
        })
    }

    pub async fn get(&self, id: Uuid) -> Result<Service> {
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
            .ok_or_else(|| AppError::NotFound(format!("Service '{}' not found", client_id)))?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.set_service_config(service.id.0, &service).await;
        }
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

    pub async fn update(&self, id: Uuid, input: UpdateServiceInput) -> Result<Service> {
        input.validate()?;
        let _ = self.get(id).await?;
        let service = self.repo.update(id, &input).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(id).await;
        }
        Ok(service)
    }

    pub async fn regenerate_secret(&self, id: Uuid) -> Result<String> {
        let _ = self.get(id).await?;

        let client_secret = generate_client_secret();
        self.regenerate_secret_with_value(id, client_secret.clone())
            .await?;
        Ok(client_secret)
    }

    pub async fn regenerate_secret_with_value(
        &self,
        id: Uuid,
        client_secret: String,
    ) -> Result<String> {
        let _ = self.get(id).await?;
        let secret_hash = hash_secret(&client_secret)?;
        self.repo.update_secret(id, &secret_hash).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(id).await;
        }
        Ok(client_secret)
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let _ = self.get(id).await?;
        self.repo.delete(id).await?;
        if let Some(cache) = &self.cache_manager {
            let _ = cache.invalidate_service_config(id).await;
        }
        Ok(())
    }

    pub async fn verify_secret(&self, client_id: &str, secret: &str) -> Result<Service> {
        let service = self.get_by_client_id(client_id).await?;

        if verify_secret(secret, &service.client_secret_hash)? {
            Ok(service)
        } else {
            Err(AppError::Unauthorized(
                "Invalid client credentials".to_string(),
            ))
        }
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
}
