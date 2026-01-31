//! validate_token gRPC method tests

use super::*;
use auth9_core::grpc::proto::token_exchange_server::TokenExchange;
use auth9_core::grpc::proto::ValidateTokenRequest;
use tonic::Request;
use uuid::Uuid;

#[tokio::test]
async fn test_validate_token_success() {
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

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "test-client".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.valid);
    assert_eq!(response.user_id, user_id.to_string());
    assert_eq!(response.tenant_id, tenant_id.to_string());
    assert!(response.error.is_empty());
}

#[tokio::test]
async fn test_validate_token_invalid_token() {
    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(ValidateTokenRequest {
        access_token: "invalid-token".to_string(),
        audience: "test-client".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.valid);
    assert!(!response.error.is_empty());
}

#[tokio::test]
async fn test_validate_token_wrong_audience() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let access_token = builder
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "service-a",
            vec![],
            vec![],
        )
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "service-b".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.valid);
}

#[tokio::test]
async fn test_validate_token_empty_audience() {
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

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: String::new(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(response.valid);
}

#[tokio::test]
async fn test_validate_token_empty_token() {
    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(ValidateTokenRequest {
        access_token: "".to_string(),
        audience: "test-client".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    assert!(!response.valid);
    assert!(!response.error.is_empty());
}

#[tokio::test]
async fn test_validate_token_with_special_characters_in_audience() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let access_token = builder
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "service-with-special-chars_123",
            vec![],
            vec![],
        )
        .unwrap();

    let service = builder.build_with_noop_cache();

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "service-with-special-chars_123".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().valid);
}

#[tokio::test]
async fn test_validate_token_case_sensitive_audience() {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let builder = GrpcTestBuilder::new();
    let access_token = builder
        .jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "MyService",
            vec![],
            vec![],
        )
        .unwrap();

    let service = builder.build_with_noop_cache();

    // Try with different case
    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "myservice".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    // Should be invalid due to case mismatch
    assert!(!response.valid);
}

#[tokio::test]
async fn test_validate_token_with_different_issuer() {
    // Create a token with different issuer configuration
    let mut different_config = test_jwt_config();
    different_config.issuer = "https://different-issuer.test".to_string();

    let different_jwt_manager = JwtManager::new(different_config);
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let access_token = different_jwt_manager
        .create_tenant_access_token(
            user_id,
            "test@example.com",
            tenant_id,
            "test-client",
            vec![],
            vec![],
        )
        .unwrap();

    // Use default jwt_manager (different issuer)
    let service = GrpcTestBuilder::new().build_with_noop_cache();

    let request = Request::new(ValidateTokenRequest {
        access_token,
        audience: "test-client".to_string(),
    });

    let response = service.validate_token(request).await;
    assert!(response.is_ok());

    let response = response.unwrap().into_inner();
    // Token should be invalid due to issuer mismatch
    assert!(!response.valid);
}
