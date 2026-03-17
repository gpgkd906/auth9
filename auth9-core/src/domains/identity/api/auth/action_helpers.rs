//! Action-related helper functions for resolving service/tenant context.

use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use crate::models::enterprise_sso::EnterpriseSsoDiscoveryResult;
use crate::state::HasServices;
use sqlx::Row;

/// Resolve service_id and tenant_id for action execution.
/// Returns (service_id, tenant_id) -- both optional.
pub(super) async fn resolve_service_ids_for_actions<S: HasServices>(
    state: &S,
    client_id: &str,
) -> (Option<StringUuid>, Option<StringUuid>) {
    match state.client_service().get_by_client_id(client_id).await {
        Ok(service) => (Some(service.id), service.tenant_id),
        Err(e) => {
            tracing::warn!(
                "Failed to look up service for client_id '{}': {}, skipping actions",
                client_id,
                e
            );
            (None, None)
        }
    }
}

/// Resolve (service_id, tenant_id) for action execution at post-login.
/// Falls back to the user's first tenant membership if no service-level tenant found.
pub(super) async fn resolve_action_ids<S: HasServices>(
    state: &S,
    client_id: &str,
    user_id: StringUuid,
    service_id: Option<StringUuid>,
    service_tenant_id: Option<StringUuid>,
) -> (Option<StringUuid>, Option<StringUuid>) {
    if let (Some(sid), Some(tid)) = (service_id, service_tenant_id) {
        return (Some(sid), Some(tid));
    }

    match state.user_service().get_user_tenants(user_id).await {
        Ok(tenants) => {
            let active = tenants.into_iter().next();
            if active.is_none() {
                tracing::debug!(
                    "Service '{}' is cross-tenant and user {} has no tenant memberships, skipping actions",
                    client_id,
                    user_id
                );
            }
            (service_id, active.map(|tu| tu.tenant_id))
        }
        Err(e) => {
            tracing::warn!(
                "Failed to look up tenants for user {}: {}, skipping actions",
                user_id,
                e
            );
            (service_id, None)
        }
    }
}

pub(super) async fn resolve_action_tenant_profile<S: HasServices>(
    state: &S,
    tenant_id: StringUuid,
) -> (String, String) {
    match state.tenant_service().get(tenant_id).await {
        Ok(tenant) => (tenant.slug, tenant.name),
        Err(_) => (String::new(), String::new()),
    }
}

pub(super) async fn discover_connector_by_domain(
    pool: &sqlx::MySqlPool,
    domain: &str,
) -> Result<EnterpriseSsoDiscoveryResult> {
    let row = sqlx::query(
        r#"
        SELECT c.tenant_id, t.slug as tenant_slug, c.alias as connector_alias,
               COALESCE(c.provider_alias, c.keycloak_alias) AS provider_alias, c.provider_type
        FROM enterprise_sso_domains d
        INNER JOIN enterprise_sso_connectors c ON c.id = d.connector_id
        INNER JOIN tenants t ON t.id = c.tenant_id
        WHERE d.domain = ? AND c.enabled = TRUE
        LIMIT 1
        "#,
    )
    .bind(domain.to_lowercase())
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| {
        AppError::NotFound(format!(
            "No enterprise SSO connector configured for domain '{}'",
            domain
        ))
    })?;

    Ok(EnterpriseSsoDiscoveryResult {
        tenant_id: row.try_get("tenant_id")?,
        tenant_slug: row.try_get("tenant_slug")?,
        connector_alias: row.try_get("connector_alias")?,
        provider_alias: row.try_get("provider_alias")?,
        provider_type: row.try_get("provider_type")?,
    })
}
