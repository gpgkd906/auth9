//! get_user_roles gRPC method tests

use super::*;
use auth9_core::grpc::proto::token_exchange_server::TokenExchange;
use auth9_core::grpc::proto::GetUserRolesRequest;
use tonic::Request;
use uuid::Uuid;

#[tokio::test]
async fn test_get_user_roles_success() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let rbac_repo = builder.rbac_repo.clone();

    rbac_repo
        .set_user_roles(
            user_id,
            tenant_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["admin".to_string(), "viewer".to_string()],
                vec!["user:read".to_string(), "user:write".to_string()],
            ),
        )
        .await;
    rbac_repo
        .add_role(create_test_role(role_id, service_id, "admin"))
        .await;

    let service = builder.build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_ok(), "Expected success but got: {:?}", response.err());

    let response = response.unwrap().into_inner();
    assert!(!response.roles.is_empty());
    assert!(!response.permissions.is_empty());
}

#[tokio::test]
async fn test_get_user_roles_with_service_filter() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let rbac_repo = builder.rbac_repo.clone();

    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["admin".to_string()],
                vec!["service:manage".to_string()],
            ),
        )
        .await;
    rbac_repo
        .add_role(create_test_role(role_id, service_id, "admin"))
        .await;

    let service = builder.build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: service_id.to_string(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.roles.len(), 1);
    assert_eq!(response.roles[0].name, "admin");
}

#[tokio::test]
async fn test_get_user_roles_invalid_user_id() {
    let tenant_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: "invalid-uuid".to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_user_roles_invalid_tenant_id() {
    let user_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: "invalid-uuid".to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_user_roles_invalid_service_id() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: "not-a-uuid".to_string(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_user_roles_empty_user_id() {
    let tenant_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: "".to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_user_roles_empty_tenant_id() {
    let user_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: "".to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_get_user_roles_empty_result() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.roles.is_empty());
    assert!(response.permissions.is_empty());
}

#[tokio::test]
async fn test_get_user_roles_with_multiple_role_records() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id_1 = Uuid::new_v4();
    let role_id_2 = Uuid::new_v4();
    let role_id_3 = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let rbac_repo = builder.rbac_repo.clone();

    rbac_repo
        .set_user_roles(
            user_id,
            tenant_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["admin".to_string(), "editor".to_string(), "viewer".to_string()],
                vec!["read".to_string(), "write".to_string()],
            ),
        )
        .await;

    // Add multiple role records with hierarchy
    let mut role1 = create_test_role(role_id_1, service_id, "admin");
    role1.description = Some("Full admin".to_string());
    rbac_repo.add_role(role1).await;

    let mut role2 = create_test_role(role_id_2, service_id, "editor");
    role2.description = Some("Content editor".to_string());
    role2.parent_role_id = Some(StringUuid::from(role_id_1));
    rbac_repo.add_role(role2).await;

    let mut role3 = create_test_role(role_id_3, service_id, "viewer");
    role3.parent_role_id = Some(StringUuid::from(role_id_2));
    rbac_repo.add_role(role3).await;

    let service = builder.build_with_noop_cache();

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.roles.len(), 3);
    assert_eq!(response.permissions.len(), 2);

    let role_names: Vec<&str> = response.roles.iter().map(|r| r.name.as_str()).collect();
    assert!(role_names.contains(&"admin"));
    assert!(role_names.contains(&"editor"));
    assert!(role_names.contains(&"viewer"));
}

#[tokio::test]
async fn test_get_user_roles_cache_hit() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let rbac_repo = builder.rbac_repo.clone();

    // Only add role record (no user_roles in repo)
    rbac_repo
        .add_role(create_test_role(role_id, service_id, "cached-admin"))
        .await;

    // Pre-populate cache
    let mock_cache = MockCacheManager::new();
    mock_cache
        .seed_roles(
            user_id,
            tenant_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["cached-admin".to_string()],
                vec!["cached:permission".to_string()],
            ),
        )
        .await;

    let service = builder.build_with_mock_cache(mock_cache);

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: String::new(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    // Should get cached permissions
    assert_eq!(response.permissions, vec!["cached:permission"]);
}

#[tokio::test]
async fn test_get_user_roles_with_service_filter_cache_hit() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let rbac_repo = builder.rbac_repo.clone();

    rbac_repo
        .add_role(create_test_role(role_id, service_id, "service-admin"))
        .await;

    // Pre-populate cache for specific service
    let mock_cache = MockCacheManager::new();
    mock_cache
        .seed_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["service-admin".to_string()],
                vec!["service:cached".to_string()],
            ),
        )
        .await;

    let service = builder.build_with_mock_cache(mock_cache);

    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: service_id.to_string(),
    });

    let response = service.get_user_roles(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert_eq!(response.permissions, vec!["service:cached"]);
}
