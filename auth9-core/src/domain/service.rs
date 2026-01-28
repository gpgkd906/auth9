//! Service/Client domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Service status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Active,
    Inactive,
}

impl Default for ServiceStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Service/Client entity (OIDC client)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Service {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub name: String,
    pub client_id: String,
    /// Hashed client secret (never expose raw secret)
    #[serde(skip_serializing)]
    pub client_secret_hash: String,
    pub base_url: Option<String>,
    #[sqlx(json)]
    pub redirect_uris: Vec<String>,
    #[sqlx(json)]
    pub logout_uris: Vec<String>,
    pub status: ServiceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Default for Service {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            tenant_id: None,
            name: String::new(),
            client_id: String::new(),
            client_secret_hash: String::new(),
            base_url: None,
            redirect_uris: Vec::new(),
            logout_uris: Vec::new(),
            status: ServiceStatus::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Input for registering a new service
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateServiceInput {
    pub tenant_id: Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 255))]
    pub client_id: String,
    #[validate(url)]
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub logout_uris: Option<Vec<String>>,
}

/// Input for updating a service
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateServiceInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(url)]
    pub base_url: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    pub logout_uris: Option<Vec<String>>,
    pub status: Option<ServiceStatus>,
}

/// Service response (with generated secret for creation)
#[derive(Debug, Clone, Serialize)]
pub struct ServiceWithSecret {
    #[serde(flatten)]
    pub service: Service,
    /// Only present on creation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_default() {
        let service = Service::default();
        assert!(!service.id.is_nil());
        assert_eq!(service.status, ServiceStatus::Active);
    }

    #[test]
    fn test_service_serialization_hides_secret() {
        let service = Service {
            client_secret_hash: "secret-hash".to_string(),
            ..Default::default()
        };
        
        let json = serde_json::to_string(&service).unwrap();
        assert!(!json.contains("secret-hash"));
    }
}
