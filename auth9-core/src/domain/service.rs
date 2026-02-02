//! Service/Client domain model

use super::common::StringUuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use url::Url;
use uuid::Uuid;
use validator::Validate;

/// Validate a single redirect URI.
/// HTTP is only allowed for localhost/127.0.0.1, all other hosts must use HTTPS.
fn validate_single_redirect_uri(uri: &str) -> Result<(), validator::ValidationError> {
    let parsed = Url::parse(uri).map_err(|_| {
        let mut err = validator::ValidationError::new("invalid_url");
        err.message = Some(format!("Invalid URL: {}", uri).into());
        err
    })?;

    let scheme = parsed.scheme();
    let host = parsed.host_str().unwrap_or("");

    // Allow HTTP only for localhost/127.0.0.1
    if scheme == "http" {
        let is_localhost = host == "localhost" || host == "127.0.0.1" || host == "::1";
        if !is_localhost {
            let mut err = validator::ValidationError::new("insecure_redirect_uri");
            err.message = Some(
                format!(
                    "HTTP is only allowed for localhost. Use HTTPS for: {}",
                    uri
                )
                .into(),
            );
            return Err(err);
        }
    } else if scheme != "https" {
        let mut err = validator::ValidationError::new("invalid_scheme");
        err.message = Some(format!("Only HTTP and HTTPS schemes are allowed: {}", uri).into());
        return Err(err);
    }
    Ok(())
}

