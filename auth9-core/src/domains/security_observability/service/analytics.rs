//! Analytics service for login statistics and event tracking

use crate::error::Result;
use crate::models::analytics::{
    CreateLoginEventInput, DailyTrendPoint, LoginEvent, LoginEventType, LoginStats,
};
use crate::models::common::StringUuid;
use crate::repository::LoginEventRepository;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

/// Metadata for recording login events
#[derive(Debug, Clone)]
pub struct LoginEventMetadata {
    pub user_id: StringUuid,
    pub email: String,
    pub tenant_id: Option<StringUuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub device_type: Option<String>,
    pub session_id: Option<StringUuid>,
}

impl LoginEventMetadata {
    /// Create a new LoginEventMetadata with required fields
    pub fn new(user_id: StringUuid, email: impl Into<String>) -> Self {
        Self {
            user_id,
            email: email.into(),
            tenant_id: None,
            ip_address: None,
            user_agent: None,
            device_type: None,
            session_id: None,
        }
    }

    /// Builder method: set tenant_id
    pub fn with_tenant_id(mut self, tenant_id: StringUuid) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Builder method: set ip_address
    pub fn with_ip_address(mut self, ip_address: impl Into<String>) -> Self {
        self.ip_address = Some(ip_address.into());
        self
    }

    /// Builder method: set user_agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Builder method: set device_type
    pub fn with_device_type(mut self, device_type: impl Into<String>) -> Self {
        self.device_type = Some(device_type.into());
        self
    }

    /// Builder method: set session_id
    pub fn with_session_id(mut self, session_id: StringUuid) -> Self {
        self.session_id = Some(session_id);
        self
    }
}

/// Metadata for recording federation login events (social or enterprise SSO)
#[derive(Debug, Clone)]
pub struct FederationEventMetadata {
    pub user_id: Option<StringUuid>,
    pub email: Option<String>,
    pub tenant_id: Option<StringUuid>,
    pub provider_alias: String,
    pub provider_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<StringUuid>,
}

pub struct AnalyticsService<R: LoginEventRepository> {
    login_event_repo: Arc<R>,
}

impl<R: LoginEventRepository> AnalyticsService<R> {
    pub fn new(login_event_repo: Arc<R>) -> Self {
        Self { login_event_repo }
    }

    /// Record a login event
    pub async fn record_login_event(&self, input: CreateLoginEventInput) -> Result<i64> {
        self.login_event_repo.create(&input).await
    }

    /// Get a login event by ID
    pub async fn get_event(&self, id: i64) -> Result<Option<LoginEvent>> {
        self.login_event_repo.find_by_id(id).await
    }

