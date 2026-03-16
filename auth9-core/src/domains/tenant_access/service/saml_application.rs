//! SAML Application service — business logic for SAML IdP outbound

use crate::error::{AppError, Result};
use crate::keycloak::KeycloakClient;
use crate::keycloak::{KeycloakProtocolMapper, KeycloakSamlClient};
use crate::models::common::StringUuid;
use crate::models::saml_application::{
    validate_attribute_mappings, AttributeMapping, CreateSamlApplicationInput, NameIdFormat,
    SamlApplication, SamlApplicationResponse, UpdateSamlApplicationInput,
};
use crate::repository::saml_application::SamlApplicationRepository;
use std::collections::HashMap;
use std::sync::Arc;
use validator::Validate;

pub struct SamlApplicationService<R: SamlApplicationRepository> {
    repo: Arc<R>,
    keycloak: Arc<KeycloakClient>,
}

impl<R: SamlApplicationRepository> SamlApplicationService<R> {
    pub fn new(repo: Arc<R>, keycloak: Arc<KeycloakClient>) -> Self {
        Self { repo, keycloak }
    }

    /// Create a new SAML Application
    pub async fn create(
        &self,
        tenant_id: StringUuid,
        input: CreateSamlApplicationInput,
    ) -> Result<SamlApplicationResponse> {
        input.validate()?;
        validate_attribute_mappings(&input.attribute_mappings).map_err(|e| {
            AppError::Validation(
                e.message
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| e.code.to_string()),
            )
        })?;

        // Check entity_id uniqueness within tenant
        if self
            .repo
            .find_by_tenant_and_entity_id(tenant_id, &input.entity_id)
            .await?
            .is_some()
        {
            return Err(AppError::Conflict(format!(
                "SAML application with entity_id '{}' already exists in this tenant",
                input.entity_id
            )));
        }

        let name_id_format = input
            .name_id_format
            .as_ref()
            .unwrap_or(&NameIdFormat::Email);

        // Build Keycloak SAML Client
        let kc_client = build_keycloak_saml_client(
            &input.name,
            &input.entity_id,
            &input.acs_url,
            input.slo_url.as_deref(),
            name_id_format,
            input.sign_assertions,
            input.sign_responses,
            input.encrypt_assertions,
            input.sp_certificate.as_deref(),
            &input.attribute_mappings,
        );

        // Create in Keycloak
        let kc_client_uuid = self.keycloak.create_saml_client(&kc_client).await?;

        // Build domain model
        let app = SamlApplication {
            id: StringUuid::new_v4(),
            tenant_id,
            name: input.name,
            entity_id: input.entity_id,
            acs_url: input.acs_url,
            slo_url: input.slo_url,
            name_id_format: name_id_format.to_urn().to_string(),
            sign_assertions: input.sign_assertions,
            sign_responses: input.sign_responses,
            encrypt_assertions: input.encrypt_assertions,
            sp_certificate: input.sp_certificate,
            attribute_mappings: input.attribute_mappings,
            keycloak_client_id: kc_client_uuid,
            enabled: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created = self.repo.create(&app).await?;
        Ok(self.to_response(created))
    }

    /// Get a SAML Application by ID
    pub async fn get(
        &self,
        tenant_id: StringUuid,
        app_id: StringUuid,
    ) -> Result<SamlApplicationResponse> {
        let app = self.find_owned(tenant_id, app_id).await?;
        Ok(self.to_response(app))
    }

    /// List all SAML Applications for a tenant
    pub async fn list(&self, tenant_id: StringUuid) -> Result<Vec<SamlApplicationResponse>> {
        let apps = self.repo.list_by_tenant(tenant_id).await?;
        Ok(apps.into_iter().map(|a| self.to_response(a)).collect())
    }

    /// Update a SAML Application
    pub async fn update(
        &self,
        tenant_id: StringUuid,
        app_id: StringUuid,
        input: UpdateSamlApplicationInput,
    ) -> Result<SamlApplicationResponse> {
        input.validate()?;
        if let Some(ref mappings) = input.attribute_mappings {
            validate_attribute_mappings(mappings).map_err(|e| {
                AppError::Validation(
                    e.message
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| e.code.to_string()),
                )
            })?;
        }

