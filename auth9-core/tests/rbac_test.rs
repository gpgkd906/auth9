//! RBAC repository integration tests

use auth9_core::domain::{
    AssignRolesInput, CreatePermissionInput, CreateRoleInput, CreateServiceInput,
    CreateTenantInput, CreateUserInput, AddUserToTenantInput, UpdateRoleInput,
};
use auth9_core::repository::rbac::RbacRepositoryImpl;
use auth9_core::repository::service::ServiceRepositoryImpl;
use auth9_core::repository::tenant::TenantRepositoryImpl;
use auth9_core::repository::user::UserRepositoryImpl;
use auth9_core::repository::{RbacRepository, ServiceRepository, TenantRepository, UserRepository};
use uuid::Uuid;

mod common;

/// Test helper to set up a tenant, user, and service
async fn setup_test_entities(
    pool: &sqlx::MySqlPool,
) -> (auth9_core::domain::Tenant, auth9_core::domain::User, auth9_core::domain::Service) {
    let tenant_repo = TenantRepositoryImpl::new(pool.clone());
    let user_repo = UserRepositoryImpl::new(pool.clone());
    let service_repo = ServiceRepositoryImpl::new(pool.clone());

    let tenant = tenant_repo
        .create(&CreateTenantInput {
            name: "RBAC Test Tenant".to_string(),
            slug: format!("rbac-test-{}", Uuid::new_v4()),
            logo_url: None,
            settings: None,
        })
        .await
        .unwrap();

    let user = user_repo
        .create(
            &format!("keycloak-rbac-{}", Uuid::new_v4()),
            &CreateUserInput {
                email: format!("rbac-{}@example.com", Uuid::new_v4()),
                display_name: Some("RBAC Test User".to_string()),
                avatar_url: None,
            },
        )
        .await
        .unwrap();

    let service = service_repo
        .create(&CreateServiceInput {
            tenant_id: Some(Uuid::parse_str(&tenant.id.to_string()).unwrap()),
            name: "RBAC Test Service".to_string(),
            client_id: format!("rbac-client-{}", Uuid::new_v4()),
            base_url: None,
            redirect_uris: vec![],
            logout_uris: None,
        })
        .await
        .unwrap();

    (tenant, user, service)
}

