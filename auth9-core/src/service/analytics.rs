//! Analytics service for login statistics and event tracking

use crate::domain::{CreateLoginEventInput, LoginEvent, LoginEventType, LoginStats, StringUuid};
use crate::error::Result;
use crate::repository::LoginEventRepository;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

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

    /// Record a successful login
    pub async fn record_successful_login(
        &self,
        user_id: StringUuid,
        email: &str,
        tenant_id: Option<StringUuid>,
        ip_address: Option<String>,
        user_agent: Option<String>,
        device_type: Option<String>,
        session_id: Option<StringUuid>,
    ) -> Result<i64> {
        let input = CreateLoginEventInput {
            user_id: Some(user_id),
            email: Some(email.to_string()),
            tenant_id,
            event_type: LoginEventType::Success,
            ip_address,
            user_agent,
            device_type,
            location: None,
            session_id,
            failure_reason: None,
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
        };

        self.login_event_repo.create(&input).await
    }

    /// Get login statistics for a time period
    pub async fn get_stats(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<LoginStats> {
        self.login_event_repo.get_stats(start, end).await
    }

    /// Get login statistics for a date range (alias for get_stats)
    pub async fn get_stats_for_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<LoginStats> {
        self.get_stats(start, end).await
    }

    /// Get login statistics for the last N days
    pub async fn get_stats_for_days(&self, days: i64) -> Result<LoginStats> {
        let end = Utc::now();
        let start = end - Duration::days(days);
        self.get_stats(start, end).await
    }

    /// Get login statistics for the last 24 hours
    pub async fn get_daily_stats(&self) -> Result<LoginStats> {
        self.get_stats_for_days(1).await
    }

    /// Get login statistics for the last 7 days
    pub async fn get_weekly_stats(&self) -> Result<LoginStats> {
        self.get_stats_for_days(7).await
    }

    /// Get login statistics for the last 30 days
    pub async fn get_monthly_stats(&self) -> Result<LoginStats> {
        self.get_stats_for_days(30).await
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

        let result = service
            .record_successful_login(
                user_id,
                "test@example.com",
                None,
                Some("192.168.1.1".to_string()),
                None,
                Some("desktop".to_string()),
                None,
            )
            .await;

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

        mock.expect_get_stats().returning(|start, end| {
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

        let stats = service.get_weekly_stats().await.unwrap();
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
}
