//! Token Exchange gRPC service implementation

use crate::domain::StringUuid;
use crate::grpc::proto::{
    token_exchange_server::TokenExchange, ExchangeTokenRequest, ExchangeTokenResponse,
    GetUserRolesRequest, GetUserRolesResponse, IntrospectTokenRequest, IntrospectTokenResponse,
    Role as ProtoRole, ValidateTokenRequest, ValidateTokenResponse,
};
use crate::jwt::JwtManager;
use crate::repository::{RbacRepository, ServiceRepository, UserRepository};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// Trait for cache operations needed by TokenExchangeService
pub trait TokenExchangeCache: Send + Sync {
    fn get_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
    ) -> impl std::future::Future<Output = crate::error::Result<Option<crate::domain::UserRolesInTenant>>> + Send;

    fn set_user_roles_for_service(
        &self,
        roles: &crate::domain::UserRolesInTenant,
        service_id: Uuid,
    ) -> impl std::future::Future<Output = crate::error::Result<()>> + Send;

    fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> impl std::future::Future<Output = crate::error::Result<Option<crate::domain::UserRolesInTenant>>> + Send;

    fn set_user_roles(
        &self,
        roles: &crate::domain::UserRolesInTenant,
    ) -> impl std::future::Future<Output = crate::error::Result<()>> + Send;
}

impl TokenExchangeCache for crate::cache::CacheManager {
    async fn get_user_roles_for_service(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        service_id: Uuid,
    ) -> crate::error::Result<Option<crate::domain::UserRolesInTenant>> {
        crate::cache::CacheManager::get_user_roles_for_service(self, user_id, tenant_id, service_id).await
    }

    async fn set_user_roles_for_service(
        &self,
        roles: &crate::domain::UserRolesInTenant,
        service_id: Uuid,
    ) -> crate::error::Result<()> {
        crate::cache::CacheManager::set_user_roles_for_service(self, roles, service_id).await
    }

    async fn get_user_roles(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> crate::error::Result<Option<crate::domain::UserRolesInTenant>> {
        crate::cache::CacheManager::get_user_roles(self, user_id, tenant_id).await
    }

    async fn set_user_roles(
        &self,
        roles: &crate::domain::UserRolesInTenant,
    ) -> crate::error::Result<()> {
        crate::cache::CacheManager::set_user_roles(self, roles).await
    }
}

impl TokenExchangeCache for crate::cache::NoOpCacheManager {
    async fn get_user_roles_for_service(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
        _service_id: Uuid,
    ) -> crate::error::Result<Option<crate::domain::UserRolesInTenant>> {
        Ok(None)
    }

    async fn set_user_roles_for_service(
        &self,
        _roles: &crate::domain::UserRolesInTenant,
        _service_id: Uuid,
    ) -> crate::error::Result<()> {
        Ok(())
    }

    async fn get_user_roles(
        &self,
        _user_id: Uuid,
        _tenant_id: Uuid,
    ) -> crate::error::Result<Option<crate::domain::UserRolesInTenant>> {
        Ok(None)
    }