        let existing = self.find_owned(tenant_id, app_id).await?;

        // Rebuild Keycloak client with merged values
        let name = input.name.as_ref().unwrap_or(&existing.name);
        let acs_url = input.acs_url.as_ref().unwrap_or(&existing.acs_url);
        let slo_url = input
            .slo_url
            .as_ref()
            .or(existing.slo_url.as_ref())
            .map(|s| s.as_str());
        let name_id_format = input
            .name_id_format
            .clone()
            .unwrap_or_else(|| NameIdFormat::from_str_flexible(&existing.name_id_format));
        let sign_assertions = input.sign_assertions.unwrap_or(existing.sign_assertions);
        let sign_responses = input.sign_responses.unwrap_or(existing.sign_responses);
        let encrypt_assertions = input
            .encrypt_assertions
            .unwrap_or(existing.encrypt_assertions);
        let sp_certificate = input
            .sp_certificate
            .as_ref()
            .or(existing.sp_certificate.as_ref())
            .map(|s| s.as_str());
        let attribute_mappings = input
            .attribute_mappings
            .as_ref()
            .unwrap_or(&existing.attribute_mappings);
        let enabled = input.enabled.unwrap_or(existing.enabled);

        let mut kc_client = build_keycloak_saml_client(
            name,
            &existing.entity_id,
            acs_url,
            slo_url,
            &name_id_format,
            sign_assertions,
            sign_responses,
            encrypt_assertions,
            sp_certificate,
            attribute_mappings,
        );
        kc_client.id = Some(existing.keycloak_client_id.clone());
        kc_client.enabled = enabled;

        self.keycloak
            .update_saml_client(&existing.keycloak_client_id, &kc_client)
            .await?;

        let updated = self.repo.update(app_id, &input).await?;
        Ok(self.to_response(updated))
    }

    /// Delete a SAML Application
    pub async fn delete(&self, tenant_id: StringUuid, app_id: StringUuid) -> Result<()> {
        let existing = self.find_owned(tenant_id, app_id).await?;

        // Delete from Keycloak first
        self.keycloak
            .delete_saml_client(&existing.keycloak_client_id)
            .await?;

        // Delete from database
        self.repo.delete(app_id).await?;

        Ok(())
    }

    /// Get IdP Metadata XML for a SAML Application
    ///
    /// Returns the realm-level SAML IdP descriptor. All SAML clients in the same
    /// realm share the same IdP signing key and SSO endpoint; the per-client
    /// difference is in the SP configuration, not the IdP metadata.
    pub async fn get_idp_metadata(
        &self,
        tenant_id: StringUuid,
        app_id: StringUuid,
    ) -> Result<String> {
        // Verify the app exists and belongs to the tenant
        let _app = self.find_owned(tenant_id, app_id).await?;
        self.keycloak.get_saml_idp_descriptor().await
    }

    /// Find an app and verify it belongs to the tenant
    async fn find_owned(
        &self,
        tenant_id: StringUuid,
        app_id: StringUuid,
    ) -> Result<SamlApplication> {
        let app =
            self.repo.find_by_id(app_id).await?.ok_or_else(|| {
                AppError::NotFound(format!("SAML application {} not found", app_id))
            })?;

        if app.tenant_id != tenant_id {
            return Err(AppError::NotFound(format!(
                "SAML application {} not found",
                app_id
            )));
        }

        Ok(app)
    }

    /// Convert domain model to API response with computed SSO URL
    fn to_response(&self, app: SamlApplication) -> SamlApplicationResponse {
        let sso_url = format!(
            "{}/realms/{}/protocol/saml",
            self.keycloak.public_url(),
            self.keycloak.realm()
        );
        SamlApplicationResponse { app, sso_url }
    }
}

