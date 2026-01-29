//! Token Exchange gRPC service implementation

use crate::cache::CacheManager;
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

pub struct TokenExchangeService<U, S, R>
where
    U: UserRepository,
    S: ServiceRepository,
    R: RbacRepository,
{
    jwt_manager: JwtManager,
    cache_manager: CacheManager,
    user_repo: Arc<U>,
    service_repo: Arc<S>,
    rbac_repo: Arc<R>,
}

impl<U, S, R> TokenExchangeService<U, S, R>
where
    U: UserRepository,
    S: ServiceRepository,
    R: RbacRepository,
{
    pub fn new(
        jwt_manager: JwtManager,
        cache_manager: CacheManager,
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
impl<U, S, R> TokenExchange for TokenExchangeService<U, S, R>
where
    U: UserRepository + 'static,
    S: ServiceRepository + 'static,
    R: RbacRepository + 'static,
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

        // Verify service exists
        let service = self
            .service_repo
            .find_by_client_id(&req.service_id)
            .await
            .map_err(|e| Status::internal(format!("Failed to lookup service: {}", e)))?
            .ok_or_else(|| Status::not_found("Service not found"))?;

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
                &service.client_id,
                user_roles.roles,
                user_roles.permissions,
            )
            .map_err(|e| Status::internal(format!("Failed to create access token: {}", e)))?;

        let refresh_token = self
            .jwt_manager
            .create_refresh_token(Uuid::from(user_id), Uuid::from(tenant_id), &service.client_id)
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
