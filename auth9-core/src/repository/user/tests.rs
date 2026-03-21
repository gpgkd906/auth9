//! User repository tests

use super::*;
use mockall::predicate::*;

#[tokio::test]
async fn test_mock_user_repository_find_by_id() {
    let mut mock = MockUserRepository::new();
    let user = User::default();
    let user_clone = user.clone();
    let id = user.id;

    mock.expect_find_by_id()
        .with(eq(id))
        .returning(move |_| Ok(Some(user_clone.clone())));

    let result = mock.find_by_id(id).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().id, id);
}

#[tokio::test]
async fn test_mock_user_repository_find_by_id_not_found() {
    let mut mock = MockUserRepository::new();
    let id = StringUuid::new_v4();

    mock.expect_find_by_id()
        .with(eq(id))
        .returning(|_| Ok(None));

    let result = mock.find_by_id(id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_mock_user_repository_find_by_email() {
    let mut mock = MockUserRepository::new();
    let user = User {
        email: "test@example.com".to_string(),
        ..Default::default()
    };
    let user_clone = user.clone();

    mock.expect_find_by_email()
        .with(eq("test@example.com"))
        .returning(move |_| Ok(Some(user_clone.clone())));

    let result = mock.find_by_email("test@example.com").await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().email, "test@example.com");
}

#[tokio::test]
async fn test_mock_user_repository_create() {
    let mut mock = MockUserRepository::new();
    let identity_subject = "subj-123";
    let input = CreateUserInput {
        email: "new@example.com".to_string(),
        display_name: Some("New User".to_string()),
        avatar_url: None,
    };

    mock.expect_create().returning(|_, input| {
        Ok(User {
            email: input.email.clone(),
            display_name: input.display_name.clone(),
            ..Default::default()
        })
    });

    let result = mock.create(identity_subject, &input).await.unwrap();
    assert_eq!(result.email, "new@example.com");
    assert_eq!(result.display_name, Some("New User".to_string()));
}

#[tokio::test]
async fn test_mock_user_repository_list() {
    let mut mock = MockUserRepository::new();

    mock.expect_list().with(eq(0), eq(10)).returning(|_, _| {
        Ok(vec![
            User {
                email: "user1@example.com".to_string(),
                ..Default::default()
            },
            User {
                email: "user2@example.com".to_string(),
                ..Default::default()
            },
        ])
    });

    let result = mock.list(0, 10).await.unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_mock_user_repository_count() {
    let mut mock = MockUserRepository::new();

    mock.expect_count().returning(|| Ok(42));

    let result = mock.count().await.unwrap();
    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_mock_user_repository_delete() {
    let mut mock = MockUserRepository::new();
    let id = StringUuid::new_v4();

    mock.expect_delete().with(eq(id)).returning(|_| Ok(()));

    let result = mock.delete(id).await;
    assert!(result.is_ok());
}
