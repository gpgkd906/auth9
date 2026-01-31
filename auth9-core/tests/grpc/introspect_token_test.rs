//! introspect_token gRPC method tests

use super::*;
use auth9_core::grpc::proto::token_exchange_server::TokenExchange;
use auth9_core::grpc::proto::IntrospectTokenRequest;
use tonic::Request;
use uuid::Uuid;

#[tokio::test]
async fn test_introspect_tenant_access_token() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let access_token = builder
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec!["admin".to_string()],
            vec!["user:read".to_string()],
        )
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    assert_eq!(response.sub, user_id.to_string());
    assert_eq!(response.email, "test@example.com");
    assert_eq!(response.tenant_id, tenant_id.to_string());
    assert_eq!(response.roles, vec!["admin"]);
    assert_eq!(response.permissions, vec!["user:read"]);
}

#[tokio::test]
async fn test_introspect_identity_token() {
    let user_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let identity_token = builder
        .jwt_manager
        .create_identity_token(user_id, "test@example.com", Some("Test User"))
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: identity_token,
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    assert_eq!(response.sub, user_id.to_string());
    assert_eq!(response.email, "test@example.com");
    assert!(response.tenant_id.is_empty());
    assert!(response.roles.is_empty());
}

#[tokio::test]
async fn test_introspect_invalid_token() {
    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: "invalid-token".to_string(),
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.active);
    assert!(response.sub.is_empty());
    assert!(response.email.is_empty());
}

#[tokio::test]
async fn test_introspect_expired_token() {
    // Create a JWT manager with very negative TTL to ensure token is expired
    let mut config = test_jwt_config();
    config.access_token_ttl_secs = -3600;

    let expired_jwt_manager = JwtManager::new(config);
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let access_token = expired_jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec![],
            vec![],
        )
        .unwrap();

    // Use fresh jwt_manager for verification (with normal TTL)
    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.active);
}

#[tokio::test]
async fn test_introspect_token_empty_token() {
    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: "".to_string(),
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.active);
}

#[tokio::test]
async fn test_introspect_token_with_all_fields() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let access_token = builder
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "full@example.com",
            tenant_id,
            "full-service",
            vec!["super-admin".to_string(), "manager".to_string()],
            vec![
                "all:read".to_string(),
                "all:write".to_string(),
                "all:delete".to_string(),
            ],
        )
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    assert_eq!(response.email, "full@example.com");
    assert_eq!(response.roles, vec!["super-admin", "manager"]);
    assert_eq!(response.permissions, vec!["all:read", "all:write", "all:delete"]);
    assert!(response.exp > 0);
    assert!(response.iat > 0);
    assert!(!response.iss.is_empty());
    assert_eq!(response.aud, "full-service");
}

#[tokio::test]
async fn test_introspect_refresh_token() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let refresh_token = builder
        .jwt_manager
        .create_refresh_token(user_id, tenant_id, "test-client")
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: refresh_token,
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    // Refresh tokens use RefreshClaims which lacks email/roles/permissions fields,
    // so they cannot be introspected as either TenantAccessClaims or IdentityClaims.
    assert!(!response.active);
}

#[tokio::test]
async fn test_introspect_token_timestamps() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let access_token = builder
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec![],
            vec![],
        )
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(IntrospectTokenRequest {
        token: access_token,
    });

    let response = service.introspect_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.active);
    // exp should be after iat
    assert!(response.exp > response.iat);
    // exp - iat should be approximately access_token_ttl_secs (3600)
    assert!((response.exp - response.iat) <= 3600);
    assert!((response.exp - response.iat) >= 3599);
}
