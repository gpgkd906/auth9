//! Tenant-scoped enterprise SSO connector APIs.

use crate::api::{
    is_platform_admin_with_db, write_audit_log_generic, MessageResponse, SuccessResponse,
};
use crate::domain::{
    CreateEnterpriseSsoConnectorInput, EnterpriseSsoConnector, StringUuid,
    UpdateEnterpriseSsoConnectorInput,
};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakIdentityProvider;
use crate::middleware::auth::{AuthUser, TokenType};
use crate::state::{HasDbPool, HasServices};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::Serialize;
use sqlx::{MySqlPool, Row};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Serialize)]
pub struct ConnectorTestResult {
    pub ok: bool,
    pub message: String,
}

pub async fn list_connectors<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<SuccessResponse<Vec<EnterpriseSsoConnector>>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;
    let connectors = list_connectors_by_tenant(state.db_pool(), tenant_id).await?;
    Ok(Json(SuccessResponse::new(connectors)))
}

pub async fn create_connector<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path(tenant_id): Path<Uuid>,
    Json(input): Json<CreateEnterpriseSsoConnectorInput>,
) -> Result<Json<SuccessResponse<EnterpriseSsoConnector>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;
    input.validate()?;

    let provider_type = normalize_provider_type(&input.provider_type)?;
    let config = normalize_config(&provider_type, input.config);
    validate_required_config(&provider_type, &config)?;
    let domains = normalize_domains(input.domains)?;

    let tenant = state
        .tenant_service()
        .get(StringUuid::from(tenant_id))
        .await?;
    let alias = input.alias.trim().to_lowercase();
    let keycloak_alias = format!("{}--{}", tenant.slug, alias);

    let keycloak_provider = KeycloakIdentityProvider {
        alias: keycloak_alias.clone(),
        display_name: input.display_name.clone(),
        provider_id: provider_type.clone(),
        enabled: input.enabled,
        trust_email: false,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: config.clone(),
        extra: HashMap::new(),
    };
    state
        .keycloak_client()
        .create_identity_provider(&keycloak_provider)
        .await?;

    let connector_id = StringUuid::new_v4();
    let insert_result = insert_connector(
        state.db_pool(),
        connector_id,
        tenant_id,
        &alias,
        input.display_name.as_deref(),
        &provider_type,
        input.enabled,
        input.priority,
        &keycloak_alias,
        &config,
        &domains,
    )
    .await;

    if let Err(e) = insert_result {
        let _ = state
            .keycloak_client()
            .delete_identity_provider(&keycloak_alias)
            .await;
        return Err(e);
    }

    let created = get_connector_by_id(state.db_pool(), tenant_id, connector_id).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "tenant.sso.connector.create",
        "enterprise_sso_connector",
        Some(*created.id),
        None,
        serde_json::to_value(&created).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(created)))
}

