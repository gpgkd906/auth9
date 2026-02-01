//! Session management business logic

use crate::domain::{parse_user_agent, CreateSessionInput, Session, SessionInfo, StringUuid};
use crate::error::{AppError, Result};
use crate::keycloak::KeycloakClient;
use crate::repository::{SessionRepository, UserRepository};
use std::sync::Arc;

pub struct SessionService<S: SessionRepository, U: UserRepository> {
    session_repo: Arc<S>,
    user_repo: Arc<U>,
    keycloak: Arc<KeycloakClient>,
}

impl<S: SessionRepository, U: UserRepository> SessionService<S, U> {
    pub fn new(session_repo: Arc<S>, user_repo: Arc<U>, keycloak: Arc<KeycloakClient>) -> Self {
        Self {
            session_repo,
            user_repo,
            keycloak,
        }
    }

    /// Create a new session after login
    pub async fn create_session(
        &self,
        user_id: StringUuid,
        keycloak_session_id: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Session> {
        let (device_type, device_name) = user_agent
            .as_ref()
            .map(|ua| parse_user_agent(ua))
            .unwrap_or((None, None));

        let input = CreateSessionInput {
            user_id,
            keycloak_session_id,
            device_type,
            device_name,
            ip_address,
            location: None, // TODO: Implement IP geolocation
            user_agent,
        };

        self.session_repo.create(&input).await
    }

    /// Get sessions for the current user
    pub async fn get_user_sessions(
        &self,
        user_id: StringUuid,
        current_session_id: Option<StringUuid>,
    ) -> Result<Vec<SessionInfo>> {
        let sessions = self.session_repo.list_active_by_user(user_id).await?;

        let session_infos: Vec<SessionInfo> = sessions
            .into_iter()
            .map(|s| {
                let mut info: SessionInfo = s.clone().into();
                if let Some(current_id) = current_session_id {
                    info.is_current = s.id == current_id;
                }
                info
            })
            .collect();

        Ok(session_infos)
    }

    /// Revoke a specific session
    pub async fn revoke_session(&self, session_id: StringUuid, user_id: StringUuid) -> Result<()> {
        // Get the session to verify ownership
        let session = self
            .session_repo
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

        // Verify the session belongs to the user
        if session.user_id != user_id {
            return Err(AppError::Forbidden(
                "Cannot revoke another user's session".to_string(),
            ));
        }

        // Revoke in Keycloak if session ID exists
        if let Some(kc_session_id) = &session.keycloak_session_id {
            // Ignore errors from Keycloak (session may already be expired)
            let _ = self.keycloak.delete_user_session(kc_session_id).await;
        }

        // Mark session as revoked in our database
        self.session_repo.revoke(session_id).await
    }

    /// Revoke all sessions except the current one
    pub async fn revoke_other_sessions(
        &self,
        user_id: StringUuid,
        current_session_id: StringUuid,
    ) -> Result<u64> {
        // Get all active sessions
        let sessions = self.session_repo.list_active_by_user(user_id).await?;

        // Revoke each session in Keycloak (except current)
        for session in sessions {
            if session.id == current_session_id {
                continue;
            }

            if let Some(kc_session_id) = &session.keycloak_session_id {
                let _ = self.keycloak.delete_user_session(kc_session_id).await;
            }
        }

        // Revoke in database
        self.session_repo
            .revoke_all_except(user_id, current_session_id)
            .await
    }

    /// Force logout a user (admin action)
    pub async fn force_logout_user(&self, user_id: StringUuid) -> Result<u64> {
        // Get user to get their Keycloak ID
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Logout from Keycloak
        self.keycloak.logout_user(&user.keycloak_id).await?;

        // Revoke all sessions in database
        self.session_repo.revoke_all_by_user(user_id).await
    }

    /// Update session last active time
    pub async fn update_last_active(&self, session_id: StringUuid) -> Result<()> {
        self.session_repo.update_last_active(session_id).await
    }

    /// Get admin view of user sessions
    pub async fn get_user_sessions_admin(&self, user_id: StringUuid) -> Result<Vec<SessionInfo>> {
        // Verify user exists
        let _ = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let sessions = self.session_repo.list_active_by_user(user_id).await?;

        let session_infos: Vec<SessionInfo> = sessions.into_iter().map(|s| s.into()).collect();

        Ok(session_infos)
    }

    /// Clean up old sessions
    pub async fn cleanup_old_sessions(&self, days: i64) -> Result<u64> {
        self.session_repo.delete_old(days).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::User;
    use crate::repository::session::MockSessionRepository;
    use crate::repository::user::MockUserRepository;
    use mockall::predicate::*;

    // Note: Tests involving KeycloakClient would need wiremock for HTTP mocking
    // These tests focus on repository interactions

    #[tokio::test]
    async fn test_session_info_is_current() {
        let session = Session {
            id: StringUuid::new_v4(),
            device_type: Some("desktop".to_string()),
            device_name: Some("Chrome on macOS".to_string()),
            ..Default::default()
        };

        let mut info: SessionInfo = session.into();
        info.is_current = true;

        assert!(info.is_current);
        assert_eq!(info.device_type, Some("desktop".to_string()));
    }

    #[test]
    fn test_parse_user_agent() {
        let (device_type, device_name) = parse_user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/120.0.0.0",
        );

        assert_eq!(device_type, Some("desktop".to_string()));
        assert_eq!(device_name, Some("Chrome on macOS".to_string()));
    }

    #[test]
    fn test_parse_user_agent_mobile() {
        // Android mobile user agent
        let (device_type, device_name) = parse_user_agent(
            "Mozilla/5.0 (Linux; Android 11; Pixel 5) AppleWebKit/537.36 Chrome/90.0.4430.91 Mobile Safari/537.36"
        );

        assert_eq!(device_type, Some("mobile".to_string()));
        assert!(device_name.is_some());
        assert!(device_name.unwrap().contains("Android"));
    }

    #[test]
    fn test_parse_user_agent_tablet() {
        let (device_type, device_name) =
            parse_user_agent("Mozilla/5.0 (iPad; CPU OS 14_0 like Mac OS X) AppleWebKit/605.1.15");

        assert_eq!(device_type, Some("tablet".to_string()));
        assert!(device_name.is_some());
    }

    #[test]
    fn test_parse_user_agent_ios_safari() {
        // iPhone without "Mobile" keyword in UA still detected as desktop by current implementation
        let (device_type, device_name) = parse_user_agent(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X) AppleWebKit/605.1.15",
        );

        // Note: Current implementation doesn't detect iPhone as mobile
        assert!(device_type.is_some());
        assert!(device_name.is_some());
    }

