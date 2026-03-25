//! LDAP group-role mapping management APIs.

use crate::error::{AppError, Result};
use crate::http_support::SuccessResponse;
use crate::middleware::auth::AuthUser;
use crate::models::common::StringUuid;
use crate::models::ldap::{CreateLdapGroupRoleMappingInput, LdapGroupRoleMapping};
use crate::policy::{self, PolicyAction, PolicyInput, ResourceScope};
use crate::state::{HasDbPool, HasServices};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use uuid::Uuid;
use validator::Validate;

pub async fn list_mappings<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SuccessResponse<Vec<LdapGroupRoleMapping>>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;

    // Verify connector exists and belongs to tenant
    let _connector = super::tenant_sso::get_connector_by_id(
        state.db_pool(),
        tenant_id,
        StringUuid::from(connector_id),
    )
    .await?;

    let mappings = sqlx::query_as::<_, LdapGroupRoleMapping>(
        r#"
        SELECT id, tenant_id, connector_id, ldap_group_dn, ldap_group_display_name,
               role_id, created_at, updated_at
        FROM ldap_group_role_mappings
        WHERE connector_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(connector_id.to_string())
    .fetch_all(state.db_pool())
    .await?;

    Ok(Json(SuccessResponse::new(mappings)))
}

pub async fn create_mapping<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CreateLdapGroupRoleMappingInput>,
) -> Result<Json<SuccessResponse<LdapGroupRoleMapping>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;
    input.validate()?;

    // Verify connector exists, belongs to tenant, and is LDAP type
    let connector = super::tenant_sso::get_connector_by_id(
        state.db_pool(),
        tenant_id,
        StringUuid::from(connector_id),
    )
    .await?;

    if connector.provider_type != "ldap" {
        return Err(AppError::Validation(
            "LDAP group mappings are only supported for LDAP connectors".to_string(),
        ));
    }

    let id = StringUuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO ldap_group_role_mappings
            (id, tenant_id, connector_id, ldap_group_dn, ldap_group_display_name, role_id)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(tenant_id.to_string())
    .bind(connector_id.to_string())
    .bind(&input.ldap_group_dn)
    .bind(&input.ldap_group_display_name)
    .bind(input.role_id)
    .execute(state.db_pool())
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.code().as_deref() == Some("1062") {
                return AppError::Conflict(
                    "This LDAP group DN + role combination already exists".to_string(),
                );
            }
        }
        AppError::Database(e)
    })?;

    let mapping = sqlx::query_as::<_, LdapGroupRoleMapping>(
        r#"
        SELECT id, tenant_id, connector_id, ldap_group_dn, ldap_group_display_name,
               role_id, created_at, updated_at
        FROM ldap_group_role_mappings
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_one(state.db_pool())
    .await?;

    Ok(Json(SuccessResponse::new(mapping)))
}

pub async fn delete_mapping<S: HasServices + HasDbPool>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, _connector_id, mapping_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<crate::http_support::MessageResponse>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;

    let result = sqlx::query(
        "DELETE FROM ldap_group_role_mappings WHERE id = ? AND tenant_id = ?",
    )
    .bind(mapping_id.to_string())
    .bind(tenant_id.to_string())
    .execute(state.db_pool())
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(
            "LDAP group mapping not found".to_string(),
        ));
    }

    Ok(Json(crate::http_support::MessageResponse::new(
        "LDAP group mapping deleted.",
    )))
}

pub async fn search_ldap_users<S: HasServices + HasDbPool + crate::state::HasLdapAuth>(
    State(state): State<S>,
    auth: AuthUser,
    headers: HeaderMap,
    Path((tenant_id, connector_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<LdapSearchUsersInput>,
) -> Result<Json<SuccessResponse<Vec<LdapSearchUserResult>>>> {
    ensure_tenant_access(&state, &headers, &auth, tenant_id).await?;

    let connector = super::tenant_sso::get_connector_by_id(
        state.db_pool(),
        tenant_id,
        StringUuid::from(connector_id),
    )
    .await?;

    if connector.provider_type != "ldap" {
        return Err(AppError::Validation(
            "User search is only supported for LDAP connectors".to_string(),
        ));
    }

    let ldap_config = crate::models::ldap::parse_ldap_config(&connector.config)?;
    let limit = input.limit.unwrap_or(20).min(100);
    let profiles = state
        .ldap_authenticator()
        .search_users(&ldap_config, &input.query, limit)
        .await?;

    let results: Vec<LdapSearchUserResult> = profiles
        .into_iter()
        .map(|p| LdapSearchUserResult {
            dn: p.dn,
            username: p.username,
            email: p.email,
            display_name: p.display_name,
            groups: p.groups,
        })
        .collect();

    Ok(Json(SuccessResponse::new(results)))
}

#[derive(Debug, serde::Deserialize)]
pub struct LdapSearchUsersInput {
    pub query: String,
    pub limit: Option<u32>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LdapSearchUserResult {
    pub dn: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub groups: Vec<String>,
}

async fn ensure_tenant_access<S: HasServices>(
    state: &S,
    _headers: &HeaderMap,
    auth: &AuthUser,
    tenant_id: Uuid,
) -> Result<()> {
    policy::enforce_with_state(
        state,
        auth,
        &PolicyInput {
            action: PolicyAction::TenantSsoWrite,
            scope: ResourceScope::Tenant(StringUuid::from(tenant_id)),
        },
    )
    .await
}
