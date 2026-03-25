//! LDAP enterprise SSO login handler.
//!
//! Unlike OIDC/SAML (redirect-based), LDAP authenticates with direct credentials.

use crate::domains::identity::api::enterprise_common::{
    self, ConnectorRecord, EnterpriseProfile, UserResolution,
};
use crate::domains::identity::api::hosted_login::HostedLoginTokenResponse;
use crate::domains::security_observability::service::analytics::FederationEventMetadata;
use crate::error::{AppError, Result};
use crate::models::common::StringUuid;
use crate::state::{
    HasAnalytics, HasCache, HasDbPool, HasIdentityProviders, HasLdapAuth, HasServices,
    HasSessionManagement,
};
use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct LdapLoginRequest {
    pub connector_alias: String,
    pub username: String,
    pub password: String,
    pub login_challenge: Option<String>,
}

pub async fn ldap_login<
    S: HasServices
        + HasCache
        + HasDbPool
        + HasLdapAuth
        + HasIdentityProviders
        + HasSessionManagement
        + HasAnalytics,
>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(input): Json<LdapLoginRequest>,
) -> Result<impl IntoResponse> {
    // 1. Load connector by alias
    let connector =
        enterprise_common::load_connector(state.db_pool(), &input.connector_alias).await?;

    if connector.provider_type != "ldap" {
        return Err(AppError::BadRequest(
            "Connector is not an LDAP connector".to_string(),
        ));
    }

    // 2. Parse LDAP config and authenticate
    let ldap_config = crate::models::ldap::parse_ldap_config(&connector.config)?;
    let profile = match state
        .ldap_authenticator()
        .authenticate(&ldap_config, &input.username, &input.password)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            // Record federation failure event
            record_federation_event(
                &state,
                &connector,
                None,
                None,
                &headers,
                false,
                Some(e.to_string()),
            )
            .await;
            return Err(e);
        }
    };

    // 3. Map to EnterpriseProfile
    let display_name = profile
        .display_name
        .clone()
        .or_else(|| {
            match (&profile.first_name, &profile.last_name) {
                (Some(f), Some(l)) => Some(format!("{} {}", f, l)),
                (Some(f), None) => Some(f.clone()),
                (None, Some(l)) => Some(l.clone()),
                _ => None,
            }
        });

    let enterprise_profile = EnterpriseProfile {
        external_user_id: profile.dn.clone(),
        email: profile.email.clone(),
        name: display_name,
    };

    // 4. Find or create user
    let login_challenge_id = input.login_challenge.as_deref().unwrap_or("");
    let resolution = enterprise_common::find_or_create_enterprise_user(
        &state,
        &connector,
        &connector.tenant_id,
        &enterprise_profile,
        "ldap",
        login_challenge_id,
    )
    .await?;

    let user = match resolution {
        UserResolution::Found(user) => user,
        UserResolution::PendingMerge(data) => {
            // Return pending merge response for user confirmation
            return Ok(Json(serde_json::json!({
                "pending_merge": true,
                "existing_email": data.existing_email,
                "external_email": data.external_email,
                "login_challenge_id": data.login_challenge_id,
            }))
            .into_response());
        }
    };

    // 5. Create session
    let ip = extract_client_ip(&headers);
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let session = state
        .session_service()
        .create_session(user.id, None, ip, ua)
        .await?;

    // 6. Record success analytics event
    record_federation_event(
        &state,
        &connector,
        Some(user.id),
        Some(&user.email),
        &headers,
        true,
        None,
    )
    .await;

    // 7. Complete flow
    if let Some(ref challenge_id) = input.login_challenge {
        if !challenge_id.is_empty() {
            let redirect_url =
                enterprise_common::complete_login_flow(&state, challenge_id, &user, session.id)
                    .await?;
            return Ok(Json(serde_json::json!({
                "redirect_url": redirect_url,
            }))
            .into_response());
        }
    }

    // No login challenge — return identity token directly
    let jwt_manager = HasServices::jwt_manager(&state);
    let identity_token = jwt_manager.create_identity_token_with_session(
        *user.id,
        &user.email,
        user.display_name.as_deref(),
        Some(*session.id),
    )?;

    Ok(Json(HostedLoginTokenResponse {
        access_token: identity_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt_manager.access_token_ttl(),
        pending_actions: vec![],
    })
    .into_response())
}

fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        })
}

async fn record_federation_event<S: HasAnalytics>(
    state: &S,
    connector: &ConnectorRecord,
    user_id: Option<StringUuid>,
    email: Option<&str>,
    _headers: &HeaderMap,
    success: bool,
    failure_reason: Option<String>,
) {
    let tenant_id = StringUuid::parse_str(&connector.tenant_id).ok();
    let metadata = FederationEventMetadata {
        user_id,
        email: email.map(String::from),
        tenant_id,
        provider_alias: connector.alias.clone(),
        provider_type: "ldap".to_string(),
        ip_address: None,
        user_agent: None,
        session_id: None,
    };

    if success {
        let _ = state
            .analytics_service()
            .record_federation_login(metadata)
            .await;
    } else {
        let reason = failure_reason.unwrap_or_else(|| "LDAP authentication failed".to_string());
        let _ = state
            .analytics_service()
            .record_federation_failure(metadata, &reason)
            .await;
    }
}
