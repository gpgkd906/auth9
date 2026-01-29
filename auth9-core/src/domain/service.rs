//! Service/Client domain model

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Service status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    #[default]
    Active,
    Inactive,
}

impl sqlx::Type<sqlx::MySql> for ServiceStatus {
    fn type_info() -> sqlx::mysql::MySqlTypeInfo {
        <String as sqlx::Type<sqlx::MySql>>::type_info()
    }

    fn compatible(ty: &sqlx::mysql::MySqlTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::MySql>>::compatible(ty)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::MySql> for ServiceStatus {
    fn decode(value: sqlx::mysql::MySqlValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::MySql>>::decode(value)?;
        match s.to_lowercase().as_str() {
            "active" => Ok(ServiceStatus::Active),
            "inactive" => Ok(ServiceStatus::Inactive),
            _ => Err(format!("Unknown service status: {}", s).into()),
        }
    }
}

impl<'q> sqlx::Encode<'q, sqlx::MySql> for ServiceStatus {
    fn encode_by_ref(&self, buf: &mut Vec<u8>) -> sqlx::encode::IsNull {
        let s = match self {
            ServiceStatus::Active => "active",
            ServiceStatus::Inactive => "inactive",
        };
        <&str as sqlx::Encode<sqlx::MySql>>::encode_by_ref(&s, buf)
    }
}

/// Service entity (OIDC client container)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Service {
    pub id: StringUuid,
    pub tenant_id: Option<StringUuid>,
    pub name: String,
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
            id: StringUuid::new_v4(),
            tenant_id: None,
            name: String::new(),
            base_url: None,
            redirect_uris: Vec::new(),
            logout_uris: Vec::new(),
            status: ServiceStatus::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Client entity (OIDC credentials)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Client {
    pub id: StringUuid,
    pub service_id: StringUuid,
    pub client_id: String,
    /// Hashed client secret (never expose raw secret)
    #[serde(skip_serializing)]
    pub client_secret_hash: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Input for registering a new service
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateServiceInput {
    pub tenant_id: Option<Uuid>,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 255))]
    pub client_id: String, // Keep for initial client creation
    #[validate(url)]
    pub base_url: Option<String>,
    pub redirect_uris: Vec<String>,
    pub logout_uris: Option<Vec<String>>,
}

/// Input for creating a new client
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateClientInput {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
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

/// Service response with initial client
#[derive(Debug, Clone, Serialize)]
pub struct ServiceWithClient {
    #[serde(flatten)]
    pub service: Service,
    pub client: ClientWithSecret,
}

/// Client response with generated secret
#[derive(Debug, Clone, Serialize)]
pub struct ClientWithSecret {
    #[serde(flatten)]
    pub client: Client,
    pub client_secret: String,
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
    fn test_service_serialization() {
        let service = Service {
            name: "Test Service".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_string(&service).unwrap();
        assert!(json.contains("Test Service"));
        assert!(json.contains("active"));
    }

    #[test]
    fn test_client_serialization_hides_secret() {
        let client = Client {
            id: StringUuid::new_v4(),
            service_id: StringUuid::new_v4(),
            client_id: "test-client".to_string(),
            client_secret_hash: "secret-hash".to_string(),
            name: Some("Test Client".to_string()),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&client).unwrap();
        // client_secret_hash is marked with #[serde(skip_serializing)]
        assert!(!json.contains("secret-hash"));
        assert!(json.contains("test-client"));
    }

    #[test]
    fn test_service_status_serialization() {
        assert_eq!(serde_json::to_string(&ServiceStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&ServiceStatus::Inactive).unwrap(), "\"inactive\"");
    }

    #[test]
    fn test_service_status_deserialization() {
        let active: ServiceStatus = serde_json::from_str("\"active\"").unwrap();
        let inactive: ServiceStatus = serde_json::from_str("\"inactive\"").unwrap();
        
        assert_eq!(active, ServiceStatus::Active);
        assert_eq!(inactive, ServiceStatus::Inactive);
    }
}
