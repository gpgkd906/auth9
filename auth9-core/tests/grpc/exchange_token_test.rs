//! exchange_token gRPC method tests

use super::*;
use auth9_core::grpc::proto::token_exchange_server::TokenExchange;
use auth9_core::grpc::proto::ExchangeTokenRequest;
use tonic::Request;
use uuid::Uuid;

#[tokio::test]
async fn test_exchange_token_success() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let service = builder
        .with_user(create_test_user(user_id))
        .await
        .with_service(create_test_service(service_id, tenant_id))
        .await
        .with_client(create_test_client(client_uuid, service_id, "test-client"))
        .await
        .with_user_roles(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["admin".to_string()],
                vec!["user:read".to_string(), "user:write".to_string()],
            ),
        )
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(
        response.is_ok(),
        "Expected success but got: {:?}",
        response.err()
    );

    let response = response.unwrap().into_inner();
    assert!(!response.access_token.is_empty());
    assert_eq!(response.token_type, "Bearer");
    assert!(response.expires_in > 0);
    assert!(!response.refresh_token.is_empty());
}

#[tokio::test]
async fn test_exchange_token_invalid_identity_token() {
    let tenant_id = Uuid::new_v4();

    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token: "invalid-token".to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn test_exchange_token_user_not_found() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    // Empty user repository - user won't be found
    let service = builder.build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_exchange_token_client_not_found() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    // Add user but no client
    let service = builder
        .with_user(create_test_user(user_id))
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "nonexistent-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_exchange_token_invalid_tenant_id() {
    let user_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: "invalid-uuid".to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_exchange_token_empty_tenant_id() {
    let user_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: "".to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn test_exchange_token_empty_service_id() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    let service = builder
        .with_user(create_test_user(user_id))
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn test_exchange_token_with_empty_roles() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    let service = builder
        .with_user(create_test_user(user_id))
        .await
        .with_service(create_test_service(service_id, tenant_id))
        .await
        .with_client(create_test_client(client_uuid, service_id, "test-client"))
        .await
        .with_user_roles(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(user_id, tenant_id, vec![], vec![]),
        )
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_ok());
    assert!(!response.unwrap().into_inner().access_token.is_empty());
}

#[tokio::test]
async fn test_exchange_token_with_multiple_permissions() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "multi-perm@example.com", Some("Multi Perm User"))
        .unwrap();

    let jwt_manager = builder.jwt_manager.clone();
    let service = builder
        .with_user(create_test_user_with_email(
            user_id,
            "multi-perm@example.com",
        ))
        .await
        .with_service(create_test_service(service_id, tenant_id))
        .await
        .with_client(create_test_client(client_uuid, service_id, "test-client"))
        .await
        .with_user_roles(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec![
                    "admin".to_string(),
                    "editor".to_string(),
                    "viewer".to_string(),
                ],
                vec![
                    "users:read".to_string(),
                    "users:write".to_string(),
                    "users:delete".to_string(),
                    "tenants:read".to_string(),
                    "tenants:manage".to_string(),
                ],
            ),
        )
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    let claims = jwt_manager
        .verify_tenant_access_token_with_optional_audience(
            &response.access_token,
            Some("test-client"),
        )
        .unwrap();
    assert_eq!(claims.roles.len(), 3);
    assert_eq!(claims.permissions.len(), 5);
}

#[tokio::test]
async fn test_exchange_token_service_with_null_tenant() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", None)
        .unwrap();

    // Global service without tenant
    let service = builder
        .with_user(create_test_user(user_id))
        .await
        .with_service(create_test_service_without_tenant(service_id))
        .await
        .with_client(create_test_client(client_uuid, service_id, "test-client"))
        .await
        .with_user_roles(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["global-user".to_string()],
                vec!["global:access".to_string()],
            ),
        )
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(
        response.is_ok(),
        "Expected success but got: {:?}",
        response.err()
    );
}

#[tokio::test]
async fn test_exchange_token_with_mfa_enabled_user() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "mfa@example.com", Some("MFA User"))
        .unwrap();

    let mut mfa_user = create_test_user_with_email(user_id, "mfa@example.com");
    mfa_user.mfa_enabled = true;

    let service = builder
        .with_user(mfa_user)
        .await
        .with_service(create_test_service(service_id, tenant_id))
        .await
        .with_client(create_test_client(client_uuid, service_id, "test-client"))
        .await
        .with_user_roles(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["user".to_string()],
                vec!["read".to_string()],
            ),
        )
        .await
        .build_with_noop_cache();

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(
        response.is_ok(),
        "Expected success but got: {:?}",
        response.err()
    );
}

#[tokio::test]
async fn test_exchange_token_cache_miss_then_set() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let user_repo = builder.user_repo.clone();
    let service_repo = builder.service_repo.clone();
    let rbac_repo = builder.rbac_repo.clone();

    user_repo.add_user(create_test_user(user_id)).await;
    service_repo
        .add_service(create_test_service(service_id, tenant_id))
        .await;
    service_repo
        .add_client(create_test_client(client_uuid, service_id, "test-client"))
        .await;
    rbac_repo
        .set_user_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["admin".to_string()],
                vec!["read".to_string()],
            ),
        )
        .await;

    let mock_cache = MockCacheManager::new();
    let service = builder.build_with_mock_cache(mock_cache);

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_ok());

    // Verify cache was checked (get) and then set
    // Note: We can't easily access the cache after build_with_mock_cache
    // because ownership is moved. This test verifies the flow works.
}

#[tokio::test]
async fn test_exchange_token_cache_hit() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let user_repo = builder.user_repo.clone();
    let service_repo = builder.service_repo.clone();

    user_repo.add_user(create_test_user(user_id)).await;
    service_repo
        .add_service(create_test_service(service_id, tenant_id))
        .await;
    service_repo
        .add_client(create_test_client(client_uuid, service_id, "test-client"))
        .await;

    // Pre-populate cache - no need to set up rbac_repo data
    let mock_cache = MockCacheManager::new();
    mock_cache
        .seed_roles_for_service(
            user_id,
            tenant_id,
            service_id,
            create_user_roles(
                user_id,
                tenant_id,
                vec!["cached-admin".to_string()],
                vec!["cached:read".to_string()],
            ),
        )
        .await;

    let jwt_manager = builder.jwt_manager.clone();
    let service = builder.build_with_mock_cache(mock_cache);

    let request = Request::new(ExchangeTokenRequest {
        identity_token,
        tenant_id: tenant_id.to_string(),
        service_id: "test-client".to_string(),
    });

    let response = service.exchange_token(request).await;
    assert!(response.is_ok());

    // Verify the token contains cached roles
    let response = response.unwrap().into_inner();
    let claims = jwt_manager
        .verify_tenant_access_token_with_optional_audience(
            &response.access_token,
            Some("test-client"),
        )
        .unwrap();
    assert_eq!(claims.roles, vec!["cached-admin"]);
    assert_eq!(claims.permissions, vec!["cached:read"]);
}