#[tokio::test]
async fn test_create_and_find_permission() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Create permission - service_id needs Uuid type
    let permission = rbac_repo
        .create_permission(&CreatePermissionInput {
            service_id: *service.id,
            code: "users:read".to_string(),
            name: "Read Users".to_string(),
            description: Some("Permission to read users".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(permission.code, "users:read");
    assert_eq!(permission.name, "Read Users");

    // Find by ID
    let found = rbac_repo.find_permission_by_id(permission.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().code, "users:read");

    // Find by service
    let permissions = rbac_repo
        .find_permissions_by_service(service.id)
        .await
        .unwrap();
    assert_eq!(permissions.len(), 1);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_delete_permission() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    let permission = rbac_repo
        .create_permission(&CreatePermissionInput {
            service_id: *service.id,
            code: "users:delete".to_string(),
            name: "Delete Users".to_string(),
            description: None,
        })
        .await
        .unwrap();

    // Delete
    rbac_repo.delete_permission(permission.id).await.unwrap();

    // Verify deleted
    let found = rbac_repo.find_permission_by_id(permission.id).await.unwrap();
    assert!(found.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_create_and_find_role() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Create role
    let role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Admin".to_string(),
            description: Some("Administrator role".to_string()),
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    assert_eq!(role.name, "Admin");

    // Find by ID
    let found = rbac_repo.find_role_by_id(role.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Admin");

    // Find by service
    let roles = rbac_repo.find_roles_by_service(service.id).await.unwrap();
    assert_eq!(roles.len(), 1);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_update_role() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    let role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Original Role".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    // Update
    let updated = rbac_repo
        .update_role(
            role.id,
            &UpdateRoleInput {
                name: Some("Updated Role".to_string()),
                description: Some("Updated description".to_string()),
                parent_role_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Role");
    assert_eq!(updated.description, Some("Updated description".to_string()));

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_delete_role() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    let role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Delete Me".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    rbac_repo.delete_role(role.id).await.unwrap();

    let found = rbac_repo.find_role_by_id(role.id).await.unwrap();
    assert!(found.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_assign_permission_to_role() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Create role and permission
    let role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Role With Perms".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    let permission = rbac_repo
        .create_permission(&CreatePermissionInput {
            service_id: *service.id,
            code: "docs:write".to_string(),
            name: "Write Docs".to_string(),
            description: None,
        })
        .await
        .unwrap();

    // Assign permission to role
    rbac_repo
        .assign_permission_to_role(role.id, permission.id)
        .await
        .unwrap();

    // Find role permissions
    let role_perms = rbac_repo.find_role_permissions(role.id).await.unwrap();
    assert_eq!(role_perms.len(), 1);
    assert_eq!(role_perms[0].code, "docs:write");

    // Remove permission
    rbac_repo
        .remove_permission_from_role(role.id, permission.id)
        .await
        .unwrap();

    let role_perms_after = rbac_repo.find_role_permissions(role.id).await.unwrap();
    assert_eq!(role_perms_after.len(), 0);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_assign_role_to_user_in_tenant() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (tenant, user, service) = setup_test_entities(&pool).await;
    let user_repo = UserRepositoryImpl::new(pool.clone());
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Add user to tenant first - use Uuid types
    user_repo
        .add_to_tenant(&AddUserToTenantInput {
            user_id: *user.id,
            tenant_id: *tenant.id,
            role_in_tenant: "member".to_string(),
        })
        .await
        .unwrap();

    // Create roles
    let admin_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Admin".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    let viewer_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Viewer".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    // Assign roles to user - use Uuid types
    rbac_repo
        .assign_roles_to_user(
            &AssignRolesInput {
                user_id: *user.id,
                tenant_id: *tenant.id,
                role_ids: vec![*admin_role.id, *viewer_role.id],
            },
            None,
        )
        .await
        .unwrap();

    // Find user roles in tenant
    let user_roles = rbac_repo
        .find_user_roles_in_tenant(user.id, tenant.id)
        .await
        .unwrap();

    assert_eq!(user_roles.user_id, *user.id);
    assert_eq!(user_roles.tenant_id, *tenant.id);
    assert_eq!(user_roles.roles.len(), 2);

    // Find user roles for specific service
    let service_roles = rbac_repo
        .find_user_roles_in_tenant_for_service(user.id, tenant.id, service.id)
        .await
        .unwrap();
    assert_eq!(service_roles.roles.len(), 2);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_role_inheritance() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (_, _, service) = setup_test_entities(&pool).await;
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Create parent role
    let viewer_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Viewer".to_string(),
            description: Some("Basic viewer".to_string()),
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    // Create child role that inherits from viewer
    let editor_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Editor".to_string(),
            description: Some("Can edit, inherits viewer".to_string()),
            parent_role_id: Some(*viewer_role.id),
            permission_ids: None,
        })
        .await
        .unwrap();

    assert!(editor_role.parent_role_id.is_some());
    assert_eq!(editor_role.parent_role_id.unwrap(), viewer_role.id);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_find_tenant_user_id() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (tenant, user, _) = setup_test_entities(&pool).await;
    let user_repo = UserRepositoryImpl::new(pool.clone());
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Before adding user to tenant, find_tenant_user_id should return None
    let not_found = rbac_repo
        .find_tenant_user_id(user.id, tenant.id)
        .await
        .unwrap();
    assert!(not_found.is_none());

    // Add user to tenant
    let tenant_user = user_repo
        .add_to_tenant(&AddUserToTenantInput {
            user_id: *user.id,
            tenant_id: *tenant.id,
            role_in_tenant: "member".to_string(),
        })
        .await
        .unwrap();

    // Now find_tenant_user_id should return the tenant_user id
    let found = rbac_repo
        .find_tenant_user_id(user.id, tenant.id)
        .await
        .unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap(), tenant_user.id);

    // Test with non-existent user_id
    let non_existent_user = auth9_core::domain::StringUuid::new_v4();
    let not_found2 = rbac_repo
        .find_tenant_user_id(non_existent_user, tenant.id)
        .await
        .unwrap();
    assert!(not_found2.is_none());

    // Test with non-existent tenant_id
    let non_existent_tenant = auth9_core::domain::StringUuid::new_v4();
    let not_found3 = rbac_repo
        .find_tenant_user_id(user.id, non_existent_tenant)
        .await
        .unwrap();
    assert!(not_found3.is_none());

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_remove_role_from_user() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (tenant, user, service) = setup_test_entities(&pool).await;
    let user_repo = UserRepositoryImpl::new(pool.clone());
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Add user to tenant
    let tenant_user = user_repo
        .add_to_tenant(&AddUserToTenantInput {
            user_id: *user.id,
            tenant_id: *tenant.id,
            role_in_tenant: "member".to_string(),
        })
        .await
        .unwrap();

    // Create two roles
    let admin_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Admin".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    let viewer_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Viewer".to_string(),
            description: None,
            parent_role_id: None,
            permission_ids: None,
        })
        .await
        .unwrap();

    // Assign both roles to user
    rbac_repo
        .assign_roles_to_user(
            &AssignRolesInput {
                user_id: *user.id,
                tenant_id: *tenant.id,
                role_ids: vec![*admin_role.id, *viewer_role.id],
            },
            None,
        )
        .await
        .unwrap();

    // Verify user has 2 roles
    let roles_before = rbac_repo
        .find_user_roles_in_tenant(user.id, tenant.id)
        .await
        .unwrap();
    assert_eq!(roles_before.roles.len(), 2);

    // Remove the admin role from user
    rbac_repo
        .remove_role_from_user(tenant_user.id, admin_role.id)
        .await
        .unwrap();

    // Verify user now has only 1 role
    let roles_after = rbac_repo
        .find_user_roles_in_tenant(user.id, tenant.id)
        .await
        .unwrap();
    assert_eq!(roles_after.roles.len(), 1);
    assert!(roles_after.roles.contains(&"Viewer".to_string()));
    assert!(!roles_after.roles.contains(&"Admin".to_string()));

    // Remove the remaining role
    rbac_repo
        .remove_role_from_user(tenant_user.id, viewer_role.id)
        .await
        .unwrap();

    // Verify user now has no roles
    let roles_empty = rbac_repo
        .find_user_roles_in_tenant(user.id, tenant.id)
        .await
        .unwrap();
    assert_eq!(roles_empty.roles.len(), 0);

    common::cleanup_database(&pool).await.unwrap();
}

#[tokio::test]
async fn test_find_user_role_records_in_tenant() {
    let pool = match common::get_test_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Skipping test: could not connect to database: {}", e);
            return;
        }
    };

    common::setup_database(&pool).await.unwrap();
    common::cleanup_database(&pool).await.unwrap();

    let (tenant, user, service) = setup_test_entities(&pool).await;
    let user_repo = UserRepositoryImpl::new(pool.clone());
    let rbac_repo = RbacRepositoryImpl::new(pool.clone());

    // Add user to tenant
    user_repo
        .add_to_tenant(&AddUserToTenantInput {
            user_id: *user.id,
            tenant_id: *tenant.id,
            role_in_tenant: "member".to_string(),
        })
        .await
        .unwrap();

    // Create a role with permission
    let permission = rbac_repo
        .create_permission(&CreatePermissionInput {
            service_id: *service.id,
            code: "reports:view".to_string(),
            name: "View Reports".to_string(),
            description: None,
        })
        .await
        .unwrap();

    let analyst_role = rbac_repo
        .create_role(&CreateRoleInput {
            service_id: *service.id,
            name: "Analyst".to_string(),
            description: Some("Data analyst role".to_string()),
            parent_role_id: None,
            permission_ids: Some(vec![*permission.id]),
        })
        .await
        .unwrap();

    // Assign role to user
    rbac_repo
        .assign_roles_to_user(
            &AssignRolesInput {
                user_id: *user.id,
                tenant_id: *tenant.id,
                role_ids: vec![*analyst_role.id],
            },
            None,
        )
        .await
        .unwrap();

    // Test find_user_role_records_in_tenant without service filter
    let role_records = rbac_repo
        .find_user_role_records_in_tenant(user.id, tenant.id, None)
        .await
        .unwrap();
    assert_eq!(role_records.len(), 1);
    assert_eq!(role_records[0].name, "Analyst");
    assert_eq!(role_records[0].id, analyst_role.id);

    // Test find_user_role_records_in_tenant with service filter
    let role_records_filtered = rbac_repo
        .find_user_role_records_in_tenant(user.id, tenant.id, Some(service.id))
        .await
        .unwrap();
    assert_eq!(role_records_filtered.len(), 1);
    assert_eq!(role_records_filtered[0].name, "Analyst");

    // Test with different service_id (should return empty)
    let other_service_id = auth9_core::domain::StringUuid::new_v4();
    let role_records_other = rbac_repo
        .find_user_role_records_in_tenant(user.id, tenant.id, Some(other_service_id))
        .await
        .unwrap();
    assert_eq!(role_records_other.len(), 0);

    common::cleanup_database(&pool).await.unwrap();
}
