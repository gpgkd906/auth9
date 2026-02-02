//! Security detection service for identifying suspicious activity

use crate::domain::{
    AlertSeverity, CreateSecurityAlertInput, LoginEvent, LoginEventType, SecurityAlert,
    SecurityAlertType, StringUuid, WebhookEvent,
};
use crate::error::Result;
use crate::repository::WebhookRepository;
use crate::repository::{LoginEventRepository, SecurityAlertRepository};
use crate::service::{WebhookEventPublisher, WebhookService};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for security detection rules
#[derive(Debug, Clone)]
pub struct SecurityDetectionConfig {
    /// Number of failed login attempts before triggering brute force alert
    pub brute_force_threshold: i32,
    /// Time window in minutes for brute force detection
    pub brute_force_window_mins: i64,
    /// Number of different accounts from same IP before password spray alert
    pub password_spray_threshold: i32,
    /// Time window in minutes for password spray detection
    pub password_spray_window_mins: i64,
    /// Distance in km that's considered "impossible travel" within 1 hour
    pub impossible_travel_distance_km: f64,
}

impl Default for SecurityDetectionConfig {
    fn default() -> Self {
        Self {
            brute_force_threshold: 5,
            brute_force_window_mins: 10,
            password_spray_threshold: 5,
            password_spray_window_mins: 10,
            impossible_travel_distance_km: 500.0,
        }
    }
}

/// Security detection service
pub struct SecurityDetectionService<
    L: LoginEventRepository,
    S: SecurityAlertRepository,
    W: WebhookRepository,
> {
    login_event_repo: Arc<L>,
    security_alert_repo: Arc<S>,
    webhook_service: Arc<WebhookService<W>>,
    config: SecurityDetectionConfig,
}