    async fn set_user_roles(
        &self,
        _roles: &crate::domain::UserRolesInTenant,
    ) -> crate::error::Result<()> {
        Ok(())
    }
}

pub struct TokenExchangeService<U, S, R, C>
where
    U: UserRepository,
    S: ServiceRepository,
    R: RbacRepository,
    C: TokenExchangeCache,
{
    jwt_manager: JwtManager,
    cache_manager: C,
    user_repo: Arc<U>,
    service_repo: Arc<S>,
    rbac_repo: Arc<R>,
}

impl<U, S, R, C> TokenExchangeService<U, S, R, C>
where
    U: UserRepository,
    S: ServiceRepository,
    R: RbacRepository,
    C: TokenExchangeCache,
{
    pub fn new(
        jwt_manager: JwtManager,
        cache_manager: C,
        user_repo: Arc<U>,
        service_repo: Arc<S>,
        rbac_repo: Arc<R>,
    ) -> Self {
        Self {
            jwt_manager,
            cache_manager,
            user_repo,
            service_repo,
            rbac_repo,
        }
    }
}

#[tonic::async_trait]
impl<U, S, R, C> TokenExchange for TokenExchangeService<U, S, R, C>
where
    U: UserRepository + 'static,
    S: ServiceRepository + 'static,
    R: RbacRepository + 'static,
    C: TokenExchangeCache + 'static,
{
    async fn exchange_token(
        &self,
        request: Request<ExchangeTokenRequest>,
    ) -> Result<Response<ExchangeTokenResponse>, Status> {
        let req = request.into_inner();

        // Verify identity token
        let claims = self
            .jwt_manager
            .verify_identity_token(&req.identity_token)
            .map_err(|e| Status::unauthenticated(format!("Invalid identity token: {}", e)))?;

        let user_id = claims.sub
            .parse::<StringUuid>()
            .map_err(|_| Status::internal("Invalid user ID in token"))?;
        let tenant_id = req.tenant_id
            .parse::<StringUuid>()
            .map_err(|_| Status::invalid_argument("Invalid tenant ID"))?;

        let user_exists = self
            .user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to lookup user: {}", e)))?
            .is_some();
        if !user_exists {
            return Err(Status::not_found("User not found"));
        }

        // Verify client exists
        let client = self
            .service_repo
            .find_client_by_client_id(&req.service_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to lookup client: {}", e)))?
            .ok_or_else(|| Status::not_found("Client not found"))?;
            
        // Get Service
        let service = self
            .service_repo
            .find_by_id(client.service_id.0)
            .await
            .map_err(|e| Status::internal(format!("Failed to lookup service: {}", e)))?
            .ok_or_else(|| Status::internal("Service integrity error"))?;

        let user_roles = match self
            .cache_manager
            .get_user_roles_for_service(Uuid::from(user_id), Uuid::from(tenant_id), service.id.0)
            .await
        {
            Ok(Some(roles)) => roles,
            _ => {
                let roles = self
                    .rbac_repo
                    .find_user_roles_in_tenant_for_service(user_id, tenant_id, service.id)
                    .await
                    .map_err(|e| Status::internal(format!("Failed to get user roles: {}", e)))?;

                let _ = self
                    .cache_manager
                    .set_user_roles_for_service(&roles, service.id.0)
                    .await;
                roles
            }
        };

        // Create tenant access token
        let access_token = self
            .jwt_manager
            .create_tenant_access_token(
                Uuid::from(user_id),
                &claims.email,
                Uuid::from(tenant_id),
                &client.client_id,
                user_roles.roles,
                user_roles.permissions,
            )
            .map_err(|e| Status::internal(format!("Failed to create access token: {}", e)))?;

        let refresh_token = self
            .jwt_manager
            .create_refresh_token(Uuid::from(user_id), Uuid::from(tenant_id), &client.client_id)
            .map_err(|e| Status::internal(format!("Failed to create refresh token: {}", e)))?;

        Ok(Response::new(ExchangeTokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_manager.access_token_ttl(),
            refresh_token,
        }))
    }

    async fn validate_token(
        &self,
        request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        let req = request.into_inner();

        let audience = if req.audience.is_empty() {
            None
        } else {
            Some(req.audience.as_str())
        };

        match self
            .jwt_manager
            .verify_tenant_access_token(&req.access_token, audience)
        {
            Ok(claims) => Ok(Response::new(ValidateTokenResponse {
                valid: true,
                user_id: claims.sub,
                tenant_id: claims.tenant_id,
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(ValidateTokenResponse {
                valid: false,
                user_id: String::new(),
                tenant_id: String::new(),
                error: e.to_string(),
            })),
        }
    }

    async fn get_user_roles(
        &self,
        request: Request<GetUserRolesRequest>,
    ) -> Result<Response<GetUserRolesResponse>, Status> {
        let req = request.into_inner();

        let user_id = req.user_id
            .parse::<StringUuid>()
            .map_err(|_| Status::invalid_argument("Invalid user ID"))?;
        let tenant_id = req.tenant_id
            .parse::<StringUuid>()
            .map_err(|_| Status::invalid_argument("Invalid tenant ID"))?;

        let (user_roles, role_records) = if req.service_id.is_empty() {
            let user_roles = match self.cache_manager.get_user_roles(Uuid::from(user_id), Uuid::from(tenant_id)).await {
                Ok(Some(roles)) => roles,
                _ => {
                    let roles = self
                        .rbac_repo
                        .find_user_roles_in_tenant(user_id, tenant_id)
                        .await
                        .map_err(|e| {
                            Status::internal(format!("Failed to get user roles: {}", e))
                        })?;

                    let _ = self.cache_manager.set_user_roles(&roles).await;
                    roles
                }
            };

            let role_records = self
                .rbac_repo
                .find_user_role_records_in_tenant(user_id, tenant_id, None)
                .await
                .map_err(|e| Status::internal(format!("Failed to get role records: {}", e)))?;
            (user_roles, role_records)
        } else {
            let service_id = req.service_id
                .parse::<StringUuid>()
                .map_err(|_| Status::invalid_argument("Invalid service ID"))?;

            let user_roles = match self
                .cache_manager
                .get_user_roles_for_service(Uuid::from(user_id), Uuid::from(tenant_id), service_id.0)
                .await
            {
                Ok(Some(roles)) => roles,
                _ => {
                    let roles = self
                        .rbac_repo
                        .find_user_roles_in_tenant_for_service(user_id, tenant_id, service_id)
                        .await
                        .map_err(|e| {
                            Status::internal(format!("Failed to get user roles: {}", e))
                        })?;

                    let _ = self
                        .cache_manager
                        .set_user_roles_for_service(&roles, service_id.0)
                        .await;
                    roles
                }
            };

            let role_records = self
                .rbac_repo
                .find_user_role_records_in_tenant(user_id, tenant_id, Some(service_id))
                .await
                .map_err(|e| Status::internal(format!("Failed to get role records: {}", e)))?;

            (user_roles, role_records)
        };

        let roles = role_records
            .into_iter()
            .map(|role| ProtoRole {
                id: role.id.to_string(),
                name: role.name,
                service_id: role.service_id.to_string(),
            })
            .collect();

        Ok(Response::new(GetUserRolesResponse {
            roles,
            permissions: user_roles.permissions,
        }))
    }

    async fn introspect_token(
        &self,
        request: Request<IntrospectTokenRequest>,
    ) -> Result<Response<IntrospectTokenResponse>, Status> {
        let req = request.into_inner();

        // Try as tenant access token first
        match self
            .jwt_manager
            .verify_tenant_access_token(&req.token, None)
        {
            Ok(claims) => Ok(Response::new(IntrospectTokenResponse {
                active: true,
                sub: claims.sub,
                email: claims.email,
                tenant_id: claims.tenant_id,
                roles: claims.roles,
                permissions: claims.permissions,
                exp: claims.exp,
                iat: claims.iat,
                iss: claims.iss,
                aud: claims.aud,
            })),
            Err(_) => {
                // Try as identity token
                match self.jwt_manager.verify_identity_token(&req.token) {
                    Ok(claims) => Ok(Response::new(IntrospectTokenResponse {
                        active: true,
                        sub: claims.sub,
                        email: claims.email,
                        tenant_id: String::new(),
                        roles: vec![],
                        permissions: vec![],
                        exp: claims.exp,
                        iat: claims.iat,
                        iss: claims.iss,
                        aud: claims.aud,
                    })),
                    Err(_) => Ok(Response::new(IntrospectTokenResponse {
                        active: false,
                        sub: String::new(),
                        email: String::new(),
                        tenant_id: String::new(),
                        roles: vec![],
                        permissions: vec![],
                        exp: 0,
                        iat: 0,
                        iss: String::new(),
                        aud: String::new(),
                    })),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_token_request_structure() {
        let request = ExchangeTokenRequest {
            identity_token: "test-token".to_string(),
            tenant_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            service_id: "my-service".to_string(),
        };
        
        assert_eq!(request.identity_token, "test-token");
        assert!(!request.tenant_id.is_empty());
        assert_eq!(request.service_id, "my-service");
    }

    #[test]
    fn test_exchange_token_response_structure() {
        let response = ExchangeTokenResponse {
            access_token: "access-token-xyz".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: "refresh-token-abc".to_string(),
        };
        
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, 3600);
        assert!(!response.access_token.is_empty());
    }

    #[test]
    fn test_validate_token_request_structure() {
        let request = ValidateTokenRequest {
            access_token: "test-access-token".to_string(),
            audience: "my-service".to_string(),
        };
        
        assert!(!request.access_token.is_empty());
        assert_eq!(request.audience, "my-service");
    }

    #[test]
    fn test_validate_token_request_empty_audience() {
        let request = ValidateTokenRequest {
            access_token: "test-token".to_string(),
            audience: String::new(),
        };
        
        assert!(request.audience.is_empty());
    }

    #[test]
    fn test_validate_token_response_valid() {
        let response = ValidateTokenResponse {
            valid: true,
            user_id: "user-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            error: String::new(),
        };
        
        assert!(response.valid);
        assert!(response.error.is_empty());
    }

    #[test]
    fn test_validate_token_response_invalid() {
        let response = ValidateTokenResponse {
            valid: false,
            user_id: String::new(),
            tenant_id: String::new(),
            error: "Token expired".to_string(),
        };
        
        assert!(!response.valid);
        assert!(!response.error.is_empty());
    }

    #[test]
    fn test_get_user_roles_request_with_service() {
        let request = GetUserRolesRequest {
            user_id: "user-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            service_id: "service-789".to_string(),
        };
        
        assert!(!request.service_id.is_empty());
    }

    #[test]
    fn test_get_user_roles_request_without_service() {
        let request = GetUserRolesRequest {
            user_id: "user-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            service_id: String::new(),
        };
        
        assert!(request.service_id.is_empty());
    }

    #[test]
    fn test_get_user_roles_response_structure() {
        let response = GetUserRolesResponse {
            roles: vec![
                ProtoRole {
                    id: "role-1".to_string(),
                    name: "admin".to_string(),
                    service_id: "service-1".to_string(),
                },
                ProtoRole {
                    id: "role-2".to_string(),
                    name: "viewer".to_string(),
                    service_id: "service-1".to_string(),
                },
            ],
            permissions: vec!["read".to_string(), "write".to_string()],
        };
        
        assert_eq!(response.roles.len(), 2);
        assert_eq!(response.permissions.len(), 2);
    }

    #[test]
    fn test_introspect_token_request_structure() {
        let request = IntrospectTokenRequest {
            token: "some-jwt-token".to_string(),
        };
        
        assert!(!request.token.is_empty());
    }

    #[test]
    fn test_introspect_token_response_active() {
        let response = IntrospectTokenResponse {
            active: true,
            sub: "user-123".to_string(),
            email: "user@example.com".to_string(),
            tenant_id: "tenant-456".to_string(),
            roles: vec!["admin".to_string()],
            permissions: vec!["read".to_string(), "write".to_string()],
            exp: 1700000000,
            iat: 1699996400,
            iss: "https://auth9.example.com".to_string(),
            aud: "my-service".to_string(),
        };
        
        assert!(response.active);
        assert_eq!(response.sub, "user-123");
        assert!(!response.roles.is_empty());
    }

    #[test]
    fn test_introspect_token_response_inactive() {
        let response = IntrospectTokenResponse {
            active: false,
            sub: String::new(),
            email: String::new(),
            tenant_id: String::new(),
            roles: vec![],
            permissions: vec![],
            exp: 0,
            iat: 0,
            iss: String::new(),
            aud: String::new(),
        };
        
        assert!(!response.active);
        assert!(response.sub.is_empty());
        assert!(response.roles.is_empty());
    }

    #[test]
    fn test_proto_role_structure() {
        let role = ProtoRole {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            name: "administrator".to_string(),
            service_id: "550e8400-e29b-41d4-a716-446655440001".to_string(),
        };
        
        assert_eq!(role.name, "administrator");
        assert!(!role.id.is_empty());
        assert!(!role.service_id.is_empty());
    }
}
