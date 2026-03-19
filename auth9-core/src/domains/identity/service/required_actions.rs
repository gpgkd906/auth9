//! Required actions service.
//!
//! Manages pending/required actions for users (verify email, force password
//! update, complete profile) through the IdentityEngine abstraction.

use crate::error::Result;
use crate::identity_engine::IdentityEngine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

/// Known action types that map to portal pages.
pub const ACTION_VERIFY_EMAIL: &str = "verify_email";
pub const ACTION_UPDATE_PASSWORD: &str = "update_password"; // pragma: allowlist secret
pub const ACTION_COMPLETE_PROFILE: &str = "complete_profile";
pub const ACTION_CONFIGURE_TOTP: &str = "CONFIGURE_TOTP";

/// Response object for a pending action with its redirect URL.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingActionResponse {
    pub id: String,
    pub action_type: String,
    pub redirect_url: String,
}

pub struct RequiredActionService {
    identity_engine: Arc<dyn IdentityEngine>,
}

impl RequiredActionService {
    pub fn new(identity_engine: Arc<dyn IdentityEngine>) -> Self {
        Self { identity_engine }
    }

    /// Get all pending actions for a user, with redirect URLs.
    pub async fn get_pending_actions(&self, user_id: &str) -> Result<Vec<PendingActionResponse>> {
        let actions = self
            .identity_engine
            .action_store()
            .get_pending_actions(user_id)
            .await?;

        Ok(actions
            .into_iter()
            .map(|a| PendingActionResponse {
                id: a.id,
                redirect_url: Self::action_redirect_url(&a.action_type),
                action_type: a.action_type,
            })
            .collect())
    }

    /// Check pending actions and also auto-detect implied actions
    /// (e.g., temporary password → force update password).
    pub async fn check_post_login_actions(
        &self,
        identity_subject: &str,
    ) -> Result<Vec<PendingActionResponse>> {
        let mut actions = self.get_pending_actions(identity_subject).await?;

        // Check if password is temporary → auto-create update_password action
        if !actions
            .iter()
            .any(|a| a.action_type == ACTION_UPDATE_PASSWORD)
            && self.is_password_temporary(identity_subject).await?
        {
            let id = self
                .identity_engine
                .action_store()
                .create_action(identity_subject, ACTION_UPDATE_PASSWORD, None)
                .await?;
            actions.push(PendingActionResponse {
                id,
                action_type: ACTION_UPDATE_PASSWORD.to_string(),
                redirect_url: Self::action_redirect_url(ACTION_UPDATE_PASSWORD),
            });
        }

        Ok(actions)
    }

    /// Complete a pending action.
    pub async fn complete_action(&self, action_id: &str) -> Result<()> {
        self.identity_engine
            .action_store()
            .complete_action(action_id)
            .await
    }

    /// Create a new pending action for a user.
    pub async fn create_action(
        &self,
        user_id: &str,
        action_type: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<String> {
        self.identity_engine
            .action_store()
            .create_action(user_id, action_type, metadata)
            .await
    }

    /// Cancel a pending action.
    pub async fn cancel_action(&self, action_id: &str) -> Result<()> {
        self.identity_engine
            .action_store()
            .cancel_action(action_id)
            .await
    }

    /// Map action type to portal redirect URL.
    fn action_redirect_url(action_type: &str) -> String {
        match action_type {
            ACTION_VERIFY_EMAIL => "/verify-email".to_string(),
            ACTION_UPDATE_PASSWORD => "/force-update-password".to_string(),
            ACTION_COMPLETE_PROFILE => "/complete-profile".to_string(),
            ACTION_CONFIGURE_TOTP => "/mfa/setup-totp".to_string(),
            other => format!("/pending-action?type={}", other),
        }
    }

    /// Check if the user's password credential has `temporary: true`.
    async fn is_password_temporary(&self, user_id: &str) -> Result<bool> {
        // Query the credential store for the password credential
        // The auth9-oidc engine stores password data as JSON with a "temporary" field
        let credentials = self
            .identity_engine
            .credential_store()
            .list_user_credentials(user_id)
            .await?;

        // If we can't determine, assume not temporary
        // The actual temporary flag check requires reading credential_data JSON,
        // which is only available in the auth9-oidc adapter.
        // For now, the explicit pending_actions table is the primary mechanism.
        let _ = credentials;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_redirect_url_known_types() {
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_VERIFY_EMAIL),
            "/verify-email"
        );
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_UPDATE_PASSWORD),
            "/force-update-password"
        );
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_COMPLETE_PROFILE),
            "/complete-profile"
        );
        assert_eq!(
            RequiredActionService::action_redirect_url(ACTION_CONFIGURE_TOTP),
            "/mfa/setup-totp"
        );
    }

    #[test]
    fn action_redirect_url_unknown_type() {
        let url = RequiredActionService::action_redirect_url("custom_action");
        assert_eq!(url, "/pending-action?type=custom_action");
    }

    #[test]
    fn pending_action_response_serialization() {
        let resp = PendingActionResponse {
            id: "act-1".to_string(),
            action_type: "verify_email".to_string(),
            redirect_url: "/verify-email".to_string(),
        };

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["action_type"], "verify_email");
        assert_eq!(json["redirect_url"], "/verify-email");
    }
}