/// Build a Keycloak SAML Client representation from Auth9 input
#[allow(clippy::too_many_arguments)]
fn build_keycloak_saml_client(
    name: &str,
    entity_id: &str,
    acs_url: &str,
    slo_url: Option<&str>,
    name_id_format: &NameIdFormat,
    sign_assertions: bool,
    sign_responses: bool,
    encrypt_assertions: bool,
    sp_certificate: Option<&str>,
    attribute_mappings: &[AttributeMapping],
) -> KeycloakSamlClient {
    let mut attributes = HashMap::new();

    // SAML signing/encryption settings
    attributes.insert(
        "saml.assertion.signature".to_string(),
        sign_assertions.to_string(),
    );
    attributes.insert(
        "saml.server.signature".to_string(),
        sign_responses.to_string(),
    );
    attributes.insert("saml.encrypt".to_string(), encrypt_assertions.to_string());

    // NameID format — Keycloak uses short names
    let kc_name_id = match name_id_format {
        NameIdFormat::Email => "email",
        NameIdFormat::Persistent => "persistent",
        NameIdFormat::Transient => "transient",
        NameIdFormat::Unspecified => "unspecified",
    };
    attributes.insert("saml_name_id_format".to_string(), kc_name_id.to_string());

    // SLO URL
    if let Some(slo) = slo_url {
        attributes.insert(
            "saml_single_logout_service_url_redirect".to_string(),
            slo.to_string(),
        );
    }

    // SP certificate for AuthnRequest signature verification and Assertion encryption
    if let Some(cert) = sp_certificate {
        attributes.insert("saml.signing.certificate".to_string(), cert.to_string());
        if encrypt_assertions {
            attributes.insert("saml.encryption.certificate".to_string(), cert.to_string());
        }
    }

    // Force POST binding for ACS (most common for SAML)
    attributes.insert("saml_force_post_binding".to_string(), "true".to_string());

    let protocol_mappers = build_protocol_mappers(attribute_mappings);

    KeycloakSamlClient {
        id: None,
        client_id: entity_id.to_string(),
        name: Some(name.to_string()),
        enabled: true,
        protocol: "saml".to_string(),
        base_url: None,
        redirect_uris: vec![acs_url.to_string()],
        attributes,
        protocol_mappers,
    }
}