pub async fn update_connector<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateEnterpriseSsoConnectorInput>,
) -> Result<Json<SuccessResponse<EnterpriseSsoConnector>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;
    input.validate()?;

    let before =
        get_connector_by_id(state.db_pool(), tenant_id, StringUuid::from(connector_id)).await?;
    let config = match input.config {
        Some(new_config) => {
            let normalized = normalize_config(&before.provider_type, new_config);
            validate_required_config(&before.provider_type, &normalized)?;
            normalized
        }
        None => before.config.clone(),
    };

    let updated_domains = input
        .domains
        .map(normalize_domains)
        .transpose()?
        .unwrap_or(before.domains.clone());
    let enabled = input.enabled.unwrap_or(before.enabled);
    let priority = input.priority.unwrap_or(before.priority);
    let display_name = input.display_name.or(before.display_name.clone());

    let keycloak_provider = KeycloakIdentityProvider {
        alias: before.keycloak_alias.clone(),
        display_name: display_name.clone(),
        provider_id: before.provider_type.clone(),
        enabled,
        trust_email: false,
        store_token: false,
        link_only: false,
        first_broker_login_flow_alias: None,
        config: config.clone(),
        extra: HashMap::new(),
    };
    state
        .keycloak_client()
        .update_identity_provider(&before.keycloak_alias, &keycloak_provider)
        .await?;

    let mut tx = state.db_pool().begin().await?;
    sqlx::query(
        r#"
        UPDATE enterprise_sso_connectors
        SET display_name = ?, enabled = ?, priority = ?, config = ?, updated_at = NOW()
        WHERE id = ? AND tenant_id = ?
        "#,
    )
    .bind(&display_name)
    .bind(enabled)
    .bind(priority)
    .bind(serde_json::to_string(&config).map_err(|e| AppError::Internal(e.into()))?)
    .bind(connector_id.to_string())
    .bind(tenant_id.to_string())
    .execute(tx.as_mut())
    .await?;

    sqlx::query("DELETE FROM enterprise_sso_domains WHERE connector_id = ?")
        .bind(connector_id.to_string())
        .execute(tx.as_mut())
        .await?;

    for (idx, domain) in updated_domains.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO enterprise_sso_domains
                (id, tenant_id, connector_id, domain, is_primary, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id.to_string())
        .bind(connector_id.to_string())
        .bind(domain)
        .bind(idx == 0)
        .execute(tx.as_mut())
        .await
        .map_err(map_conflict_if_duplicate)?;
    }
    tx.commit().await?;

    let after =
        get_connector_by_id(state.db_pool(), tenant_id, StringUuid::from(connector_id)).await?;
    let _ = write_audit_log_generic(
        &state,
        &headers,
        "tenant.sso.connector.update",
        "enterprise_sso_connector",
        Some(*after.id),
        serde_json::to_value(&before).ok(),
        serde_json::to_value(&after).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(after)))
}

pub async fn delete_connector<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MessageResponse>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;
    let before =
        get_connector_by_id(state.db_pool(), tenant_id, StringUuid::from(connector_id)).await?;

    state
        .keycloak_client()
        .delete_identity_provider(&before.keycloak_alias)
        .await?;

    let mut tx = state.db_pool().begin().await?;
    sqlx::query("DELETE FROM enterprise_sso_domains WHERE connector_id = ?")
        .bind(connector_id.to_string())
        .execute(tx.as_mut())
        .await?;
    sqlx::query("DELETE FROM enterprise_sso_connectors WHERE id = ? AND tenant_id = ?")
        .bind(connector_id.to_string())
        .bind(tenant_id.to_string())
        .execute(tx.as_mut())
        .await?;
    tx.commit().await?;

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "tenant.sso.connector.delete",
        "enterprise_sso_connector",
        Some(connector_id),
        serde_json::to_value(&before).ok(),
        None,
    )
    .await;

    Ok(Json(MessageResponse::new(
        "Connector deleted successfully.",
    )))
}

pub async fn test_connector<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SuccessResponse<ConnectorTestResult>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;
    let connector =
        get_connector_by_id(state.db_pool(), tenant_id, StringUuid::from(connector_id)).await?;

    let result = match state
        .keycloak_client()
        .get_identity_provider(&connector.keycloak_alias)
        .await
    {
        Ok(_) => ConnectorTestResult {
            ok: true,
            message: "Connector is available in Keycloak.".to_string(),
        },
        Err(err) => ConnectorTestResult {
            ok: false,
            message: format!("Connector check failed: {}", err),
        },
    };

    let _ = write_audit_log_generic(
        &state,
        &headers,
        "tenant.sso.connector.test",
        "enterprise_sso_connector",
        Some(connector_id),
        None,
        serde_json::to_value(&result).ok(),
    )
    .await;

    Ok(Json(SuccessResponse::new(result)))
}