/// Validate redirect URIs.
/// Note: For Option<Vec<String>> fields, validator automatically unwraps the Option.
fn validate_redirect_uris(uris: &[String]) -> Result<(), validator::ValidationError> {
    for uri in uris {
        validate_single_redirect_uri(uri)?;
    }
    Ok(())
}

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
    fn encode_by_ref(
        &self,
        buf: &mut Vec<u8>,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
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
    #[validate(custom(function = "validate_redirect_uris"))]
    pub redirect_uris: Vec<String>,
    #[validate(custom(function = "validate_redirect_uris"))]
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
    #[validate(custom(function = "validate_redirect_uris"))]
    pub redirect_uris: Option<Vec<String>>,
    #[validate(custom(function = "validate_redirect_uris"))]
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
    use validator::Validate;

    #[test]
    fn test_service_default() {
        let service = Service::default();
        assert!(!service.id.is_nil());
        assert_eq!(service.status, ServiceStatus::Active);
        assert!(service.tenant_id.is_none());
        assert!(service.name.is_empty());
        assert!(service.base_url.is_none());
        assert!(service.redirect_uris.is_empty());
        assert!(service.logout_uris.is_empty());
    }

    #[test]
    fn test_service_with_values() {
        let tenant_id = StringUuid::new_v4();
        let service = Service {
            tenant_id: Some(tenant_id),
            name: "My Service".to_string(),
            base_url: Some("https://example.com".to_string()),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            logout_uris: vec!["https://example.com/logout".to_string()],
            ..Default::default()
        };

        assert_eq!(service.tenant_id, Some(tenant_id));
        assert_eq!(service.name, "My Service");
        assert!(service.base_url.is_some());
        assert_eq!(service.redirect_uris.len(), 1);
        assert_eq!(service.logout_uris.len(), 1);
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
    fn test_service_deserialization() {
        let service = Service {
            name: "Test Service".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_string(&service).unwrap();
        let deserialized: Service = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "Test Service");
        assert_eq!(deserialized.status, ServiceStatus::Active);
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
    fn test_client_without_name() {
        let client = Client {
            id: StringUuid::new_v4(),
            service_id: StringUuid::new_v4(),
            client_id: "test-client".to_string(),
            client_secret_hash: "secret-hash".to_string(),
            name: None,
            created_at: Utc::now(),
        };

        assert!(client.name.is_none());
    }

    #[test]
    fn test_service_status_default() {
        let status = ServiceStatus::default();
        assert_eq!(status, ServiceStatus::Active);
    }

    #[test]
    fn test_service_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&ServiceStatus::Inactive).unwrap(),
            "\"inactive\""
        );
    }

    #[test]
    fn test_service_status_deserialization() {
        let active: ServiceStatus = serde_json::from_str("\"active\"").unwrap();
        let inactive: ServiceStatus = serde_json::from_str("\"inactive\"").unwrap();

        assert_eq!(active, ServiceStatus::Active);
        assert_eq!(inactive, ServiceStatus::Inactive);
    }

    #[test]
    fn test_create_service_input_valid() {
        let input = CreateServiceInput {
            tenant_id: Some(Uuid::new_v4()),
            name: "My Service".to_string(),
            client_id: "my-client-id".to_string(),
            base_url: Some("https://example.com".to_string()),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            logout_uris: Some(vec!["https://example.com/logout".to_string()]),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_service_input_minimal() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "S".to_string(),
            client_id: "c".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_service_input_empty_name() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "".to_string(),
            client_id: "valid-client".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_service_input_empty_client_id() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "Valid Name".to_string(),
            client_id: "".to_string(),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_service_input_invalid_base_url() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "Valid Name".to_string(),
            client_id: "valid-client".to_string(),
            base_url: Some("not-a-url".to_string()),
            redirect_uris: vec![],
            logout_uris: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_create_client_input_valid() {
        let input = CreateClientInput {
            name: Some("My Client".to_string()),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_client_input_no_name() {
        let input = CreateClientInput { name: None };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_client_input_empty_name() {
        let input = CreateClientInput {
            name: Some("".to_string()),
        };

        // Empty string is within length bounds (0 is not < 1 for Some values)
        // Actually, the validation is min = 1, so empty string should fail
        // But since name is Option<String>, the validation only applies when Some
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_service_input_valid() {
        let input = UpdateServiceInput {
            name: Some("Updated Name".to_string()),
            base_url: Some("https://updated.example.com".to_string()),
            redirect_uris: Some(vec!["https://new-callback.com".to_string()]),
            logout_uris: Some(vec!["https://new-logout.com".to_string()]),
            status: Some(ServiceStatus::Inactive),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_service_input_partial() {
        let input = UpdateServiceInput {
            name: None,
            base_url: None,
            redirect_uris: None,
            logout_uris: None,
            status: Some(ServiceStatus::Inactive),
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_update_service_input_empty_name() {
        let input = UpdateServiceInput {
            name: Some("".to_string()),
            base_url: None,
            redirect_uris: None,
            logout_uris: None,
            status: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_service_input_invalid_base_url() {
        let input = UpdateServiceInput {
            name: None,
            base_url: Some("invalid-url".to_string()),
            redirect_uris: None,
            logout_uris: None,
            status: None,
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_service_with_client_structure() {
        let service = Service::default();
        let client = Client {
            id: StringUuid::new_v4(),
            service_id: service.id,
            client_id: "test-client".to_string(),
            client_secret_hash: "hash".to_string(),
            name: None,
            created_at: Utc::now(),
        };
        let client_secret = "secret123".to_string();

        let swc = ServiceWithClient {
            service: service.clone(),
            client: ClientWithSecret {
                client,
                client_secret: client_secret.clone(),
            },
        };

        assert_eq!(swc.service.id, service.id);
        assert_eq!(swc.client.client_secret, client_secret);
    }

    #[test]
    fn test_service_with_client_serialization() {
        let service = Service {
            name: "Test".to_string(),
            ..Default::default()
        };
        let client = Client {
            id: StringUuid::new_v4(),
            service_id: service.id,
            client_id: "test-client".to_string(),
            client_secret_hash: "hash".to_string(),
            name: None,
            created_at: Utc::now(),
        };

        let swc = ServiceWithClient {
            service,
            client: ClientWithSecret {
                client,
                client_secret: "secret123".to_string(),
            },
        };

        let json = serde_json::to_string(&swc).unwrap();
        // Secret should be visible in ClientWithSecret
        assert!(json.contains("secret123"));
        // But hash should not be visible
        assert!(!json.contains("hash"));
    }

    #[test]
    fn test_client_with_secret_structure() {
        let client = Client {
            id: StringUuid::new_v4(),
            service_id: StringUuid::new_v4(),
            client_id: "my-client".to_string(),
            client_secret_hash: "hashed".to_string(),
            name: Some("My Client".to_string()),
            created_at: Utc::now(),
        };

        let cws = ClientWithSecret {
            client: client.clone(),
            client_secret: "plain-secret".to_string(),
        };

        assert_eq!(cws.client.client_id, "my-client");
        assert_eq!(cws.client_secret, "plain-secret");
    }

    #[test]
    fn test_service_status_equality() {
        assert_eq!(ServiceStatus::Active, ServiceStatus::Active);
        assert_eq!(ServiceStatus::Inactive, ServiceStatus::Inactive);
        assert_ne!(ServiceStatus::Active, ServiceStatus::Inactive);
    }

    #[test]
    fn test_redirect_uri_https_valid() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "Test".to_string(),
            client_id: "test".to_string(),
            base_url: None,
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
            logout_uris: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_redirect_uri_http_localhost_valid() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "Test".to_string(),
            client_id: "test".to_string(),
            base_url: None,
            redirect_uris: vec![
                "http://localhost:3000/callback".to_string(),
                "http://127.0.0.1:8080/callback".to_string(),
            ],
            logout_uris: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_redirect_uri_http_non_localhost_invalid() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "Test".to_string(),
            client_id: "test".to_string(),
            base_url: None,
            redirect_uris: vec!["http://app.example.com/callback".to_string()],
            logout_uris: None,
        };
        let result = input.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        let field_errors = errors.field_errors();
        assert!(field_errors.contains_key("redirect_uris"));
    }

    #[test]
    fn test_redirect_uri_invalid_url() {
        let input = CreateServiceInput {
            tenant_id: None,
            name: "Test".to_string(),
            client_id: "test".to_string(),
            base_url: None,
            redirect_uris: vec!["not-a-valid-url".to_string()],
            logout_uris: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_update_redirect_uri_validation() {
        let input = UpdateServiceInput {
            name: None,
            base_url: None,
            redirect_uris: Some(vec!["http://evil.com/callback".to_string()]),
            logout_uris: None,
            status: None,
        };
        assert!(input.validate().is_err());
    }
}
