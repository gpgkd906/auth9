//! Shared types and helpers for enterprise SSO brokers (OIDC + SAML).

use crate::cache::CacheOperations;
use crate::domains::identity::api::auth::helpers::{
    AuthorizationCodeData, LoginChallengeData, AUTH_CODE_TTL_SECS,
};
use crate::error::{AppError, Result};
use crate::models::linked_identity::{CreateLinkedIdentityInput, FirstLoginPolicy, PendingMergeData};
use crate::models::user::AddUserToTenantInput;
use crate::state::{HasCache, HasIdentityProviders, HasServices};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use url::Url;

/// Enterprise SSO login state TTL (10 minutes)
pub const ENTERPRISE_SSO_STATE_TTL_SECS: u64 = 600;

// ── Data Structures ──

#[derive(Debug, Serialize, Deserialize)]
pub struct EnterpriseSsoLoginState {
    pub login_challenge_id: String,
    pub connector_alias: String,
    pub tenant_id: String,
    /// SAML AuthnRequest ID for InResponseTo validation (None for OIDC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authn_request_id: Option<String>,
    /// When set, this is a link flow (not login): link the external identity to this user
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_user_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectorRecord {
    pub alias: String,
    pub tenant_id: String,
    pub provider_type: String,
    pub config: std::collections::HashMap<String, String>,
    pub first_login_policy: String,
}

#[derive(Debug, Clone)]
pub struct EnterpriseProfile {
    pub external_user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

// ── URL Helpers ──

pub fn enterprise_callback_url(config: &crate::config::Config) -> String {
    let base = config
        .core_public_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!(
        "{}/api/v1/enterprise-sso/callback",
        base.trim_end_matches('/')
    )
}

pub fn saml_acs_url(config: &crate::config::Config) -> String {
    let base = config
        .core_public_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!(
        "{}/api/v1/enterprise-sso/saml/acs",
        base.trim_end_matches('/')
    )
}

pub fn sp_entity_id(config: &crate::config::Config) -> String {
    let base = config
        .core_public_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    base.trim_end_matches('/').to_string()
}

pub fn portal_login_url(config: &crate::config::Config) -> String {
    let portal = config
        .portal_url
        .as_deref()
        .unwrap_or(&config.jwt.issuer);
    format!("{}/login", portal.trim_end_matches('/'))
}

// ── DB Helpers ──

pub async fn load_connector(
    pool: &sqlx::MySqlPool,
    alias: &str,
) -> Result<ConnectorRecord> {
    let row = sqlx::query(
        r#"
        SELECT alias, tenant_id, provider_type, config, first_login_policy
        FROM enterprise_sso_connectors
        WHERE alias = ? AND enabled = TRUE
        LIMIT 1
        "#,
    )
    .bind(alias)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| {
        AppError::NotFound(format!(
            "No enabled enterprise SSO connector with alias '{}'",
            alias
        ))
    })?;

    let config_value: serde_json::Value = row.try_get("config")?;
    let config: std::collections::HashMap<String, String> =
        serde_json::from_value(config_value).unwrap_or_default();

    let first_login_policy: String = row
        .try_get("first_login_policy")
        .unwrap_or_else(|_| "auto_merge".to_string());

