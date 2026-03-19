//! Confirm-link API handler for first_login_policy=prompt_confirm flow.
//!
//! When a social or enterprise SSO login finds an email match and the provider's
//! first_login_policy is "prompt_confirm", the callback stores a PendingMergeData
//! in cache and redirects to the portal's confirm-link page. The user then POSTs
//! here to either confirm the merge or create a new account.

use crate::cache::CacheOperations;
use crate::domains::identity::api::auth::helpers::{
    AuthorizationCodeData, LoginChallengeData, AUTH_CODE_TTL_SECS,
};
use crate::error::AppError;
use crate::models::linked_identity::{CreateLinkedIdentityInput, PendingMergeData};
use crate::state::{
    HasAnalytics, HasCache, HasIdentityProviders, HasServices, HasSessionManagement,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

/// Pending merge TTL (10 minutes, same as social/enterprise SSO states)
pub const PENDING_MERGE_TTL_SECS: u64 = 600;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ConfirmLinkInput {
    pub token: String,
    /// If "create_new", creates a separate account instead of linking.
    /// If absent/null, confirms the link to the existing account.
    pub action: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfirmLinkResponse {
    pub redirect_url: String,
}

/// Confirm or reject a pending identity merge.
///
/// POST /api/v1/auth/confirm-link (public, no JWT required)
#[utoipa::path(
    post,
    path = "/api/v1/auth/confirm-link",
    tag = "Identity",
    responses(
        (status = 200, description = "Merge confirmed, redirect URL returned")
    )
)]
pub async fn confirm_link<
    S: HasServices + HasIdentityProviders + HasCache + HasSessionManagement + HasAnalytics,
>(
    State(state): State<S>,
    Json(input): Json<ConfirmLinkInput>,
) -> std::result::Result<Json<ConfirmLinkResponse>, AppError> {
    // 1. Consume pending merge from cache
    let pending_json = state
        .cache()
        .consume_pending_merge(&input.token)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest(
                "Link confirmation token expired or invalid. Please try logging in again."
                    .to_string(),
            )
        })?;

    let pending: PendingMergeData =
        serde_json::from_str(&pending_json).map_err(|e| AppError::Internal(e.into()))?;

    // 2. Resolve user: link to existing or create new
    let user = if input.action.as_deref() == Some("create_new") {
        // Create a new user instead of linking
        let email = pending
            .external_email
            .clone()
            .unwrap_or(pending.existing_email.clone());
        let identity_subject = uuid::Uuid::new_v4().to_string();
        let create_input = crate::models::user::CreateUserInput {
            email,
            display_name: pending.display_name.clone(),
            avatar_url: None,
        };
        state
            .user_service()
            .create(&identity_subject, create_input)
            .await?
    } else {
        // Link to the existing user
        let existing_user_id =
            crate::models::common::StringUuid::parse_str(&pending.existing_user_id)
                .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid pending user_id")))?;
        state.user_service().get(existing_user_id).await?
    };

    // 3. Create linked identity
    let link_input = CreateLinkedIdentityInput {
        user_id: user.id,
        provider_type: pending.provider_type,
        provider_alias: pending.provider_alias,
        external_user_id: pending.external_user_id,
        external_email: pending.external_email,
    };
    let _ = state
        .identity_provider_service()
        .create_linked_identity(&link_input)
        .await;

    // Record identity linked event
    if let Err(e) = state
        .analytics_service()
        .record_identity_linked(
            user.id,
            &link_input.provider_alias,
            &link_input.provider_type,
        )
        .await
    {
        tracing::warn!("Failed to record identity linked event: {}", e);
    }

    // 4. Ensure tenant membership if enterprise SSO
    if let Some(ref tenant_id_str) = pending.tenant_id {
        if let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id_str) {
            crate::domains::identity::api::enterprise_common::ensure_tenant_membership(
                &state,
                user.id,
                tenant_uuid,
            )
            .await;
        }
    }

    // 5. Create session
    let session = state
        .session_service()
        .create_session(
            user.id,
            None,
            pending.ip_address.clone(),
            pending.user_agent.clone(),
        )
        .await?;

    // 6. Consume login challenge and generate authorization code
    let challenge_json = state
        .cache()
        .consume_login_challenge(&pending.login_challenge_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest(
                "Login challenge expired during identity confirmation.".to_string(),
            )
        })?;
    let challenge: LoginChallengeData =
        serde_json::from_str(&challenge_json).map_err(|e| AppError::Internal(e.into()))?;

    let auth_code = uuid::Uuid::new_v4().to_string();
    let code_data = AuthorizationCodeData {
        user_id: user.id.to_string(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        session_id: session.id.to_string(),
        client_id: challenge.client_id.clone(),
        redirect_uri: challenge.redirect_uri.clone(),
        scope: challenge.scope,
        nonce: challenge.nonce,
        code_challenge: challenge.code_challenge,
        code_challenge_method: challenge.code_challenge_method,
    };
    let code_json = serde_json::to_string(&code_data).map_err(|e| AppError::Internal(e.into()))?;
    state
        .cache()
        .store_authorization_code(&auth_code, &code_json, AUTH_CODE_TTL_SECS)
        .await?;

    // 7. Build redirect URL
    let mut redirect_url = url::Url::parse(&challenge.redirect_uri)
        .map_err(|e| AppError::BadRequest(format!("Invalid redirect_uri: {}", e)))?;
    {
        let mut pairs = redirect_url.query_pairs_mut();
        pairs.append_pair("code", &auth_code);
        if let Some(original_state) = challenge.original_state {
            pairs.append_pair("state", &original_state);
        }
    }

    Ok(Json(ConfirmLinkResponse {
        redirect_url: redirect_url.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_link_input_deserialize() {
        let json = r#"{"token": "abc-123"}"#;
        let input: ConfirmLinkInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.token, "abc-123");
        assert!(input.action.is_none());
    }

    #[test]
    fn test_confirm_link_input_create_new() {
        let json = r#"{"token": "abc-123", "action": "create_new"}"#;
        let input: ConfirmLinkInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.action.as_deref(), Some("create_new"));
    }

    #[test]
    fn test_confirm_link_response_serialize() {
        let resp = ConfirmLinkResponse {
            redirect_url: "https://example.com/callback?code=xyz".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("redirect_url"));
    }
}
