use auth9_core::repository::audit::{AuditLogQuery, AuditRepositoryImpl, CreateAuditLogInput};
use auth9_core::repository::{AuditRepository, UserRepository};
use auth9_core::repository::user::UserRepositoryImpl;
use auth9_core::domain::CreateUserInput;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_create_audit_log() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = AuditRepositoryImpl::new(pool.clone());
    let user_repo = UserRepositoryImpl::new(pool.clone());

    // Create a user for actor_id
    let user = user_repo.create(
        &format!("test-actor-{}", Uuid::new_v4()), 
        &CreateUserInput {
            email: format!("actor-{}@example.com", Uuid::new_v4()),
            display_name: None,
            avatar_url: None,
        }
    ).await.unwrap();
    let actor_id = user.id;

    // Create an audit log entry
    let resource_id = Uuid::new_v4();

    let result = repo
        .create(&CreateAuditLogInput {
            actor_id: Some(*actor_id),
            action: "user.create".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(resource_id),
            old_value: None,
            new_value: Some(serde_json::json!({"email": "test@example.com"})),
            ip_address: Some("192.168.1.1".to_string()),
        })
        .await;

    assert!(result.is_ok());

    // Verify the log was created by querying
    let logs = repo
        .find(&AuditLogQuery {
            actor_id: Some(*actor_id),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action, "user.create");
    assert_eq!(logs[0].resource_type, "user");
    assert_eq!(logs[0].resource_id, Some(resource_id.to_string()));
    assert_eq!(logs[0].ip_address, Some("192.168.1.1".to_string()));

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_create_audit_log_minimal() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = AuditRepositoryImpl::new(pool.clone());

    // Create a minimal audit log entry (no optional fields)
    let result = repo
        .create(&CreateAuditLogInput {
            actor_id: None,
            action: "system.startup".to_string(),
            resource_type: "system".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await;

    assert!(result.is_ok());

    // Verify the log was created
    let logs = repo
        .find(&AuditLogQuery {
            action: Some("system.startup".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert!(logs[0].actor_id.is_none());
    assert!(logs[0].resource_id.is_none());
    assert!(logs[0].ip_address.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_find_audit_logs_with_filters() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = AuditRepositoryImpl::new(pool.clone());
    let user_repo = UserRepositoryImpl::new(pool.clone());

    let actor1_user = user_repo.create(
        &format!("test-actor-{}", Uuid::new_v4()), 
        &CreateUserInput {
            email: format!("actor-{}@example.com", Uuid::new_v4()),
            display_name: None,
            avatar_url: None,
        }
    ).await.unwrap();
    let actor1 = actor1_user.id;

    let actor2_user = user_repo.create(
        &format!("test-actor-{}", Uuid::new_v4()), 
        &CreateUserInput {
            email: format!("actor-{}@example.com", Uuid::new_v4()),
            display_name: None,
            avatar_url: None,
        }
    ).await.unwrap();
    let actor2 = actor2_user.id;
    let resource1 = Uuid::new_v4();

    // Create multiple audit logs
    repo.create(&CreateAuditLogInput {
        actor_id: Some(*actor1),
        action: "tenant.create".to_string(),
        resource_type: "tenant".to_string(),
        resource_id: Some(resource1),
        old_value: None,
        new_value: Some(serde_json::json!({"name": "Tenant 1"})),
        ip_address: Some("10.0.0.1".to_string()),
    })
    .await
    .unwrap();

    repo.create(&CreateAuditLogInput {
        actor_id: Some(*actor1),
        action: "tenant.update".to_string(),
        resource_type: "tenant".to_string(),
        resource_id: Some(resource1),
        old_value: Some(serde_json::json!({"name": "Tenant 1"})),
        new_value: Some(serde_json::json!({"name": "Tenant 1 Updated"})),
        ip_address: Some("10.0.0.1".to_string()),
    })
    .await
    .unwrap();

    repo.create(&CreateAuditLogInput {
        actor_id: Some(*actor2),
        action: "user.create".to_string(),
        resource_type: "user".to_string(),
        resource_id: None,
        old_value: None,
        new_value: None,
        ip_address: Some("10.0.0.2".to_string()),
    })
    .await
    .unwrap();

    // Test filter by actor_id
    let logs_actor1 = repo
        .find(&AuditLogQuery {
            actor_id: Some(*actor1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(logs_actor1.len(), 2);

    // Test filter by resource_type
    let logs_tenant = repo
        .find(&AuditLogQuery {
            resource_type: Some("tenant".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(logs_tenant.len(), 2);

    // Test filter by action
    let logs_create = repo
        .find(&AuditLogQuery {
            action: Some("tenant.create".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(logs_create.len(), 1);
    assert_eq!(logs_create[0].action, "tenant.create");

    // Test filter by resource_id
    let logs_resource = repo
        .find(&AuditLogQuery {
            resource_id: Some(resource1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(logs_resource.len(), 2);

    // Test combined filters
    let logs_combined = repo
        .find(&AuditLogQuery {
            actor_id: Some(*actor1),
            action: Some("tenant.update".to_string()),
            ..Default::default() 
        })
        .await
        .unwrap();
    assert_eq!(logs_combined.len(), 1);

    // Test pagination
    let logs_page1 = repo
        .find(&AuditLogQuery {
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(logs_page1.len(), 2);

    let logs_page2 = repo
        .find(&AuditLogQuery {
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(logs_page2.len(), 1);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_count_audit_logs() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = AuditRepositoryImpl::new(pool.clone());
    let user_repo = UserRepositoryImpl::new(pool.clone());

    let actor1_user = user_repo.create(
        &format!("test-actor-{}", Uuid::new_v4()), 
        &CreateUserInput {
            email: format!("actor-{}@example.com", Uuid::new_v4()),
            display_name: None,
            avatar_url: None,
        }
    ).await.unwrap();
    let actor1 = actor1_user.id;

    // Create multiple audit logs
    for i in 0..5 {
        repo.create(&CreateAuditLogInput {
            actor_id: Some(*actor1),
            action: format!("action.{}", i),
            resource_type: "test".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();
    }

    // Create some logs with different actor
    let actor2_user = user_repo.create(
        &format!("test-actor-{}", Uuid::new_v4()), 
        &CreateUserInput {
            email: format!("actor-{}@example.com", Uuid::new_v4()),
            display_name: None,
            avatar_url: None,
        }
    ).await.unwrap();
    let actor2 = actor2_user.id;
    for i in 0..3 {
        repo.create(&CreateAuditLogInput {
            actor_id: Some(*actor2),
            action: format!("other.{}", i),
            resource_type: "other".to_string(),
            resource_id: None,
            old_value: None,
            new_value: None,
            ip_address: None,
        })
        .await
        .unwrap();
    }

    // Count all
    let total = repo.count(&AuditLogQuery::default()).await.unwrap();
    assert_eq!(total, 8);

    // Count by actor_id
    let count_actor1 = repo
        .count(&AuditLogQuery {
            actor_id: Some(*actor1),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(count_actor1, 5);

    // Count by resource_type
    let count_test = repo
        .count(&AuditLogQuery {
            resource_type: Some("test".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(count_test, 5);

    let count_other = repo
        .count(&AuditLogQuery {
            resource_type: Some("other".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(count_other, 3);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_audit_log_with_json_values() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let repo = AuditRepositoryImpl::new(pool.clone());
    let user_repo = UserRepositoryImpl::new(pool.clone());

    let actor_user = user_repo.create(
        &format!("test-actor-{}", Uuid::new_v4()), 
        &CreateUserInput {
            email: format!("actor-{}@example.com", Uuid::new_v4()),
            display_name: None,
            avatar_url: None,
        }
    ).await.unwrap();
    let actor_id = actor_user.id;
    let old_value = serde_json::json!({
        "name": "Old Name",
        "status": "active",
        "settings": {
            "feature_a": true,
            "limit": 100
        }
    });

    let new_value = serde_json::json!({
        "name": "New Name",
        "status": "inactive",
        "settings": {
            "feature_a": false,
            "limit": 200
        }
    });

    repo.create(&CreateAuditLogInput {
        actor_id: Some(*actor_id),
        action: "tenant.update".to_string(),
        resource_type: "tenant".to_string(),
        resource_id: None,
        old_value: Some(old_value.clone()),
        new_value: Some(new_value.clone()),
        ip_address: None,
    })
    .await
    .unwrap();

    let logs = repo
        .find(&AuditLogQuery {
            actor_id: Some(*actor_id),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(logs.len(), 1);
    assert!(logs[0].old_value.is_some());
    assert!(logs[0].new_value.is_some());

    // Verify JSON structure is preserved
    let retrieved_old = logs[0].old_value.as_ref().unwrap();
    let retrieved_new = logs[0].new_value.as_ref().unwrap();

    assert_eq!(retrieved_old["name"], "Old Name");
    assert_eq!(retrieved_new["name"], "New Name");
    assert_eq!(retrieved_old["settings"]["limit"], 100);
    assert_eq!(retrieved_new["settings"]["limit"], 200);

    common::cleanup_database(&pool).await.unwrap();
}