impl<L: LoginEventRepository, S: SecurityAlertRepository, W: WebhookRepository + 'static>
    SecurityDetectionService<L, S, W>
{
    pub fn new(
        login_event_repo: Arc<L>,
        security_alert_repo: Arc<S>,
        webhook_service: Arc<WebhookService<W>>,
        config: SecurityDetectionConfig,
    ) -> Self {
        Self {
            login_event_repo,
            security_alert_repo,
            webhook_service,
            config,
        }
    }

    /// Analyze a login event for security threats
    ///
    /// This method should be called after each login event is recorded.
    /// It checks for various attack patterns and creates alerts as needed.
    pub async fn analyze_login_event(&self, event: &LoginEvent) -> Result<Vec<SecurityAlert>> {
        let mut alerts = Vec::new();

        // Check for brute force attacks
        if let Some(ip) = &event.ip_address {
            if let Some(alert) = self.check_brute_force(ip, event.user_id).await? {
                alerts.push(alert);
            }

            // Check for password spray attacks
            if let Some(alert) = self.check_password_spray(ip).await? {
                alerts.push(alert);
            }
        }

        // Check for new device login
        if event.event_type == LoginEventType::Success {
            if let Some(user_id) = event.user_id {
                if let Some(alert) = self.check_new_device(user_id, event).await? {
                    alerts.push(alert);
                }

                // Check for impossible travel
                if let Some(alert) = self.check_impossible_travel(user_id, event).await? {
                    alerts.push(alert);
                }
            }
        }

        // Trigger webhooks for each alert
        for alert in &alerts {
            let _ = self
                .webhook_service
                .trigger_event(WebhookEvent {
                    event_type: "security.alert".to_string(),
                    timestamp: Utc::now(),
                    data: serde_json::json!({
                        "alert_id": alert.id.to_string(),
                        "alert_type": alert.alert_type,
                        "severity": alert.severity,
                        "user_id": alert.user_id.map(|id| id.to_string()),
                        "details": alert.details,
                    }),
                })
                .await;
        }

        Ok(alerts)
    }

    /// Check for brute force attack pattern
    async fn check_brute_force(
        &self,
        ip_address: &str,
        user_id: Option<StringUuid>,
    ) -> Result<Option<SecurityAlert>> {
        let since = Utc::now() - Duration::minutes(self.config.brute_force_window_mins);

        let failed_attempts = self
            .login_event_repo
            .count_failed_by_ip(ip_address, since)
            .await?;

        if failed_attempts >= self.config.brute_force_threshold as i64 {
            let input = CreateSecurityAlertInput {
                user_id,
                tenant_id: None,
                alert_type: SecurityAlertType::BruteForce,
                severity: AlertSeverity::High,
                details: Some(serde_json::json!({
                    "ip_address": ip_address,
                    "failed_attempts": failed_attempts,
                    "window_minutes": self.config.brute_force_window_mins,
                })),
            };

            let alert = self.security_alert_repo.create(&input).await?;
            return Ok(Some(alert));
        }

        Ok(None)
    }

    /// Check for password spray attack pattern
    async fn check_password_spray(&self, ip_address: &str) -> Result<Option<SecurityAlert>> {
        let since = Utc::now() - Duration::minutes(self.config.password_spray_window_mins);

        let unique_accounts = self
            .login_event_repo
            .count_failed_by_ip_multi_user(ip_address, since)
            .await?;

        if unique_accounts >= self.config.password_spray_threshold as i64 {
            let input = CreateSecurityAlertInput {
                user_id: None,
                tenant_id: None,
                alert_type: SecurityAlertType::SuspiciousIp,
                severity: AlertSeverity::Critical,
                details: Some(serde_json::json!({
                    "ip_address": ip_address,
                    "unique_accounts_targeted": unique_accounts,
                    "window_minutes": self.config.password_spray_window_mins,
                    "detection_reason": "password_spray",
                })),
            };

            let alert = self.security_alert_repo.create(&input).await?;
            return Ok(Some(alert));
        }

        Ok(None)
    }

    /// Check if this is a login from a new device
    async fn check_new_device(
        &self,
        user_id: StringUuid,
        event: &LoginEvent,
    ) -> Result<Option<SecurityAlert>> {
        // Get recent successful logins for this user
        let recent_events = self.login_event_repo.list_by_user(user_id, 0, 100).await?;

        // Build a set of known device fingerprints
        let mut known_devices: HashMap<String, bool> = HashMap::new();
        for evt in &recent_events {
            if evt.event_type == LoginEventType::Success {
                if let Some(ua) = &evt.user_agent {
                    known_devices.insert(ua.clone(), true);
                }
            }
        }

        // Check if current device is new
        if let Some(user_agent) = &event.user_agent {
            if !known_devices.contains_key(user_agent) && !recent_events.is_empty() {
                let input = CreateSecurityAlertInput {
                    user_id: Some(user_id),
                    tenant_id: event.tenant_id,
                    alert_type: SecurityAlertType::NewDevice,
                    severity: AlertSeverity::Medium,
                    details: Some(serde_json::json!({
                        "user_agent": user_agent,
                        "device_type": event.device_type,
                        "ip_address": event.ip_address,
                        "location": event.location,
                    })),
                };

                let alert = self.security_alert_repo.create(&input).await?;
                return Ok(Some(alert));
            }
        }

        Ok(None)
    }

    /// Check for impossible travel (login from distant location in short time)
    async fn check_impossible_travel(
        &self,
        user_id: StringUuid,
        event: &LoginEvent,
    ) -> Result<Option<SecurityAlert>> {
        // Get the user's last successful login
        let recent_events = self.login_event_repo.list_by_user(user_id, 0, 10).await?;

        let last_login = recent_events
            .iter()
            .find(|e| e.event_type == LoginEventType::Success && e.id != event.id);

        if let Some(last) = last_login {
            // Check if both events have location data
            if let (Some(current_loc), Some(last_loc)) = (&event.location, &last.location) {
                // Parse locations (expected format: "City, Country" or lat,lng)
                let time_diff = event.created_at - last.created_at;

                // If less than 1 hour apart, check distance
                if time_diff.num_hours() < 1 && current_loc != last_loc {
                    // For simplicity, just check if locations are different
                    // In production, you'd calculate actual distance using coordinates
                    let input = CreateSecurityAlertInput {
                        user_id: Some(user_id),
                        tenant_id: event.tenant_id,
                        alert_type: SecurityAlertType::ImpossibleTravel,
                        severity: AlertSeverity::High,
                        details: Some(serde_json::json!({
                            "previous_location": last_loc,
                            "current_location": current_loc,
                            "time_difference_minutes": time_diff.num_minutes(),
                            "previous_ip": last.ip_address,
                            "current_ip": event.ip_address,
                        })),
                    };

                    let alert = self.security_alert_repo.create(&input).await?;
                    return Ok(Some(alert));
                }
            }
        }

        Ok(None)
    }

    /// List unresolved security alerts
    pub async fn list_unresolved(
        &self,
        page: i64,
        per_page: i64,
    ) -> Result<(Vec<SecurityAlert>, i64)> {
        let offset = (page - 1) * per_page;
        let alerts = self
            .security_alert_repo
            .list_unresolved(offset, per_page)
            .await?;
        let total = self.security_alert_repo.count_unresolved().await?;
        Ok((alerts, total))
    }

    /// List all security alerts
    pub async fn list(&self, page: i64, per_page: i64) -> Result<(Vec<SecurityAlert>, i64)> {
        let offset = (page - 1) * per_page;
        let alerts = self.security_alert_repo.list(offset, per_page).await?;
        let total = self.security_alert_repo.count().await?;
        Ok((alerts, total))
    }

    /// Resolve a security alert
    pub async fn resolve(
        &self,
        alert_id: StringUuid,
        resolved_by: StringUuid,
    ) -> Result<SecurityAlert> {
        self.security_alert_repo
            .resolve(alert_id, resolved_by)
            .await
    }

    /// Get a security alert by ID
    pub async fn get(&self, id: StringUuid) -> Result<SecurityAlert> {
        self.security_alert_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::NotFound(format!("Security alert {} not found", id))
            })
    }

    /// Clean up old resolved alerts
    pub async fn cleanup_old_alerts(&self, days: i64) -> Result<u64> {
        self.security_alert_repo.delete_old(days).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::login_event::MockLoginEventRepository;
    use crate::repository::security_alert::MockSecurityAlertRepository;
    use crate::repository::webhook::MockWebhookRepository;
    use mockall::predicate::*;

    #[test]
    fn test_default_config() {
        let config = SecurityDetectionConfig::default();
        assert_eq!(config.brute_force_threshold, 5);
        assert_eq!(config.brute_force_window_mins, 10);
        assert_eq!(config.password_spray_threshold, 5);
        assert_eq!(config.impossible_travel_distance_km, 500.0);
    }

    #[tokio::test]
    async fn test_list_unresolved_alerts() {
        let login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        alert_mock
            .expect_list_unresolved()
            .with(eq(0), eq(10))
            .returning(|_, _| {
                Ok(vec![SecurityAlert {
                    alert_type: SecurityAlertType::BruteForce,
                    severity: AlertSeverity::High,
                    ..Default::default()
                }])
            });

        alert_mock.expect_count_unresolved().returning(|| Ok(1));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let (alerts, total) = service.list_unresolved(1, 10).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(total, 1);
    }

    #[tokio::test]
    async fn test_resolve_alert() {
        let login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        let alert_id = StringUuid::new_v4();
        let resolved_by = StringUuid::new_v4();

        alert_mock
            .expect_resolve()
            .with(eq(alert_id), eq(resolved_by))
            .returning(|id, by| {
                Ok(SecurityAlert {
                    id,
                    resolved_by: Some(by),
                    resolved_at: Some(Utc::now()),
                    ..Default::default()
                })
            });

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let alert = service.resolve(alert_id, resolved_by).await.unwrap();
        assert!(alert.resolved_at.is_some());
        assert_eq!(alert.resolved_by, Some(resolved_by));
    }

    #[tokio::test]
    async fn test_analyze_login_event_brute_force_detected() {
        let mut login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let mut webhook_mock = MockWebhookRepository::new();

        // Mock failed attempts exceeding threshold
        login_mock
            .expect_count_failed_by_ip()
            .returning(|_, _| Ok(10)); // Above threshold of 5

        // No password spray
        login_mock
            .expect_count_failed_by_ip_multi_user()
            .returning(|_, _| Ok(1));

        // Expect alert creation for brute force
        alert_mock.expect_create().returning(|input| {
            Ok(SecurityAlert {
                id: StringUuid::new_v4(),
                user_id: input.user_id,
                tenant_id: input.tenant_id,
                alert_type: input.alert_type.clone(),
                severity: input.severity.clone(),
                details: input.details.clone(),
                ..Default::default()
            })
        });

        // Mock webhook - no webhooks configured
        webhook_mock
            .expect_list_enabled_for_event()
            .returning(|_| Ok(vec![]));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let event = LoginEvent {
            id: 1,
            user_id: Some(StringUuid::new_v4()),
            email: Some("test@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::FailedPassword,
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: Some("invalid_password".to_string()),
            created_at: Utc::now(),
        };

        let alerts = service.analyze_login_event(&event).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, SecurityAlertType::BruteForce);
        assert_eq!(alerts[0].severity, AlertSeverity::High);
    }

    #[tokio::test]
    async fn test_analyze_login_event_password_spray_detected() {
        let mut login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let mut webhook_mock = MockWebhookRepository::new();

        // No brute force
        login_mock
            .expect_count_failed_by_ip()
            .returning(|_, _| Ok(2));

        // Password spray detected - multiple accounts from same IP
        login_mock
            .expect_count_failed_by_ip_multi_user()
            .returning(|_, _| Ok(10)); // Above threshold of 5

        // Expect alert creation for password spray
        alert_mock.expect_create().returning(|input| {
            Ok(SecurityAlert {
                id: StringUuid::new_v4(),
                alert_type: input.alert_type.clone(),
                severity: input.severity.clone(),
                ..Default::default()
            })
        });

        // Mock webhook - no webhooks configured
        webhook_mock
            .expect_list_enabled_for_event()
            .returning(|_| Ok(vec![]));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let event = LoginEvent {
            id: 1,
            user_id: None,
            email: None,
            tenant_id: None,
            event_type: LoginEventType::FailedPassword,
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: None,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };

        let alerts = service.analyze_login_event(&event).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, SecurityAlertType::SuspiciousIp);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }

    #[tokio::test]
    async fn test_analyze_login_event_new_device_detected() {
        let mut login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let mut webhook_mock = MockWebhookRepository::new();

        let user_id = StringUuid::new_v4();

        // For success events, still check brute force first (no IP in this case)
        login_mock
            .expect_count_failed_by_ip()
            .returning(|_, _| Ok(0));
        login_mock
            .expect_count_failed_by_ip_multi_user()
            .returning(|_, _| Ok(0));

        // For new device check - return existing logins with different user agent
        login_mock.expect_list_by_user().returning(move |_, _, _| {
            Ok(vec![LoginEvent {
                id: 2,
                user_id: Some(user_id),
                email: Some("test@example.com".to_string()),
                tenant_id: None,
                event_type: LoginEventType::Success,
                ip_address: Some("192.168.1.1".to_string()),
                user_agent: Some("OldBrowser/1.0".to_string()), // Different from new event
                device_type: None,
                location: None,
                session_id: None,
                failure_reason: None,
                created_at: Utc::now() - Duration::hours(1),
            }])
        });

        // Expect new device alert creation
        alert_mock.expect_create().returning(|input| {
            Ok(SecurityAlert {
                id: StringUuid::new_v4(),
                alert_type: input.alert_type.clone(),
                severity: input.severity.clone(),
                ..Default::default()
            })
        });

        // Mock webhook - no webhooks configured
        webhook_mock
            .expect_list_enabled_for_event()
            .returning(|_| Ok(vec![]));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let event = LoginEvent {
            id: 1,
            user_id: Some(user_id),
            email: Some("test@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::Success,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("NewBrowser/2.0".to_string()), // New user agent
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };

        let alerts = service.analyze_login_event(&event).await.unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, SecurityAlertType::NewDevice);
        assert_eq!(alerts[0].severity, AlertSeverity::Medium);
    }

    #[tokio::test]
    async fn test_analyze_login_event_impossible_travel_detected() {
        let mut login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let mut webhook_mock = MockWebhookRepository::new();

        let user_id = StringUuid::new_v4();

        // For success events, still check brute force first
        login_mock
            .expect_count_failed_by_ip()
            .returning(|_, _| Ok(0));
        login_mock
            .expect_count_failed_by_ip_multi_user()
            .returning(|_, _| Ok(0));

        // For impossible travel check - return recent login from different location
        login_mock.expect_list_by_user().returning(move |_, _, _| {
            Ok(vec![LoginEvent {
                id: 2,
                user_id: Some(user_id),
                email: Some("test@example.com".to_string()),
                tenant_id: None,
                event_type: LoginEventType::Success,
                ip_address: Some("192.168.1.1".to_string()),
                user_agent: Some("Mozilla/5.0".to_string()),
                device_type: None,
                location: Some("New York, US".to_string()), // Different location
                session_id: None,
                failure_reason: None,
                created_at: Utc::now() - Duration::minutes(30), // Only 30 minutes ago
            }])
        });

        // Expect impossible travel alert creation
        alert_mock.expect_create().returning(|input| {
            Ok(SecurityAlert {
                id: StringUuid::new_v4(),
                alert_type: input.alert_type.clone(),
                severity: input.severity.clone(),
                ..Default::default()
            })
        });

        // Mock webhook - no webhooks configured
        webhook_mock
            .expect_list_enabled_for_event()
            .returning(|_| Ok(vec![]));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let event = LoginEvent {
            id: 1,
            user_id: Some(user_id),
            email: Some("test@example.com".to_string()),
            tenant_id: None,
            event_type: LoginEventType::Success,
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            device_type: None,
            location: Some("Tokyo, Japan".to_string()), // Very different location
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };

        let alerts = service.analyze_login_event(&event).await.unwrap();
        // Should have impossible travel alert (same user agent passes device check)
        assert!(alerts.iter().any(|a| a.alert_type == SecurityAlertType::ImpossibleTravel));
    }

    #[tokio::test]
    async fn test_analyze_login_event_no_alerts() {
        let mut login_mock = MockLoginEventRepository::new();
        let alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        // Below thresholds
        login_mock
            .expect_count_failed_by_ip()
            .returning(|_, _| Ok(2));
        login_mock
            .expect_count_failed_by_ip_multi_user()
            .returning(|_, _| Ok(1));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let event = LoginEvent {
            id: 1,
            user_id: None,
            email: None,
            tenant_id: None,
            event_type: LoginEventType::FailedPassword,
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: None,
            device_type: None,
            location: None,
            session_id: None,
            failure_reason: None,
            created_at: Utc::now(),
        };

        let alerts = service.analyze_login_event(&event).await.unwrap();
        assert_eq!(alerts.len(), 0);
    }

    #[tokio::test]
    async fn test_list_alerts_pagination() {
        let login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        // Page 2 with 5 per page = offset 5
        alert_mock
            .expect_list()
            .with(eq(5), eq(5))
            .returning(|_, _| {
                Ok(vec![
                    SecurityAlert {
                        alert_type: SecurityAlertType::BruteForce,
                        ..Default::default()
                    },
                    SecurityAlert {
                        alert_type: SecurityAlertType::NewDevice,
                        ..Default::default()
                    },
                ])
            });

        alert_mock.expect_count().returning(|| Ok(12));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let (alerts, total) = service.list(2, 5).await.unwrap();
        assert_eq!(alerts.len(), 2);
        assert_eq!(total, 12);
    }

    #[tokio::test]
    async fn test_get_alert_success() {
        let login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        let alert_id = StringUuid::new_v4();

        alert_mock.expect_find_by_id().with(eq(alert_id)).returning(
            move |id| {
                Ok(Some(SecurityAlert {
                    id,
                    alert_type: SecurityAlertType::BruteForce,
                    severity: AlertSeverity::High,
                    ..Default::default()
                }))
            },
        );

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let alert = service.get(alert_id).await.unwrap();
        assert_eq!(alert.id, alert_id);
        assert_eq!(alert.alert_type, SecurityAlertType::BruteForce);
    }

    #[tokio::test]
    async fn test_get_alert_not_found() {
        let login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        let alert_id = StringUuid::new_v4();

        alert_mock
            .expect_find_by_id()
            .with(eq(alert_id))
            .returning(|_| Ok(None));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let result = service.get(alert_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_old_alerts() {
        let login_mock = MockLoginEventRepository::new();
        let mut alert_mock = MockSecurityAlertRepository::new();
        let webhook_mock = MockWebhookRepository::new();

        alert_mock.expect_delete_old().with(eq(30)).returning(|_| Ok(5));

        let webhook_service = Arc::new(WebhookService::new(Arc::new(webhook_mock)));
        let service = SecurityDetectionService::new(
            Arc::new(login_mock),
            Arc::new(alert_mock),
            webhook_service,
            SecurityDetectionConfig::default(),
        );

        let deleted = service.cleanup_old_alerts(30).await.unwrap();
        assert_eq!(deleted, 5);
    }

    #[test]
    fn test_custom_config() {
        let config = SecurityDetectionConfig {
            brute_force_threshold: 10,
            brute_force_window_mins: 5,
            password_spray_threshold: 3,
            password_spray_window_mins: 15,
            impossible_travel_distance_km: 1000.0,
        };

        assert_eq!(config.brute_force_threshold, 10);
        assert_eq!(config.password_spray_threshold, 3);
        assert_eq!(config.impossible_travel_distance_km, 1000.0);
    }
}