/// Convert Auth9 attribute mappings to Keycloak SAML Protocol Mappers
fn build_protocol_mappers(mappings: &[AttributeMapping]) -> Vec<KeycloakProtocolMapper> {
    mappings
        .iter()
        .map(|m| {
            let mut config = HashMap::new();

            // Map source to Keycloak user attribute or property
            let (mapper_type, user_attr) = match m.source.as_str() {
                "email" => ("saml-user-property-idp-mapper", "email"),
                "first_name" => ("saml-user-property-idp-mapper", "firstName"),
                "last_name" => ("saml-user-property-idp-mapper", "lastName"),
                "user_id" => ("saml-user-property-idp-mapper", "id"),
                "display_name" => ("saml-user-attribute-idp-mapper", "displayName"),
                "tenant_roles" => ("saml-role-list-mapper", ""),
                "tenant_permissions" => ("saml-user-attribute-idp-mapper", "permissions"),
                _ => ("saml-user-attribute-idp-mapper", m.source.as_str()),
            };

            if mapper_type == "saml-role-list-mapper" {
                config.insert("single".to_string(), "false".to_string());
            } else {
                config.insert("user.attribute".to_string(), user_attr.to_string());
            }

            config.insert("attribute.name".to_string(), m.saml_attribute.clone());
            config.insert(
                "attribute.nameformat".to_string(),
                "URI Reference".to_string(),
            );

            if let Some(ref friendly) = m.friendly_name {
                config.insert("friendly.name".to_string(), friendly.clone());
            }

            KeycloakProtocolMapper {
                name: format!("auth9-{}", m.source),
                protocol: "saml".to_string(),
                protocol_mapper: mapper_type.to_string(),
                config,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::saml_application::AttributeMapping;
    use crate::repository::saml_application::MockSamlApplicationRepository;
    use chrono::Utc;
    use mockall::predicate::*;

    fn test_keycloak() -> Arc<KeycloakClient> {
        Arc::new(KeycloakClient::new(crate::config::KeycloakConfig {
            url: "http://localhost:8080".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
            webhook_secret: None,
        }))
    }

    fn make_test_app(tenant_id: StringUuid) -> SamlApplication {
        SamlApplication {
            id: StringUuid::new_v4(),
            tenant_id,
            name: "Test SP".to_string(),
            entity_id: "https://sp.test.com".to_string(),
            acs_url: "https://sp.test.com/acs".to_string(),
            slo_url: None,
            name_id_format: NameIdFormat::Email.to_urn().to_string(),
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
            keycloak_client_id: "kc-uuid-123".to_string(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_build_keycloak_saml_client_basic() {
        let kc = build_keycloak_saml_client(
            "My SP",
            "https://sp.example.com",
            "https://sp.example.com/acs",
            None,
            &NameIdFormat::Email,
            true,
            true,
            false,
            None,
            &[],
        );

        assert_eq!(kc.client_id, "https://sp.example.com");
        assert_eq!(kc.protocol, "saml");
        assert_eq!(kc.name, Some("My SP".to_string()));
        assert!(kc.enabled);
        assert_eq!(kc.redirect_uris, vec!["https://sp.example.com/acs"]);
        assert_eq!(
            kc.attributes.get("saml.assertion.signature"),
            Some(&"true".to_string())
        );
        assert_eq!(
            kc.attributes.get("saml_name_id_format"),
            Some(&"email".to_string())
        );
        assert!(kc.protocol_mappers.is_empty());
    }

    #[test]
    fn test_build_keycloak_saml_client_with_slo_and_cert() {
        let kc = build_keycloak_saml_client(
            "SP with SLO",
            "https://sp.example.com",
            "https://sp.example.com/acs",
            Some("https://sp.example.com/slo"),
            &NameIdFormat::Persistent,
            true,
            true,
            true,
            Some("MIIC...cert"),
            &[],
        );

        assert_eq!(
            kc.attributes.get("saml_single_logout_service_url_redirect"),
            Some(&"https://sp.example.com/slo".to_string())
        );
        assert_eq!(
            kc.attributes.get("saml_name_id_format"),
            Some(&"persistent".to_string())
        );
        assert_eq!(kc.attributes.get("saml.encrypt"), Some(&"true".to_string()));
        assert_eq!(
            kc.attributes.get("saml.signing.certificate"),
            Some(&"MIIC...cert".to_string())
        );
        assert_eq!(
            kc.attributes.get("saml.encryption.certificate"),
            Some(&"MIIC...cert".to_string())
        );
    }

    #[test]
    fn test_build_protocol_mappers_email() {
        let mappings = vec![AttributeMapping {
            source: "email".to_string(),
            saml_attribute: "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress"
                .to_string(),
            friendly_name: Some("email".to_string()),
        }];

        let mappers = build_protocol_mappers(&mappings);
        assert_eq!(mappers.len(), 1);
        assert_eq!(mappers[0].name, "auth9-email");
        assert_eq!(mappers[0].protocol, "saml");
        assert_eq!(mappers[0].protocol_mapper, "saml-user-property-idp-mapper");
        assert_eq!(
            mappers[0].config.get("user.attribute"),
            Some(&"email".to_string())
        );
        assert_eq!(
            mappers[0].config.get("friendly.name"),
            Some(&"email".to_string())
        );
    }

    #[test]
    fn test_build_protocol_mappers_roles() {
        let mappings = vec![AttributeMapping {
            source: "tenant_roles".to_string(),
            saml_attribute: "http://schemas.auth9.com/claims/roles".to_string(),
            friendly_name: Some("roles".to_string()),
        }];

        let mappers = build_protocol_mappers(&mappings);
        assert_eq!(mappers.len(), 1);
        assert_eq!(mappers[0].protocol_mapper, "saml-role-list-mapper");
        assert_eq!(mappers[0].config.get("single"), Some(&"false".to_string()));
    }

    #[test]
    fn test_build_protocol_mappers_multiple() {
        let mappings = vec![
            AttributeMapping {
                source: "email".to_string(),
                saml_attribute: "urn:oid:email".to_string(),
                friendly_name: None,
            },
            AttributeMapping {
                source: "first_name".to_string(),
                saml_attribute: "urn:oid:givenName".to_string(),
                friendly_name: Some("givenName".to_string()),
            },
            AttributeMapping {
                source: "last_name".to_string(),
                saml_attribute: "urn:oid:sn".to_string(),
                friendly_name: Some("sn".to_string()),
            },
        ];

        let mappers = build_protocol_mappers(&mappings);
        assert_eq!(mappers.len(), 3);
        assert_eq!(
            mappers[0].config.get("user.attribute"),
            Some(&"email".to_string())
        );
        assert_eq!(
            mappers[1].config.get("user.attribute"),
            Some(&"firstName".to_string())
        );
        assert_eq!(
            mappers[2].config.get("user.attribute"),
            Some(&"lastName".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_duplicate_entity_id() {
        let tenant_id = StringUuid::new_v4();
        let mut mock = MockSamlApplicationRepository::new();

        mock.expect_find_by_tenant_and_entity_id()
            .with(eq(tenant_id), eq("https://sp.test.com"))
            .returning(move |tid, _| Ok(Some(make_test_app(tid))));

        let keycloak = test_keycloak();

        let service = SamlApplicationService::new(Arc::new(mock), keycloak);

        let input = CreateSamlApplicationInput {
            name: "Duplicate SP".to_string(),
            entity_id: "https://sp.test.com".to_string(),
            acs_url: "https://sp.test.com/acs".to_string(),
            slo_url: None,
            name_id_format: None,
            sign_assertions: true,
            sign_responses: true,
            encrypt_assertions: false,
            sp_certificate: None,
            attribute_mappings: vec![],
        };

        let result = service.create(tenant_id, input).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Conflict(msg) => {
                assert!(msg.contains("already exists"));
            }
            other => panic!("Expected Conflict error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let tenant_id = StringUuid::new_v4();
        let app_id = StringUuid::new_v4();
        let mut mock = MockSamlApplicationRepository::new();

        mock.expect_find_by_id()
            .with(eq(app_id))
            .returning(|_| Ok(None));

        let keycloak = test_keycloak();

        let service = SamlApplicationService::new(Arc::new(mock), keycloak);

        let result = service.get(tenant_id, app_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_wrong_tenant() {
        let tenant_id = StringUuid::new_v4();
        let other_tenant = StringUuid::new_v4();
        let app = make_test_app(other_tenant);
        let app_id = app.id;

        let mut mock = MockSamlApplicationRepository::new();
        mock.expect_find_by_id()
            .with(eq(app_id))
            .returning(move |_| Ok(Some(make_test_app(other_tenant))));

        let keycloak = test_keycloak();

        let service = SamlApplicationService::new(Arc::new(mock), keycloak);

        let result = service.get(tenant_id, app_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_list_empty() {
        let tenant_id = StringUuid::new_v4();
        let mut mock = MockSamlApplicationRepository::new();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(|_| Ok(vec![]));

        let keycloak = test_keycloak();

        let service = SamlApplicationService::new(Arc::new(mock), keycloak);

        let result = service.list(tenant_id).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_list_returns_sso_url() {
        let tenant_id = StringUuid::new_v4();
        let mut mock = MockSamlApplicationRepository::new();

        mock.expect_list_by_tenant()
            .with(eq(tenant_id))
            .returning(move |tid| Ok(vec![make_test_app(tid)]));

        let keycloak = test_keycloak();

        let service = SamlApplicationService::new(Arc::new(mock), keycloak);

        let result = service.list(tenant_id).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].sso_url,
            "http://localhost:8081/realms/auth9/protocol/saml"
        );
    }
}
