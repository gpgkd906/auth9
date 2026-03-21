//! Login event repository tests

use super::*;
use mockall::predicate::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_mock_login_event_repository() {
    let mut mock = MockLoginEventRepository::new();

    mock.expect_count().returning(|| Ok(100));

    let count = mock.count().await.unwrap();
    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_mock_create() {
    let mut mock = MockLoginEventRepository::new();

    mock.expect_create().returning(|_| Ok(1));

    let input = CreateLoginEventInput {
        user_id: Some(StringUuid::new_v4()),
        email: Some("test@example.com".to_string()),
        tenant_id: None,
        event_type: LoginEventType::Success,
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: None,
        device_type: Some("desktop".to_string()),
        location: None,
        session_id: None,
        failure_reason: None,
        provider_alias: None,
        provider_type: None,
    };

    let id = mock.create(&input).await.unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn test_mock_get_stats() {
    let mut mock = MockLoginEventRepository::new();
    let start = Utc::now() - chrono::Duration::days(7);
    let end = Utc::now();

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

    let stats = mock.get_stats(None, start, end).await.unwrap();
    assert_eq!(stats.total_logins, 100);
    assert_eq!(stats.successful_logins, 80);
}

#[tokio::test]
async fn test_mock_count_failed_by_ip() {
    let mut mock = MockLoginEventRepository::new();
    let since = Utc::now() - chrono::Duration::minutes(10);

    mock.expect_count_failed_by_ip()
        .with(eq("192.168.1.1"), always())
        .returning(|_, _| Ok(5));

    let count = mock.count_failed_by_ip("192.168.1.1", since).await.unwrap();
    assert_eq!(count, 5);
}
