//! Session repository tests

use super::*;
use mockall::predicate::*;

#[tokio::test]
async fn test_mock_session_repository() {
    let mut mock = MockSessionRepository::new();

    let session = Session::default();
    let session_clone = session.clone();

    mock.expect_find_by_id()
        .with(eq(session.id))
        .returning(move |_| Ok(Some(session_clone.clone())));

    let result = mock.find_by_id(session.id).await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_mock_list_active_by_user() {
    let mut mock = MockSessionRepository::new();
    let user_id = StringUuid::new_v4();

    mock.expect_list_active_by_user()
        .with(eq(user_id))
        .returning(|_| Ok(vec![Session::default(), Session::default()]));

    let sessions = mock.list_active_by_user(user_id).await.unwrap();
    assert_eq!(sessions.len(), 2);
}

#[tokio::test]
async fn test_mock_revoke() {
    let mut mock = MockSessionRepository::new();
    let id = StringUuid::new_v4();

    mock.expect_revoke().with(eq(id)).returning(|_| Ok(()));

    let result = mock.revoke(id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_revoke_all_except() {
    let mut mock = MockSessionRepository::new();
    let user_id = StringUuid::new_v4();
    let except_id = StringUuid::new_v4();

    mock.expect_revoke_all_except()
        .with(eq(user_id), eq(except_id))
        .returning(|_, _| Ok(3));

    let count = mock.revoke_all_except(user_id, except_id).await.unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_mock_create() {
    let mut mock = MockSessionRepository::new();
    let user_id = StringUuid::new_v4();

    mock.expect_create().returning(|input| {
        Ok(Session {
            user_id: input.user_id,
            device_type: input.device_type.clone(),
            ..Default::default()
        })
    });

    let input = CreateSessionInput {
        user_id,
        provider_session_id: Some("kc-session-123".to_string()),
        device_type: Some("desktop".to_string()),
        device_name: Some("Chrome".to_string()),
        ip_address: Some("192.168.1.1".to_string()),
        location: None,
        user_agent: None,
    };

    let session = mock.create(&input).await.unwrap();
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.device_type, Some("desktop".to_string()));
}

#[tokio::test]
async fn test_mock_count_active_by_user() {
    let mut mock = MockSessionRepository::new();
    let user_id = StringUuid::new_v4();

    mock.expect_count_active_by_user()
        .with(eq(user_id))
        .returning(|_| Ok(5));

    let count = mock.count_active_by_user(user_id).await.unwrap();
    assert_eq!(count, 5);
}

#[tokio::test]
async fn test_mock_find_oldest_active_by_user() {
    let mut mock = MockSessionRepository::new();
    let user_id = StringUuid::new_v4();
    let session_id = StringUuid::new_v4();

    mock.expect_find_oldest_active_by_user()
        .with(eq(user_id))
        .returning(move |uid| {
            Ok(Some(Session {
                id: session_id,
                user_id: uid,
                device_type: Some("desktop".to_string()),
                ..Default::default()
            }))
        });

    let session = mock.find_oldest_active_by_user(user_id).await.unwrap();
    assert!(session.is_some());
    let s = session.unwrap();
    assert_eq!(s.id, session_id);
    assert_eq!(s.user_id, user_id);
}

#[tokio::test]
async fn test_mock_find_oldest_active_by_user_none() {
    let mut mock = MockSessionRepository::new();
    let user_id = StringUuid::new_v4();

    mock.expect_find_oldest_active_by_user()
        .with(eq(user_id))
        .returning(|_| Ok(None));

    let session = mock.find_oldest_active_by_user(user_id).await.unwrap();
    assert!(session.is_none());
}