pub async fn list_connectors_by_tenant(
    pool: &MySqlPool,
    tenant_id: Uuid,
) -> Result<Vec<EnterpriseSsoConnector>> {
    let connector_rows = sqlx::query(
        r#"
        SELECT id, tenant_id, alias, display_name, provider_type, enabled, priority,
               keycloak_alias, config, created_at, updated_at
        FROM enterprise_sso_connectors
        WHERE tenant_id = ?
        ORDER BY priority ASC, created_at ASC
        "#,
    )
    .bind(tenant_id.to_string())
    .fetch_all(pool)
    .await?;

    let domain_rows = sqlx::query(
        r#"
        SELECT connector_id, domain
        FROM enterprise_sso_domains
        WHERE tenant_id = ?
        ORDER BY is_primary DESC, domain ASC
        "#,
    )
    .bind(tenant_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut domains_by_connector: HashMap<StringUuid, Vec<String>> = HashMap::new();
    for row in domain_rows {
        let connector_id: StringUuid = row.try_get("connector_id")?;
        let domain: String = row.try_get("domain")?;
        domains_by_connector
            .entry(connector_id)
            .or_default()
            .push(domain);
    }

    let mut connectors = Vec::with_capacity(connector_rows.len());
    for row in connector_rows {
        let id: StringUuid = row.try_get("id")?;
        let config_value: serde_json::Value = row.try_get("config")?;
        let config = serde_json::from_value(config_value).unwrap_or_default();
        connectors.push(EnterpriseSsoConnector {
            id,
            tenant_id: row.try_get("tenant_id")?,
            alias: row.try_get("alias")?,
            display_name: row.try_get("display_name")?,
            provider_type: row.try_get("provider_type")?,
            enabled: row.try_get("enabled")?,
            priority: row.try_get("priority")?,
            keycloak_alias: row.try_get("keycloak_alias")?,
            config,
            domains: domains_by_connector.remove(&id).unwrap_or_default(),
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        });
    }

    Ok(connectors)
}

pub async fn get_connector_by_id(
    pool: &MySqlPool,
    tenant_id: Uuid,
    connector_id: StringUuid,
) -> Result<EnterpriseSsoConnector> {
    let connectors = list_connectors_by_tenant(pool, tenant_id).await?;
    connectors
        .into_iter()
        .find(|c| c.id == connector_id)
        .ok_or_else(|| AppError::NotFound("Enterprise SSO connector not found".to_string()))
}

#[allow(clippy::too_many_arguments)]
async fn insert_connector(
    pool: &MySqlPool,
    connector_id: StringUuid,
    tenant_id: Uuid,
    alias: &str,
    display_name: Option<&str>,
    provider_type: &str,
    enabled: bool,
    priority: i32,
    keycloak_alias: &str,
    config: &HashMap<String, String>,
    domains: &[String],
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO enterprise_sso_connectors
            (id, tenant_id, alias, display_name, provider_type, enabled, priority, keycloak_alias, config, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NOW(), NOW())
        "#,
    )
    .bind(connector_id.to_string())
    .bind(tenant_id.to_string())
    .bind(alias)
    .bind(display_name)
    .bind(provider_type)
    .bind(enabled)
    .bind(priority)
    .bind(keycloak_alias)
    .bind(serde_json::to_string(config).map_err(|e| AppError::Internal(e.into()))?)
    .execute(tx.as_mut())
    .await
    .map_err(map_conflict_if_duplicate)?;

    for (idx, domain) in domains.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO enterprise_sso_domains
                (id, tenant_id, connector_id, domain, is_primary, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, NOW(), NOW())
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(tenant_id.to_string())
        .bind(connector_id.to_string())
        .bind(domain)
        .bind(idx == 0)
        .execute(tx.as_mut())
        .await
        .map_err(map_conflict_if_duplicate)?;
    }

    tx.commit().await?;
    Ok(())
}

async fn ensure_tenant_access<S: HasServices>(
    state: &S,
    _headers: &HeaderMap,
    auth: &AuthUser,
    tenant_id: Uuid,
) -> Result<()> {
    match auth.token_type {
        TokenType::TenantAccess | TokenType::ServiceClient => {
            if auth.tenant_id == Some(tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Access denied: you don't have permission to access this tenant".to_string(),
                ))
            }
        }
        TokenType::Identity => {
            if is_platform_admin_with_db(state, auth).await {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Tenant-scoped token required (exchange identity token first)".to_string(),
                ))
            }
        }
    }
}