    /// Record a successful login
    pub async fn record_successful_login(&self, metadata: LoginEventMetadata) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: Some(metadata.user_id),
            email: Some(metadata.email),
            tenant_id: metadata.tenant_id,
            event_type: LoginEventType::Success,
            ip_address: metadata.ip_address,
            user_agent: metadata.user_agent,
            device_type: metadata.device_type,
            location: None,
            session_id: metadata.session_id,
            failure_reason: None,
            provider_alias: None,
            provider_type: None,
        };

        self.login_event_repo.create(&input).await
    }

    /// Record a failed login attempt
    pub async fn record_failed_login(
        &self,
        email: &str,
        user_id: Option<StringUuid>,
        event_type: LoginEventType,
        failure_reason: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id,
            email: Some(email.to_string()),
            tenant_id: None,
            event_type,
            ip_address,
            user_agent,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: Some(failure_reason.to_string()),
            provider_alias: None,
            provider_type: None,
        };

        self.login_event_repo.create(&input).await
    }

    /// Record a social login
    pub async fn record_social_login(
        &self,
        user_id: StringUuid,
        email: &str,
        _provider: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: Some(user_id),
            email: Some(email.to_string()),
            tenant_id: None,
            event_type: LoginEventType::Social,
            ip_address,
            user_agent,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: None,
            provider_alias: None,
            provider_type: None,
        };

        self.login_event_repo.create(&input).await
    }

    /// Record a successful federation login (social or enterprise)
    pub async fn record_federation_login(&self, metadata: FederationEventMetadata) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: metadata.user_id,
            email: metadata.email,
            tenant_id: metadata.tenant_id,
            event_type: LoginEventType::FederationSuccess,
            ip_address: metadata.ip_address,
            user_agent: metadata.user_agent,
            device_type: None,
            location: None,
            session_id: metadata.session_id,
            failure_reason: None,
            provider_alias: Some(metadata.provider_alias),
            provider_type: Some(metadata.provider_type),
        };

        self.login_event_repo.create(&input).await
    }

    /// Record a failed federation login
    pub async fn record_federation_failure(
        &self,
        metadata: FederationEventMetadata,
        reason: &str,
    ) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: metadata.user_id,
            email: metadata.email,
            tenant_id: metadata.tenant_id,
            event_type: LoginEventType::FederationFailed,
            ip_address: metadata.ip_address,
            user_agent: metadata.user_agent,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: Some(reason.to_string()),
            provider_alias: Some(metadata.provider_alias),
            provider_type: Some(metadata.provider_type),
        };

        self.login_event_repo.create(&input).await
    }

    /// Record an identity link event
    pub async fn record_identity_linked(
        &self,
        user_id: StringUuid,
        provider_alias: &str,
        provider_type: &str,
    ) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: Some(user_id),
            email: None,
            tenant_id: None,
            event_type: LoginEventType::IdentityLinked,
            ip_address: None,
            user_agent: None,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: None,
            provider_alias: Some(provider_alias.to_string()),
            provider_type: Some(provider_type.to_string()),
        };

        self.login_event_repo.create(&input).await
    }

    /// Record an identity unlink event
    pub async fn record_identity_unlinked(
        &self,
        user_id: StringUuid,
        provider_alias: &str,
        provider_type: &str,
    ) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: Some(user_id),
            email: None,
            tenant_id: None,
            event_type: LoginEventType::IdentityUnlinked,
            ip_address: None,
            user_agent: None,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: None,
            provider_alias: Some(provider_alias.to_string()),
            provider_type: Some(provider_type.to_string()),
        };

        self.login_event_repo.create(&input).await
    }

    /// Get login statistics for a time period, optionally filtered by tenant
    pub async fn get_stats(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<LoginStats> {
        self.login_event_repo.get_stats(tenant_id, start, end).await
    }

    /// Get login statistics for a date range
    pub async fn get_stats_for_range(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<LoginStats> {
        self.get_stats(tenant_id, start, end).await
    }

    /// Get login statistics for the last N days
    pub async fn get_stats_for_days(
        &self,
        tenant_id: Option<StringUuid>,
        days: i64,
    ) -> Result<LoginStats> {
        let end = Utc::now();
        let start = end - Duration::days(days);
        self.get_stats(tenant_id, start, end).await
    }

    /// Get login statistics for the last 24 hours
    pub async fn get_daily_stats(&self, tenant_id: Option<StringUuid>) -> Result<LoginStats> {
        self.get_stats_for_days(tenant_id, 1).await
    }

    /// Get login statistics for the last 7 days
    pub async fn get_weekly_stats(&self, tenant_id: Option<StringUuid>) -> Result<LoginStats> {
        self.get_stats_for_days(tenant_id, 7).await
    }

    /// Get login statistics for the last 30 days
    pub async fn get_monthly_stats(&self, tenant_id: Option<StringUuid>) -> Result<LoginStats> {
        self.get_stats_for_days(tenant_id, 30).await
    }

    /// Get daily trend data for the last N days
    pub async fn get_daily_trend(
        &self,
        tenant_id: Option<StringUuid>,
        days: i64,
    ) -> Result<Vec<DailyTrendPoint>> {
        let end = Utc::now();
        let start = end - Duration::days(days);
        self.login_event_repo
            .get_daily_trend(tenant_id, start, end)
            .await
    }

    /// Get daily trend data for a specific date range
    pub async fn get_daily_trend_for_range(
        &self,
        tenant_id: Option<StringUuid>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<DailyTrendPoint>> {
        self.login_event_repo
            .get_daily_trend(tenant_id, start, end)
            .await
    }

    /// List login events with pagination
    pub async fn list_events(&self, page: i64, per_page: i64) -> Result<(Vec<LoginEvent>, i64)> {
        let offset = (page - 1) * per_page;
        let events = self.login_event_repo.list(offset, per_page).await?;
        let total = self.login_event_repo.count().await?;
        Ok((events, total))
    }

    /// List login events for a specific user
    pub async fn list_user_events(
        &self,
        user_id: StringUuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<LoginEvent>, i64)> {
        let offset = (page - 1) * per_page;
        let events = self
            .login_event_repo
            .list_by_user(user_id, offset, per_page)
            .await?;
        let total = self.login_event_repo.count_by_user(user_id).await?;
        Ok((events, total))
    }

    /// List login events filtered by email address
    pub async fn list_events_by_email(
        &self,
        email: &str,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<LoginEvent>, i64)> {
        let offset = (page - 1) * per_page;
        let events = self
            .login_event_repo
            .list_by_email(email, offset, per_page)
            .await?;
        let total = self.login_event_repo.count_by_email(email).await?;
        Ok((events, total))
    }

    /// List login events for a specific tenant
    pub async fn list_tenant_events(
        &self,
        tenant_id: StringUuid,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<LoginEvent>, i64)> {
        let offset = (page - 1) * per_page;
        let events = self
            .login_event_repo
            .list_by_tenant(tenant_id, offset, per_page)
            .await?;
        let total = self.login_event_repo.count_by_tenant(tenant_id).await?;
        Ok((events, total))
    }

    /// Clean up old login events
    pub async fn cleanup_old_events(&self, days: i64) -> Result<u64> {
        self.login_event_repo.delete_old(days).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::login_event::MockLoginEventRepository;
    use mockall::predicate::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_record_successful_login() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|_| Ok(1));

        let service = AnalyticsService::new(Arc::new(mock));

        let metadata = LoginEventMetadata::new(user_id, "test@example.com")
            .with_ip_address("192.168.1.1")
            .with_device_type("desktop");

        let result = service.record_successful_login(metadata).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_record_failed_login() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::FailedPassword);
            Ok(2)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let result = service
            .record_failed_login(
                "test@example.com",
                None,
                LoginEventType::FailedPassword,
                "Invalid password",
                Some("192.168.1.1".to_string()),
                None,
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_weekly_stats() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_get_stats().returning(|_, start, end| {
            Ok(LoginStats {
                total_logins: 100,
                successful_logins: 80,
                failed_logins: 20,
                unique_users: 50,
                by_event_type: HashMap::new(),
                by_device_type: HashMap::new(),
                period_start: start,
                period_end: end,
            })
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let stats = service.get_weekly_stats(None).await.unwrap();
        assert_eq!(stats.total_logins, 100);
        assert_eq!(stats.successful_logins, 80);
    }

    #[tokio::test]
    async fn test_list_events_pagination() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_list()
            .with(eq(10), eq(10)) // offset = (page - 1) * per_page = (2 - 1) * 10 = 10
            .returning(|_, _| Ok(vec![]));

        mock.expect_count().returning(|| Ok(25));

        let service = AnalyticsService::new(Arc::new(mock));

        let (events, total) = service.list_events(2, 10).await.unwrap();
        assert!(events.is_empty());
        assert_eq!(total, 25);
    }

    #[tokio::test]
    async fn test_record_login_event_directly() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::Locked);
            Ok(42)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let input = CreateLoginEventInput {
            user_id: Some(StringUuid::new_v4()),
            email: Some("locked@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::Locked,
            ip_address: None,
            user_agent: None,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: Some("Account locked".to_string()),
            provider_alias: None,
            provider_type: None,
        };

        let result = service.record_login_event(input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_record_social_login() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::Social);
            assert!(input.failure_reason.is_none());
            Ok(3)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let result = service
            .record_social_login(
                user_id,
                "social@example.com",
                "google",
                Some("10.0.0.1".to_string()),
                Some("Chrome/120".to_string()),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_get_daily_stats() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_get_stats().returning(|_, _, _| {
            Ok(LoginStats {
                total_logins: 10,
                successful_logins: 8,
                failed_logins: 2,
                unique_users: 5,
                by_event_type: HashMap::new(),
                by_device_type: HashMap::new(),
                period_start: Utc::now(),
                period_end: Utc::now(),
            })
        });

        let service = AnalyticsService::new(Arc::new(mock));
        let stats = service.get_daily_stats(None).await.unwrap();
        assert_eq!(stats.total_logins, 10);
    }

    #[tokio::test]
    async fn test_get_monthly_stats() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_get_stats().returning(|_, _, _| {
            Ok(LoginStats {
                total_logins: 3000,
                successful_logins: 2500,
                failed_logins: 500,
                unique_users: 200,
                by_event_type: HashMap::new(),
                by_device_type: HashMap::new(),
                period_start: Utc::now(),
                period_end: Utc::now(),
            })
        });

        let service = AnalyticsService::new(Arc::new(mock));
        let stats = service.get_monthly_stats(None).await.unwrap();
        assert_eq!(stats.total_logins, 3000);
    }

    #[tokio::test]
    async fn test_get_stats_for_range() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_get_stats().returning(|_, _, _| {
            Ok(LoginStats {
                total_logins: 50,
                ..Default::default()
            })
        });

        let service = AnalyticsService::new(Arc::new(mock));
        let start = Utc::now() - Duration::days(14);
        let end = Utc::now();
        let stats = service.get_stats_for_range(None, start, end).await.unwrap();
        assert_eq!(stats.total_logins, 50);
    }

    #[tokio::test]
    async fn test_get_stats_for_days() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_get_stats().returning(|_, _, _| {
            Ok(LoginStats {
                total_logins: 90,
                ..Default::default()
            })
        });

        let service = AnalyticsService::new(Arc::new(mock));
        let stats = service.get_stats_for_days(None, 3).await.unwrap();
        assert_eq!(stats.total_logins, 90);
    }

    #[tokio::test]
    async fn test_list_user_events() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_list_by_user().returning(|_, _, _| Ok(vec![]));

        mock.expect_count_by_user().returning(|_| Ok(5));

        let service = AnalyticsService::new(Arc::new(mock));
        let (events, total) = service.list_user_events(user_id, 1, 20).await.unwrap();
        assert!(events.is_empty());
        assert_eq!(total, 5);
    }

    #[tokio::test]
    async fn test_list_tenant_events() {
        let mut mock = MockLoginEventRepository::new();
        let tenant_id = StringUuid::new_v4();

        mock.expect_list_by_tenant().returning(|_, _, _| Ok(vec![]));

        mock.expect_count_by_tenant().returning(|_| Ok(12));

        let service = AnalyticsService::new(Arc::new(mock));
        let (events, total) = service.list_tenant_events(tenant_id, 1, 10).await.unwrap();
        assert!(events.is_empty());
        assert_eq!(total, 12);
    }

    #[tokio::test]
    async fn test_cleanup_old_events() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_delete_old()
            .with(eq(90i64))
            .returning(|_| Ok(150));

        let service = AnalyticsService::new(Arc::new(mock));
        let deleted = service.cleanup_old_events(90).await.unwrap();
        assert_eq!(deleted, 150);
    }

    #[tokio::test]
    async fn test_record_successful_login_with_session_and_tenant() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();
        let tenant_id = StringUuid::new_v4();
        let session_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::Success);
            assert!(input.tenant_id.is_some());
            assert!(input.session_id.is_some());
            assert!(input.device_type.is_some());
            Ok(10)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let metadata = LoginEventMetadata::new(user_id, "user@example.com")
            .with_tenant_id(tenant_id)
            .with_ip_address("192.168.1.1")
            .with_user_agent("Mozilla/5.0")
            .with_device_type("mobile")
            .with_session_id(session_id);

        let result = service.record_successful_login(metadata).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_failed_login_with_user_id() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::FailedMfa);
            assert!(input.user_id.is_some());
            assert!(input.failure_reason.is_some());
            Ok(5)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let result = service
            .record_failed_login(
                "mfa@example.com",
                Some(user_id),
                LoginEventType::FailedMfa,
                "Invalid MFA code",
                None,
                Some("Safari/17".to_string()),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_federation_login() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::FederationSuccess);
            assert_eq!(input.provider_alias.as_deref(), Some("google"));
            assert_eq!(input.provider_type.as_deref(), Some("google"));
            assert!(input.failure_reason.is_none());
            Ok(100)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let metadata = FederationEventMetadata {
            user_id: Some(user_id),
            email: Some("user@example.com".to_string()),
            tenant_id: None,
            provider_alias: "google".to_string(),
            provider_type: "google".to_string(),
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: None,
            session_id: None,
        };

        let result = service.record_federation_login(metadata).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
    }

    #[tokio::test]
    async fn test_record_federation_failure() {
        let mut mock = MockLoginEventRepository::new();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::FederationFailed);
            assert_eq!(input.provider_alias.as_deref(), Some("okta-saml"));
            assert_eq!(input.provider_type.as_deref(), Some("saml"));
            assert_eq!(input.failure_reason.as_deref(), Some("invalid_issuer"));
            Ok(101)
        });

        let service = AnalyticsService::new(Arc::new(mock));

        let metadata = FederationEventMetadata {
            user_id: None,
            email: None,
            tenant_id: None,
            provider_alias: "okta-saml".to_string(),
            provider_type: "saml".to_string(),
            ip_address: None,
            user_agent: None,
            session_id: None,
        };

        let result = service
            .record_federation_failure(metadata, "invalid_issuer")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_identity_linked() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::IdentityLinked);
            assert_eq!(input.provider_alias.as_deref(), Some("github"));
            assert_eq!(input.provider_type.as_deref(), Some("github"));
            assert!(input.user_id.is_some());
            Ok(102)
        });

        let service = AnalyticsService::new(Arc::new(mock));
        let result = service
            .record_identity_linked(user_id, "github", "github")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_identity_unlinked() {
        let mut mock = MockLoginEventRepository::new();
        let user_id = StringUuid::new_v4();

        mock.expect_create().returning(|input| {
            assert_eq!(input.event_type, LoginEventType::IdentityUnlinked);
            assert_eq!(input.provider_alias.as_deref(), Some("azure-oidc"));
            assert_eq!(input.provider_type.as_deref(), Some("oidc"));
            Ok(103)
        });

        let service = AnalyticsService::new(Arc::new(mock));
        let result = service
            .record_identity_unlinked(user_id, "azure-oidc", "oidc")
            .await;
        assert!(result.is_ok());
    }
}