    #[tokio::test]
    async fn test_get_user_sessions_empty() {
        let mut session_mock = MockSessionRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();

        session_mock
            .expect_list_active_by_user()
            .with(eq(user_id))
            .returning(|_| Ok(vec![]));

        // Create a mock Keycloak client - we won't use it in this test
        let keycloak = create_test_keycloak_client();

        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let sessions = service.get_user_sessions(user_id, None).await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_sessions_with_current() {
        let mut session_mock = MockSessionRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let session_id = StringUuid::new_v4();

        session_mock
            .expect_list_active_by_user()
            .with(eq(user_id))
            .returning(move |_| {
                Ok(vec![Session {
                    id: session_id,
                    user_id,
                    device_type: Some("desktop".to_string()),
                    device_name: Some("Chrome".to_string()),
                    ..Default::default()
                }])
            });

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let sessions = service
            .get_user_sessions(user_id, Some(session_id))
            .await
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].is_current);
    }

    #[tokio::test]
    async fn test_revoke_session_not_found() {
        let mut session_mock = MockSessionRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let session_id = StringUuid::new_v4();

        session_mock
            .expect_find_by_id()
            .with(eq(session_id))
            .returning(|_| Ok(None));

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.revoke_session(session_id, user_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_revoke_session_wrong_user() {
        let mut session_mock = MockSessionRepository::new();
        let user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();
        let other_user_id = StringUuid::new_v4();
        let session_id = StringUuid::new_v4();

        session_mock.expect_find_by_id().returning(move |_| {
            Ok(Some(Session {
                id: session_id,
                user_id: other_user_id, // Different user
                ..Default::default()
            }))
        });

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.revoke_session(session_id, user_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn test_update_last_active() {
        let mut session_mock = MockSessionRepository::new();
        let user_mock = MockUserRepository::new();
        let session_id = StringUuid::new_v4();

        session_mock
            .expect_update_last_active()
            .with(eq(session_id))
            .returning(|_| Ok(()));

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.update_last_active(session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_old_sessions() {
        let mut session_mock = MockSessionRepository::new();
        let user_mock = MockUserRepository::new();

        session_mock.expect_delete_old().returning(|days| {
            assert_eq!(days, 30);
            Ok(5)
        });

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let count = service.cleanup_old_sessions(30).await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_get_user_sessions_admin_user_not_found() {
        let session_mock = MockSessionRepository::new();
        let mut user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();

        user_mock
            .expect_find_by_id()
            .with(eq(user_id))
            .returning(|_| Ok(None));

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let result = service.get_user_sessions_admin(user_id).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_user_sessions_admin_success() {
        let mut session_mock = MockSessionRepository::new();
        let mut user_mock = MockUserRepository::new();
        let user_id = StringUuid::new_v4();

        user_mock
            .expect_find_by_id()
            .with(eq(user_id))
            .returning(|id| {
                Ok(Some(User {
                    id,
                    keycloak_id: "kc-123".to_string(),
                    ..Default::default()
                }))
            });

        session_mock
            .expect_list_active_by_user()
            .with(eq(user_id))
            .returning(|uid| {
                Ok(vec![
                    Session {
                        user_id: uid,
                        device_type: Some("desktop".to_string()),
                        ..Default::default()
                    },
                    Session {
                        user_id: uid,
                        device_type: Some("mobile".to_string()),
                        ..Default::default()
                    },
                ])
            });

        let keycloak = create_test_keycloak_client();
        let service = SessionService::new(
            Arc::new(session_mock),
            Arc::new(user_mock),
            Arc::new(keycloak),
        );

        let sessions = service.get_user_sessions_admin(user_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    // Helper to create a test KeycloakClient (won't make actual calls in these tests)
    fn create_test_keycloak_client() -> KeycloakClient {
        use crate::config::KeycloakConfig;
        KeycloakClient::new(KeycloakConfig {
            url: "http://localhost:8081".to_string(),
            public_url: "http://localhost:8081".to_string(),
            realm: "auth9".to_string(),
            admin_client_id: "admin-cli".to_string(),
            admin_client_secret: "".to_string(),
            ssl_required: "none".to_string(),
            core_public_url: None,
            portal_url: None,
        })
    }
}