fn normalize_provider_type(provider_type: &str) -> Result<String> {
    let normalized = provider_type.trim().to_lowercase();
    if normalized == "saml" || normalized == "oidc" {
        Ok(normalized)
    } else {
        Err(AppError::Validation(
            "provider_type must be one of: saml, oidc".to_string(),
        ))
    }
}

fn normalize_config(
    provider_type: &str,
    mut config: HashMap<String, String>,
) -> HashMap<String, String> {
    if provider_type == "saml" {
        if let Some(sso_url) = config.remove("ssoUrl") {
            config
                .entry("singleSignOnServiceUrl".to_string())
                .or_insert(sso_url);
        }
        if let Some(cert) = config.remove("certificate") {
            config
                .entry("signingCertificate".to_string())
                .or_insert(cert);
        }
    }
    config
}

fn validate_required_config(provider_type: &str, config: &HashMap<String, String>) -> Result<()> {
    let required: &[&str] = match provider_type {
        "saml" => &["entityId", "singleSignOnServiceUrl", "signingCertificate"],
        "oidc" => &["clientId", "clientSecret", "authorizationUrl", "tokenUrl"],
        _ => &[],
    };
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|k| config.get(*k).is_none_or(|v| v.trim().is_empty()))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "Missing required config fields: {}",
            missing.join(", ")
        )))
    }
}

fn normalize_domains(domains: Vec<String>) -> Result<Vec<String>> {
    if domains.is_empty() {
        return Err(AppError::Validation(
            "At least one enterprise domain is required".to_string(),
        ));
    }
    let mut dedup = HashSet::new();
    let mut normalized = Vec::new();
    for domain in domains {
        let d = domain.trim().to_lowercase();
        if d.is_empty() {
            continue;
        }
        if !d.contains('.') || d.contains(' ') || d.contains('@') {
            return Err(AppError::Validation(format!("Invalid domain: {}", d)));
        }
        if dedup.insert(d.clone()) {
            normalized.push(d);
        }
    }
    if normalized.is_empty() {
        return Err(AppError::Validation(
            "At least one valid enterprise domain is required".to_string(),
        ));
    }
    Ok(normalized)
}