    Ok(ConnectorRecord {
        alias: row.try_get("alias")?,
        tenant_id: row.try_get("tenant_id")?,
        provider_type: row.try_get("provider_type")?,
        config,
        first_login_policy,
    })
}

// ── User Resolution (tenant-scoped) ──

/// Result of user resolution: either a user or a pending merge that needs confirmation
pub enum UserResolution {
    Found(crate::models::user::User),
    PendingMerge(PendingMergeData),
}

pub async fn find_or_create_enterprise_user<S: HasServices + HasIdentityProviders>(
    state: &S,
    connector: &ConnectorRecord,
    tenant_id: &str,
    profile: &EnterpriseProfile,
    provider_type: &str,
    login_challenge_id: &str,
) -> Result<UserResolution> {
    let tenant_uuid = uuid::Uuid::parse_str(tenant_id)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid tenant_id in SSO state")))?;

    // Try to find existing linked identity
    let existing_link = state
        .identity_provider_service()
        .find_linked_identity(&connector.alias, &profile.external_user_id)
        .await?;

    if let Some(linked) = existing_link {
        let user = state.user_service().get(linked.user_id).await?;
        return Ok(UserResolution::Found(user));
    }

    // Determine first login policy
    let policy = connector
        .first_login_policy
        .parse::<FirstLoginPolicy>()
        .unwrap_or(FirstLoginPolicy::AutoMerge);

    // If email exists, try to find existing user by email
    if policy != FirstLoginPolicy::CreateNew {
        if let Some(ref email) = profile.email {
            if let Ok(existing_user) = state.user_service().get_by_email(email).await {
                match policy {
                    FirstLoginPolicy::AutoMerge => {
                        let input = CreateLinkedIdentityInput {
                            user_id: existing_user.id,
                            provider_type: provider_type.to_string(),
                            provider_alias: connector.alias.to_string(),
                            external_user_id: profile.external_user_id.clone(),
                            external_email: profile.email.clone(),
                        };
                        let _ = state
                            .identity_provider_service()
                            .create_linked_identity(&input)
                            .await;

                        ensure_tenant_membership(state, existing_user.id, tenant_uuid).await;
                        return Ok(UserResolution::Found(existing_user));
                    }
                    FirstLoginPolicy::PromptConfirm => {
                        return Ok(UserResolution::PendingMerge(PendingMergeData {
                            existing_user_id: existing_user.id.to_string(),
                            existing_email: existing_user.email.clone(),
                            external_user_id: profile.external_user_id.clone(),
                            provider_alias: connector.alias.clone(),
                            provider_type: provider_type.to_string(),
                            external_email: profile.email.clone(),
                            display_name: profile.name.clone(),
                            login_challenge_id: login_challenge_id.to_string(),
                            tenant_id: Some(tenant_id.to_string()),
                            ip_address: None,
                            user_agent: None,
                        }));
                    }
                    FirstLoginPolicy::CreateNew => unreachable!(),
                }
            }
        }
    }

    // Create new user
    let email = profile.email.clone().ok_or_else(|| {
        AppError::BadRequest(
            "Enterprise IdP did not return an email. Cannot create account.".to_string(),
        )
    })?;

    let identity_subject = uuid::Uuid::new_v4().to_string();
    let create_input = crate::models::user::CreateUserInput {
        email: email.clone(),
        display_name: profile.name.clone(),
        avatar_url: None,
    };
    let new_user = state
        .user_service()
        .create(&identity_subject, create_input)
        .await?;

    let input = CreateLinkedIdentityInput {
        user_id: new_user.id,
        provider_type: provider_type.to_string(),
        provider_alias: connector.alias.to_string(),
        external_user_id: profile.external_user_id.clone(),
        external_email: profile.email.clone(),
    };
    let _ = state
        .identity_provider_service()
        .create_linked_identity(&input)
        .await;

    ensure_tenant_membership(state, new_user.id, tenant_uuid).await;
    Ok(UserResolution::Found(new_user))
}

pub async fn ensure_tenant_membership<S: HasServices>(
    state: &S,
    user_id: crate::models::common::StringUuid,
    tenant_id: uuid::Uuid,
) {
    if let Ok(tenants) = state.user_service().get_user_tenants(user_id).await {
        if tenants.iter().any(|t| *t.tenant_id == tenant_id) {
            return;
        }
    }

    let input = AddUserToTenantInput {
        user_id: *user_id,
        tenant_id,
        role_in_tenant: "member".to_string(),
    };
    let _ = state.user_service().add_to_tenant(input).await;
}

/// Complete the login flow: consume challenge, create auth code, return redirect URL.
pub async fn complete_login_flow<S: HasServices + HasCache>(
    state: &S,
    login_challenge_id: &str,
    user: &crate::models::user::User,
    session_id: crate::models::common::StringUuid,
) -> Result<String> {
    let challenge_json = state
        .cache()
        .consume_login_challenge(login_challenge_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Login challenge expired during enterprise SSO login".to_string())
        })?;
    let challenge: LoginChallengeData =
        serde_json::from_str(&challenge_json).map_err(|e| AppError::Internal(e.into()))?;

    let auth_code = uuid::Uuid::new_v4().to_string();
    let code_data = AuthorizationCodeData {
        user_id: user.id.to_string(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        session_id: session_id.to_string(),
        client_id: challenge.client_id.clone(),
        redirect_uri: challenge.redirect_uri.clone(),
        scope: challenge.scope,
        nonce: challenge.nonce,
        code_challenge: challenge.code_challenge,
        code_challenge_method: challenge.code_challenge_method,
    };
    let code_json =
        serde_json::to_string(&code_data).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_authorization_code(&auth_code, &code_json, AUTH_CODE_TTL_SECS)
        .await?;

    let mut redirect_url = Url::parse(&challenge.redirect_uri)
        .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;
    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("code", &auth_code);
        if let Some(original_state) = challenge.original_state {
            pairs.append_pair("state", &original_state);
        }
    }

    Ok(redirect_url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_enterprise_sso_login_state_roundtrip() {
        let state = EnterpriseSsoLoginState {
            login_challenge_id: "challenge-123".to_string(),
            connector_alias: "okta-saml".to_string(),
            tenant_id: "tenant-456".to_string(),
            authn_request_id: Some("_req-789".to_string()),
            link_user_id: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        let decoded: EnterpriseSsoLoginState = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.login_challenge_id, "challenge-123");
        assert_eq!(decoded.connector_alias, "okta-saml");
        assert_eq!(decoded.tenant_id, "tenant-456");
        assert_eq!(decoded.authn_request_id.as_deref(), Some("_req-789"));
    }

    #[test]
    fn test_enterprise_sso_login_state_without_authn_request_id() {
        let state = EnterpriseSsoLoginState {
            login_challenge_id: "c".to_string(),
            connector_alias: "a".to_string(),
            tenant_id: "t".to_string(),
            authn_request_id: None,
            link_user_id: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("authn_request_id"));
        let decoded: EnterpriseSsoLoginState = serde_json::from_str(&json).unwrap();
        assert!(decoded.authn_request_id.is_none());
    }

    #[test]
    fn test_connector_record_debug() {
        let record = ConnectorRecord {
            alias: "test".to_string(),
            tenant_id: "tid".to_string(),
            provider_type: "saml".to_string(),
            config: HashMap::new(),
            first_login_policy: "auto_merge".to_string(),
        };
        assert!(format!("{:?}", record).contains("saml"));
    }
}
