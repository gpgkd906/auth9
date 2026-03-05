//! RBAC repository tests

use super::*;
use mockall::predicate::*;

#[tokio::test]
async fn test_mock_rbac_find_permission_by_id() {
    let mut mock = MockRbacRepository::new();
    let permission = Permission::default();
    let permission_clone = permission.clone();
    let id = permission.id;

    mock.expect_find_permission_by_id()
        .with(eq(id))
        .returning(move |_| Ok(Some(permission_clone.clone())));

    let result = mock.find_permission_by_id(id).await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_mock_rbac_find_permissions_by_service() {
    let mut mock = MockRbacRepository::new();
    let service_id = StringUuid::new_v4();

    mock.expect_find_permissions_by_service()
        .with(eq(service_id))
        .returning(|_| {
            Ok(vec![
                Permission {
                    code: "read".to_string(),
                    name: "Read".to_string(),
                    ..Default::default()
                },
                Permission {
                    code: "write".to_string(),
                    name: "Write".to_string(),
                    ..Default::default()
                },
            ])
        });

    let result = mock.find_permissions_by_service(service_id).await.unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_mock_rbac_find_role_by_id() {
    let mut mock = MockRbacRepository::new();
    let role = Role::default();
    let role_clone = role.clone();
    let id = role.id;

    mock.expect_find_role_by_id()
        .with(eq(id))
        .returning(move |_| Ok(Some(role_clone.clone())));

    let result = mock.find_role_by_id(id).await.unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_mock_rbac_find_roles_by_service() {
    let mut mock = MockRbacRepository::new();
    let service_id = StringUuid::new_v4();

    mock.expect_find_roles_by_service()
        .with(eq(service_id))
        .returning(|_| {
            Ok(vec![
                Role {
                    name: "admin".to_string(),
                    ..Default::default()
                },
                Role {
                    name: "viewer".to_string(),
                    ..Default::default()
                },
            ])
        });

    let result = mock.find_roles_by_service(service_id).await.unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_mock_rbac_find_role_permissions() {
    let mut mock = MockRbacRepository::new();
    let role_id = StringUuid::new_v4();

    mock.expect_find_role_permissions()
        .with(eq(role_id))
        .returning(|_| {
            Ok(vec![Permission {
                code: "user:read".to_string(),
                ..Default::default()
            }])
        });

    let result = mock.find_role_permissions(role_id).await.unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].code, "user:read");
}

#[tokio::test]
async fn test_mock_rbac_find_user_roles_in_tenant() {
    let mut mock = MockRbacRepository::new();
    let user_id = StringUuid::new_v4();
    let tenant_id = StringUuid::new_v4();

    mock.expect_find_user_roles_in_tenant()
        .with(eq(user_id), eq(tenant_id))
        .returning(|uid, tid| {
            Ok(UserRolesInTenant {
                user_id: *uid,
                tenant_id: *tid,
                roles: vec!["admin".to_string()],
                permissions: vec!["read".to_string(), "write".to_string()],
            })
        });

    let result = mock
        .find_user_roles_in_tenant(user_id, tenant_id)
        .await
        .unwrap();
    assert_eq!(result.roles, vec!["admin"]);
    assert_eq!(result.permissions.len(), 2);
}

#[tokio::test]
async fn test_mock_rbac_assign_permission_to_role() {
    let mut mock = MockRbacRepository::new();
    let role_id = StringUuid::new_v4();
    let permission_id = StringUuid::new_v4();

    mock.expect_assign_permission_to_role()
        .with(eq(role_id), eq(permission_id))
        .returning(|_, _| Ok(()));

    let result = mock.assign_permission_to_role(role_id, permission_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_rbac_delete_permission() {
    let mut mock = MockRbacRepository::new();
    let id = StringUuid::new_v4();

    mock.expect_delete_permission()
        .with(eq(id))
        .returning(|_| Ok(()));

    let result = mock.delete_permission(id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mock_rbac_delete_role() {
    let mut mock = MockRbacRepository::new();
    let id = StringUuid::new_v4();

    mock.expect_delete_role().with(eq(id)).returning(|_| Ok(()));

    let result = mock.delete_role(id).await;
    assert!(result.is_ok());
}