fn map_conflict_if_duplicate(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &error {
        if db_err.code().as_deref() == Some("1062") {
            return AppError::Conflict("Duplicate connector alias or domain".to_string());
        }
    }
    AppError::Database(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // =========================================================================
    // normalize_provider_type
    // =========================================================================

    #[test]
    fn normalize_provider_type_saml_lowercase() {
        assert_eq!(normalize_provider_type("saml").unwrap(), "saml");
    }

    #[test]
    fn normalize_provider_type_saml_uppercase() {
        assert_eq!(normalize_provider_type("SAML").unwrap(), "saml");
    }

    #[test]
    fn normalize_provider_type_saml_mixed_case() {
        assert_eq!(normalize_provider_type("Saml").unwrap(), "saml");
    }

    #[test]
    fn normalize_provider_type_oidc_lowercase() {
        assert_eq!(normalize_provider_type("oidc").unwrap(), "oidc");
    }

    #[test]
    fn normalize_provider_type_oidc_uppercase() {
        assert_eq!(normalize_provider_type("OIDC").unwrap(), "oidc");
    }

    #[test]
    fn normalize_provider_type_with_whitespace() {
        assert_eq!(normalize_provider_type("  saml  ").unwrap(), "saml");
    }

    #[test]
    fn normalize_provider_type_invalid() {
        let err = normalize_provider_type("ldap").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn normalize_provider_type_empty() {
        let err = normalize_provider_type("").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    // =========================================================================
    // normalize_config
    // =========================================================================

    #[test]
    fn normalize_config_saml_renames_sso_url() {
        let config = HashMap::from([
            ("ssoUrl".to_string(), "https://idp.example.com/sso".to_string()),
        ]);
        let result = normalize_config("saml", config);
        assert_eq!(
            result.get("singleSignOnServiceUrl").unwrap(),
            "https://idp.example.com/sso"
        );
        assert!(!result.contains_key("ssoUrl"));
    }

    #[test]
    fn normalize_config_saml_renames_certificate() {
        let config = HashMap::from([
            ("certificate".to_string(), "MIID...".to_string()),
        ]);
        let result = normalize_config("saml", config);
        assert_eq!(result.get("signingCertificate").unwrap(), "MIID...");
        assert!(!result.contains_key("certificate"));
    }

    #[test]
    fn normalize_config_saml_does_not_overwrite_existing_keys() {
        let config = HashMap::from([
            ("ssoUrl".to_string(), "https://old.example.com".to_string()),
            ("singleSignOnServiceUrl".to_string(), "https://existing.example.com".to_string()),
        ]);
        let result = normalize_config("saml", config);
        // existing key should NOT be overwritten
        assert_eq!(
            result.get("singleSignOnServiceUrl").unwrap(),
            "https://existing.example.com"
        );
    }

    #[test]
    fn normalize_config_oidc_no_renames() {
        let config = HashMap::from([
            ("ssoUrl".to_string(), "https://idp.example.com".to_string()),
            ("certificate".to_string(), "cert".to_string()),
        ]);
        let result = normalize_config("oidc", config);
        // OIDC should not rename anything
        assert!(result.contains_key("ssoUrl"));
        assert!(result.contains_key("certificate"));
        assert!(!result.contains_key("singleSignOnServiceUrl"));
    }

    #[test]
    fn normalize_config_saml_both_renames() {
        let config = HashMap::from([
            ("ssoUrl".to_string(), "https://idp.example.com/sso".to_string()),
            ("certificate".to_string(), "MIID...".to_string()),
            ("entityId".to_string(), "https://sp.example.com".to_string()),
        ]);
        let result = normalize_config("saml", config);
        assert!(result.contains_key("singleSignOnServiceUrl"));
        assert!(result.contains_key("signingCertificate"));
        assert!(result.contains_key("entityId"));
        assert!(!result.contains_key("ssoUrl"));
        assert!(!result.contains_key("certificate"));
    }

    #[test]
    fn normalize_config_empty_map() {
        let result = normalize_config("saml", HashMap::new());
        assert!(result.is_empty());
    }

    // =========================================================================
    // validate_required_config
    // =========================================================================

    #[test]
    fn validate_required_config_saml_all_present() {
        let config = HashMap::from([
            ("entityId".to_string(), "https://sp.example.com".to_string()),
            ("singleSignOnServiceUrl".to_string(), "https://idp.example.com/sso".to_string()),
            ("signingCertificate".to_string(), "MIID...".to_string()),
        ]);
        assert!(validate_required_config("saml", &config).is_ok());
    }

    #[test]
    fn validate_required_config_saml_missing_entity_id() {
        let config = HashMap::from([
            ("singleSignOnServiceUrl".to_string(), "https://idp.example.com/sso".to_string()),
            ("signingCertificate".to_string(), "MIID...".to_string()),
        ]);
        let err = validate_required_config("saml", &config).unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("entityId")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn validate_required_config_saml_missing_sso_url() {
        let config = HashMap::from([
            ("entityId".to_string(), "https://sp.example.com".to_string()),
            ("signingCertificate".to_string(), "MIID...".to_string()),
        ]);
        let err = validate_required_config("saml", &config).unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("singleSignOnServiceUrl")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn validate_required_config_saml_empty_value_treated_as_missing() {
        let config = HashMap::from([
            ("entityId".to_string(), "https://sp.example.com".to_string()),
            ("singleSignOnServiceUrl".to_string(), "   ".to_string()),
            ("signingCertificate".to_string(), "MIID...".to_string()),
        ]);
        let err = validate_required_config("saml", &config).unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("singleSignOnServiceUrl")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn validate_required_config_saml_all_missing() {
        let err = validate_required_config("saml", &HashMap::new()).unwrap_err();
        match err {
            AppError::Validation(msg) => {
                assert!(msg.contains("entityId"));
                assert!(msg.contains("singleSignOnServiceUrl"));
                assert!(msg.contains("signingCertificate"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn validate_required_config_oidc_all_present() {
        let config = HashMap::from([
            ("clientId".to_string(), "my-client".to_string()),
            ("clientSecret".to_string(), "secret".to_string()),
            ("authorizationUrl".to_string(), "https://idp.example.com/auth".to_string()),
            ("tokenUrl".to_string(), "https://idp.example.com/token".to_string()),
        ]);
        assert!(validate_required_config("oidc", &config).is_ok());
    }

    #[test]
    fn validate_required_config_oidc_missing_fields() {
        let config = HashMap::from([
            ("clientId".to_string(), "my-client".to_string()),
        ]);
        let err = validate_required_config("oidc", &config).unwrap_err();
        match err {
            AppError::Validation(msg) => {
                assert!(msg.contains("clientSecret"));
                assert!(msg.contains("authorizationUrl"));
                assert!(msg.contains("tokenUrl"));
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn validate_required_config_unknown_type_no_requirements() {
        assert!(validate_required_config("ldap", &HashMap::new()).is_ok());
    }

    // =========================================================================
    // normalize_domains
    // =========================================================================

    #[test]
    fn normalize_domains_valid_single() {
        let result = normalize_domains(vec!["example.com".to_string()]).unwrap();
        assert_eq!(result, vec!["example.com"]);
    }

    #[test]
    fn normalize_domains_valid_multiple() {
        let result = normalize_domains(vec![
            "example.com".to_string(),
            "acme.org".to_string(),
        ])
        .unwrap();
        assert_eq!(result, vec!["example.com", "acme.org"]);
    }

    #[test]
    fn normalize_domains_trims_and_lowercases() {
        let result = normalize_domains(vec!["  Example.COM  ".to_string()]).unwrap();
        assert_eq!(result, vec!["example.com"]);
    }

    #[test]
    fn normalize_domains_deduplicates() {
        let result = normalize_domains(vec![
            "example.com".to_string(),
            "EXAMPLE.COM".to_string(),
            "example.com".to_string(),
        ])
        .unwrap();
        assert_eq!(result, vec!["example.com"]);
    }

    #[test]
    fn normalize_domains_empty_list_fails() {
        let err = normalize_domains(vec![]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn normalize_domains_all_blank_entries_fails() {
        let err = normalize_domains(vec!["  ".to_string(), "".to_string()]).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn normalize_domains_no_dot_fails() {
        let err = normalize_domains(vec!["localhost".to_string()]).unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("Invalid domain")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn normalize_domains_contains_space_fails() {
        let err = normalize_domains(vec!["ex ample.com".to_string()]).unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("Invalid domain")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn normalize_domains_contains_at_sign_fails() {
        let err = normalize_domains(vec!["user@example.com".to_string()]).unwrap_err();
        match err {
            AppError::Validation(msg) => assert!(msg.contains("Invalid domain")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn normalize_domains_mixed_valid_and_blank() {
        let result = normalize_domains(vec![
            "  ".to_string(),
            "example.com".to_string(),
            "".to_string(),
        ])
        .unwrap();
        assert_eq!(result, vec!["example.com"]);
    }

    // =========================================================================
    // ConnectorTestResult serialization
    // =========================================================================

    #[test]
    fn connector_test_result_serialization() {
        let result = ConnectorTestResult {
            ok: true,
            message: "All good".to_string(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["message"], "All good");
    }

    #[test]
    fn connector_test_result_failure_serialization() {
        let result = ConnectorTestResult {
            ok: false,
            message: "Connection refused".to_string(),
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["message"], "Connection refused");
    }
}
