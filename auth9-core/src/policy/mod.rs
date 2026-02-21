//! Centralized authorization policy engine for HTTP handlers.

pub(crate) mod abac;

use crate::config::Config;
use crate::domain::StringUuid;
use crate::error::AppError;
use crate::middleware::auth::{AuthUser, TokenType};
use crate::state::HasServices;

pub type PolicyResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantListMode {
    AllTenants,
    UserMemberships,
    TokenTenant(StringUuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    PlatformAdmin,
    AuditRead,
    SessionForceLogout,
    WebhookRead,
    WebhookWrite,
    TenantServiceRead,
    TenantServiceWrite,
    SecurityAlertRead,
    SecurityAlertResolve,
    SystemConfigRead,
    SystemConfigWrite,
    ActionRead,
    ActionWrite,
    UserWrite,
    TenantRead,
    TenantWrite,
    TenantSsoRead,
    TenantSsoWrite,
    ServiceRead,
    ServiceWrite,
    ServiceList,
    RbacRead,
    RbacWrite,
    RbacAssignSelf,
    InvitationRead,
    InvitationWrite,
    UserManage,
    UserTenantRead,
    UserReadOther,
    TenantOwner,
    TenantActualOwner,
    AbacRead,
    AbacWrite,
    AbacPublish,
    AbacSimulate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceScope {
    Global,
    Tenant(StringUuid),
    User(StringUuid),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyInput {
    pub action: PolicyAction,
    pub scope: ResourceScope,
}

pub fn enforce(config: &Config, auth: &AuthUser, input: &PolicyInput) -> PolicyResult<()> {
    // Platform admin bypass: applies to both Identity and TenantAccess tokens
    if auth.token_type != TokenType::ServiceClient && config.is_platform_admin_email(&auth.email) {
        return Ok(());
    }

    match input.action {
        PolicyAction::PlatformAdmin => require_platform_admin(config, auth),
        PolicyAction::AuditRead
        | PolicyAction::SessionForceLogout
        | PolicyAction::SecurityAlertRead
        | PolicyAction::SecurityAlertResolve
        | PolicyAction::UserWrite => require_platform_admin(config, auth),
        PolicyAction::WebhookRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["webhook:read", "webhook:*"])
        }
        PolicyAction::WebhookWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["webhook:write", "webhook:*"])
        }
        PolicyAction::TenantServiceRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &[
                    "tenant_service:read",
                    "tenant_service:write",
                    "tenant_service:*",
                ],
            )
        }
        PolicyAction::TenantServiceWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &["tenant_service:write", "tenant_service:*"],
            )
        }
        PolicyAction::SystemConfigRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_system_config_read(config, auth, tenant_id)
        }
        PolicyAction::SystemConfigWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_system_config_write(config, auth, tenant_id)
        }
        PolicyAction::ActionRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["action:read", "action:*"])
        }
        PolicyAction::ActionWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["action:write", "action:*"])
        }
        PolicyAction::TenantRead | PolicyAction::TenantSsoRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_scope_match(auth, tenant_id, true)
        }
        PolicyAction::TenantWrite | PolicyAction::TenantSsoWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_scope_match(auth, tenant_id, true)
        }
        PolicyAction::ServiceRead | PolicyAction::ServiceWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &["service:write", "service:*", "tenant_service:write"],
            )
        }
        PolicyAction::ServiceList => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_scope_match(auth, tenant_id, true)
        }
        PolicyAction::RbacRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_scope_match(auth, tenant_id, true)
        }
        PolicyAction::RbacWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &["rbac:write", "rbac:*", "role:write"],
            )
        }
        PolicyAction::InvitationRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_scope_match(auth, tenant_id, false)
        }
        PolicyAction::InvitationWrite => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["user:write", "user:*"])
        }
        PolicyAction::AbacRead => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &["abac:read", "abac:write", "abac:*", "rbac:write", "rbac:*"],
            )
        }
        PolicyAction::AbacWrite | PolicyAction::AbacPublish => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &["abac:write", "abac:*", "rbac:write", "rbac:*"],
            )
        }
        PolicyAction::AbacSimulate => {
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(
                auth,
                tenant_id,
                &["abac:read", "abac:write", "abac:*", "rbac:write", "rbac:*"],
            )
        }
        PolicyAction::UserManage => match auth.token_type {
            TokenType::Identity => Err(AppError::Forbidden(
                "Platform admin required for identity-token user management".to_string(),
            )),
            TokenType::TenantAccess => {
                let has_admin_role = auth.roles.iter().any(|r| r == "admin" || r == "owner");
                let has_permission = auth.permissions.iter().any(|p| {
                    p == "user:write"
                        || p == "user:delete"
                        || p == "user:*"
                        || p == "rbac:write"
                        || p == "rbac:*"
                });
                if has_admin_role || has_permission {
                    Ok(())
                } else {
                    Err(AppError::Forbidden(
                        "Admin access required to manage users".to_string(),
                    ))
                }
            }
            TokenType::ServiceClient => Err(AppError::Forbidden(
                "Service client tokens cannot manage users".to_string(),
            )),
        },
        PolicyAction::RbacAssignSelf
        | PolicyAction::UserTenantRead
        | PolicyAction::UserReadOther => Err(AppError::Internal(anyhow::anyhow!(
            "User policy action requires state-aware enforcement"
        ))),
        PolicyAction::TenantOwner | PolicyAction::TenantActualOwner => Err(AppError::Internal(
            anyhow::anyhow!("Tenant owner policy action requires state-aware enforcement"),
        )),
    }
}

pub async fn is_platform_admin_with_db<S: HasServices>(state: &S, auth: &AuthUser) -> bool {
    if auth.token_type == TokenType::ServiceClient {
        return false;
    }
    if state.config().is_platform_admin_email(&auth.email) {
        return true;
    }
    if let Ok(user_tenants) = state
        .user_service()
        .get_user_tenants_with_tenant(StringUuid::from(auth.user_id))
        .await
    {
        return user_tenants
            .iter()
            .any(|tu| tu.tenant.slug == "auth9-platform" && tu.role_in_tenant == "admin");
    }
    false
}

pub async fn require_platform_admin_with_db<S: HasServices>(
    state: &S,
    auth: &AuthUser,
) -> PolicyResult<()> {
    if is_platform_admin_with_db(state, auth).await {
        Ok(())
    } else {
        Err(AppError::Forbidden("Platform admin required".to_string()))
    }
}

pub async fn enforce_with_state<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    input: &PolicyInput,
) -> PolicyResult<()> {
    if input.action == PolicyAction::UserTenantRead {
        let tenant_id = require_tenant_scope(&input.scope)?;
        return require_user_tenant_read_with_state(state, auth, tenant_id).await;
    }
    if input.action == PolicyAction::RbacAssignSelf {
        if is_platform_admin_with_db(state, auth).await {
            return Ok(());
        }
        return Err(AppError::Forbidden(
            "Cannot assign roles to yourself".to_string(),
        ));
    }
    if input.action == PolicyAction::UserReadOther {
        let target_user_id = require_user_scope(&input.scope)?;
        return require_user_read_other_with_state(state, auth, target_user_id).await;
    }
    if matches!(
        input.action,
        PolicyAction::TenantOwner | PolicyAction::TenantActualOwner
    ) {
        let tenant_id = require_tenant_scope(&input.scope)?;
        return require_tenant_owner_with_state(
            state,
            auth,
            tenant_id,
            matches!(input.action, PolicyAction::TenantOwner),
        )
        .await;
    }

    if action_supports_db_platform_admin(input.action)
        && is_platform_admin_with_db(state, auth).await
    {
        return Ok(());
    }

    enforce(state.config(), auth, input)?;

    let should_check_abac = matches!(
        input.action,
        PolicyAction::UserTenantRead
            | PolicyAction::UserManage
            | PolicyAction::InvitationRead
            | PolicyAction::InvitationWrite
            | PolicyAction::RbacWrite
            | PolicyAction::RbacAssignSelf
    );

    if should_check_abac {
        let abac_outcome = abac::evaluate_with_state(state, auth, input).await?;
        if abac_outcome.denied {
            match abac_outcome.mode {
                abac::AbacDecisionMode::Disabled => {}
                abac::AbacDecisionMode::Shadow => {
                    tracing::warn!(
                        action = ?input.action,
                        mode = "shadow",
                        deny_rules = ?abac_outcome.matched_deny_rule_ids,
                        allow_rules = ?abac_outcome.matched_allow_rule_ids,
                        "ABAC shadow deny matched"
                    );
                }
                abac::AbacDecisionMode::Enforce => {
                    return Err(AppError::Forbidden(
                        "Access denied by ABAC policy".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

pub async fn resolve_tenant_list_mode_with_state<S: HasServices>(
    state: &S,
    auth: &AuthUser,
) -> PolicyResult<TenantListMode> {
    if is_platform_admin_with_db(state, auth).await {
        return Ok(TenantListMode::AllTenants);
    }
    match auth.token_type {
        TokenType::Identity => Ok(TenantListMode::UserMemberships),
        TokenType::TenantAccess | TokenType::ServiceClient => auth
            .tenant_id
            .map(|tenant_id| TenantListMode::TokenTenant(StringUuid::from(tenant_id)))
            .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string())),
    }
}

fn require_user_scope(scope: &ResourceScope) -> PolicyResult<StringUuid> {
    match scope {
        ResourceScope::User(user_id) => Ok(*user_id),
        _ => Err(AppError::Internal(anyhow::anyhow!(
            "User-scoped policy action requires ResourceScope::User"
        ))),
    }
}

async fn require_user_tenant_read_with_state<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    tenant_id: StringUuid,
) -> PolicyResult<()> {
    if is_platform_admin_with_db(state, auth).await {
        return Ok(());
    }
    match auth.token_type {
        TokenType::Identity => Err(AppError::Forbidden(
            "Platform admin required to list users across tenants".to_string(),
        )),
        TokenType::ServiceClient => Err(AppError::Forbidden(
            "Service client tokens cannot access user management endpoints".to_string(),
        )),
        TokenType::TenantAccess => {
            if auth.tenant_id == Some(*tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Access denied: you can only list users in your own tenant".to_string(),
                ))
            }
        }
    }
}

async fn require_user_read_other_with_state<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    target_user_id: StringUuid,
) -> PolicyResult<()> {
    if is_platform_admin_with_db(state, auth).await {
        return Ok(());
    }
    match auth.token_type {
        TokenType::ServiceClient => Err(AppError::Forbidden(
            "Service client tokens cannot access user profiles".to_string(),
        )),
        TokenType::TenantAccess => {
            let has_admin_permission = auth.roles.iter().any(|r| r == "admin" || r == "owner")
                || auth
                    .permissions
                    .iter()
                    .any(|p| p == "user:read" || p == "user:*");
            if has_admin_permission {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Access denied: you can only view your own profile".to_string(),
                ))
            }
        }
        TokenType::Identity => {
            let auth_user_id = StringUuid::from(auth.user_id);
            let auth_user_tenants = state.user_service().get_user_tenants(auth_user_id).await?;
            let target_user_tenants = state
                .user_service()
                .get_user_tenants(target_user_id)
                .await?;

            let auth_user_admin_tenant_ids: std::collections::HashSet<_> = auth_user_tenants
                .iter()
                .filter(|tu| tu.role_in_tenant == "owner" || tu.role_in_tenant == "admin")
                .map(|tu| tu.tenant_id)
                .collect();
            let target_user_tenant_ids: std::collections::HashSet<_> =
                target_user_tenants.iter().map(|tu| tu.tenant_id).collect();

            if auth_user_admin_tenant_ids
                .intersection(&target_user_tenant_ids)
                .next()
                .is_some()
            {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Access denied: you can only view your own profile".to_string(),
                ))
            }
        }
    }
}

async fn require_tenant_owner_with_state<S: HasServices>(
    state: &S,
    auth: &AuthUser,
    tenant_id: StringUuid,
    allow_platform_admin_bypass: bool,
) -> PolicyResult<()> {
    if auth.token_type == TokenType::ServiceClient {
        return Err(AppError::Forbidden(
            "Service client tokens cannot perform tenant owner operations".to_string(),
        ));
    }

    if allow_platform_admin_bypass && is_platform_admin_with_db(state, auth).await {
        return Ok(());
    }

    if auth.token_type == TokenType::TenantAccess {
        if auth.tenant_id == Some(*tenant_id) && auth.roles.iter().any(|r| r == "owner") {
            return Ok(());
        }
        return Err(AppError::Forbidden(
            "Owner access required: you must be an owner of this tenant".to_string(),
        ));
    }

    let user_id = StringUuid::from(auth.user_id);
    let tenant_users = state.user_service().get_user_tenants(user_id).await?;
    if tenant_users
        .iter()
        .any(|tu| tu.tenant_id == tenant_id && tu.role_in_tenant == "owner")
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Only the current tenant owner can perform this operation".to_string(),
        ))
    }
}

fn require_tenant_scope(scope: &ResourceScope) -> PolicyResult<StringUuid> {
    match scope {
        ResourceScope::Tenant(tenant_id) => Ok(*tenant_id),
        _ => Err(AppError::Internal(anyhow::anyhow!(
            "Tenant-scoped policy action requires ResourceScope::Tenant"
        ))),
    }
}

fn require_platform_admin(config: &Config, auth: &AuthUser) -> PolicyResult<()> {
    // Platform admin check applies to both Identity and TenantAccess tokens
    if auth.token_type != TokenType::ServiceClient && config.is_platform_admin_email(&auth.email) {
        return Ok(());
    }
    Err(AppError::Forbidden("Platform admin required".to_string()))
}

fn require_tenant_admin_or_permission(
    auth: &AuthUser,
    tenant_id: StringUuid,
    permissions: &[&str],
) -> PolicyResult<()> {
    match auth.token_type {
        TokenType::TenantAccess => {
            let token_tenant_id = auth
                .tenant_id
                .ok_or_else(|| AppError::Forbidden("No tenant context in token".to_string()))?;

            if token_tenant_id != *tenant_id {
                return Err(AppError::Forbidden(
                    "Cannot access another tenant".to_string(),
                ));
            }

            let is_admin = auth.roles.iter().any(|r| r == "owner" || r == "admin");
            let has_permission = permissions
                .iter()
                .any(|permission| auth.permissions.iter().any(|p| p == permission));

            if is_admin || has_permission {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Admin or required permission is missing".to_string(),
                ))
            }
        }
        TokenType::Identity => Err(AppError::Forbidden(
            "Tenant-scoped token required".to_string(),
        )),
        TokenType::ServiceClient => Err(AppError::Forbidden(
            "Service client tokens are not allowed for this operation".to_string(),
        )),
    }
}

fn require_tenant_scope_match(
    auth: &AuthUser,
    tenant_id: StringUuid,
    allow_service_client: bool,
) -> PolicyResult<()> {
    match auth.token_type {
        TokenType::Identity => Err(AppError::Forbidden(
            "Tenant-scoped token required".to_string(),
        )),
        TokenType::TenantAccess => {
            if auth.tenant_id == Some(*tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Cannot access another tenant".to_string(),
                ))
            }
        }
        TokenType::ServiceClient => {
            if !allow_service_client {
                return Err(AppError::Forbidden(
                    "Service client tokens are not allowed for this operation".to_string(),
                ));
            }
            if auth.tenant_id == Some(*tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Cannot access another tenant".to_string(),
                ))
            }
        }
    }
}

fn action_supports_db_platform_admin(action: PolicyAction) -> bool {
    matches!(
        action,
        PolicyAction::PlatformAdmin
            | PolicyAction::AuditRead
            | PolicyAction::SessionForceLogout
            | PolicyAction::SecurityAlertRead
            | PolicyAction::SecurityAlertResolve
            | PolicyAction::UserWrite
            | PolicyAction::SystemConfigRead
            | PolicyAction::SystemConfigWrite
            | PolicyAction::TenantRead
            | PolicyAction::TenantWrite
            | PolicyAction::InvitationRead
            | PolicyAction::InvitationWrite
            | PolicyAction::UserManage
            | PolicyAction::UserTenantRead
            | PolicyAction::UserReadOther
            | PolicyAction::ServiceRead
            | PolicyAction::ServiceWrite
            | PolicyAction::ServiceList
            | PolicyAction::RbacRead
            | PolicyAction::RbacWrite
            | PolicyAction::RbacAssignSelf
            | PolicyAction::TenantSsoRead
            | PolicyAction::TenantSsoWrite
            | PolicyAction::TenantOwner
            | PolicyAction::AbacRead
            | PolicyAction::AbacWrite
            | PolicyAction::AbacPublish
            | PolicyAction::AbacSimulate
    )
}

fn require_system_config_read(
    config: &Config,
    auth: &AuthUser,
    tenant_id: StringUuid,
) -> PolicyResult<()> {
    match auth.token_type {
        TokenType::Identity => {
            if config.is_platform_admin_email(&auth.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Platform admin or tenant-scoped token required".to_string(),
                ))
            }
        }
        TokenType::TenantAccess | TokenType::ServiceClient => {
            if auth.tenant_id == Some(*tenant_id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Access denied: invalid tenant scope".to_string(),
                ))
            }
        }
    }
}

fn require_system_config_write(
    config: &Config,
    auth: &AuthUser,
    tenant_id: StringUuid,
) -> PolicyResult<()> {
    match auth.token_type {
        TokenType::Identity => {
            if config.is_platform_admin_email(&auth.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Platform admin or tenant admin required".to_string(),
                ))
            }
        }
        TokenType::TenantAccess => {
            if auth.tenant_id != Some(*tenant_id) {
                return Err(AppError::Forbidden(
                    "Access denied: invalid tenant scope".to_string(),
                ));
            }

            if auth.roles.iter().any(|r| r == "owner" || r == "admin") {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "Only tenant owner/admin can modify this resource".to_string(),
                ))
            }
        }
        TokenType::ServiceClient => Err(AppError::Forbidden(
            "Service client tokens cannot modify this resource".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CorsConfig, DatabaseConfig, GrpcSecurityConfig, JwtConfig, KeycloakConfig,
        PasswordResetConfig, RateLimitConfig, RedisConfig, SecurityHeadersConfig, ServerConfig,
        TelemetryConfig, WebAuthnConfig,
    };
    use crate::domain::action::AsyncActionConfig;

    fn create_test_config(platform_admins: Vec<String>) -> Config {
        Config {
            environment: "test".to_string(),
            http_host: "localhost".to_string(),
            http_port: 8080,
            grpc_host: "localhost".to_string(),
            grpc_port: 50051,
            database: DatabaseConfig {
                url: "mysql://test".to_string(),
                max_connections: 5,
                min_connections: 1,
                acquire_timeout_secs: 30,
                idle_timeout_secs: 600,
            },
            redis: RedisConfig {
                url: "redis://localhost".to_string(),
            },
            jwt: JwtConfig {
                secret: "test-secret".to_string(),
                issuer: "test".to_string(),
                access_token_ttl_secs: 3600,
                refresh_token_ttl_secs: 604800,
                private_key_pem: None,
                public_key_pem: None,
                previous_public_key_pem: None,
            },
            keycloak: KeycloakConfig {
                url: "http://localhost:8081".to_string(),
                public_url: "http://localhost:8081".to_string(),
                realm: "test".to_string(),
                admin_client_id: "admin-cli".to_string(),
                admin_client_secret: "secret".to_string(),
                ssl_required: "none".to_string(),
                core_public_url: None,
                portal_url: None,
                webhook_secret: None,
                event_source: "redis_stream".to_string(),
                event_stream_key: "auth9:keycloak:events".to_string(),
                event_stream_group: "auth9-core".to_string(),
                event_stream_consumer: "auth9-core-1".to_string(),
            },
            grpc_security: GrpcSecurityConfig::default(),
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig::default(),
            telemetry: TelemetryConfig::default(),
            platform_admin_emails: platform_admins,
            webauthn: WebAuthnConfig {
                rp_id: "localhost".to_string(),
                rp_name: "Test".to_string(),
                rp_origin: "http://localhost:3000".to_string(),
                challenge_ttl_secs: 300,
            },
            server: ServerConfig::default(),
            jwt_tenant_access_allowed_audiences: vec![],
            security_headers: SecurityHeadersConfig::default(),
            portal_client_id: None,
            async_action: AsyncActionConfig::default(),
            branding_allowed_domains: vec![],
            password_reset: PasswordResetConfig {
                hmac_key: "test-key".to_string(),
                token_ttl_secs: 3600,
            },
        }
    }

    fn create_platform_admin() -> AuthUser {
        AuthUser {
            user_id: uuid::Uuid::new_v4(),
            email: "admin@platform.com".to_string(),
            token_type: TokenType::Identity,
            tenant_id: None,
            roles: vec![],
            permissions: vec![],
        }
    }

    fn create_tenant_admin(tenant_id: StringUuid) -> AuthUser {
        AuthUser {
            user_id: uuid::Uuid::new_v4(),
            email: "admin@tenant.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(tenant_id.into()),
            roles: vec!["admin".to_string()],
            permissions: vec![],
        }
    }

    fn create_tenant_owner(tenant_id: StringUuid) -> AuthUser {
        AuthUser {
            user_id: uuid::Uuid::new_v4(),
            email: "owner@tenant.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(tenant_id.into()),
            roles: vec!["owner".to_string()],
            permissions: vec![],
        }
    }

    fn create_tenant_user(tenant_id: StringUuid, permissions: Vec<String>) -> AuthUser {
        AuthUser {
            user_id: uuid::Uuid::new_v4(),
            email: "user@tenant.com".to_string(),
            token_type: TokenType::TenantAccess,
            tenant_id: Some(tenant_id.into()),
            roles: vec!["member".to_string()],
            permissions,
        }
    }

    fn create_service_client(tenant_id: Option<StringUuid>) -> AuthUser {
        AuthUser {
            user_id: uuid::Uuid::new_v4(),
            email: "client@service.com".to_string(),
            token_type: TokenType::ServiceClient,
            tenant_id: tenant_id.map(|id| id.into()),
            roles: vec![],
            permissions: vec![],
        }
    }

    #[test]
    fn test_audit_read_requires_platform_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let input = PolicyInput {
            action: PolicyAction::AuditRead,
            scope: ResourceScope::Global,
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_audit_read_rejects_non_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::AuditRead,
            scope: ResourceScope::Global,
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[test]
    fn test_session_force_logout_requires_platform_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let input = PolicyInput {
            action: PolicyAction::SessionForceLogout,
            scope: ResourceScope::Global,
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_webhook_read_platform_admin_can_access() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let tenant_id = StringUuid::new_v4();
        let input = PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_webhook_read_tenant_admin_can_access() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_webhook_read_with_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["webhook:read".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_webhook_read_with_wildcard_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["webhook:*".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_webhook_read_rejects_wrong_tenant() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let wrong_tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["webhook:read".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(wrong_tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[test]
    fn test_webhook_read_rejects_without_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec![]);
        let input = PolicyInput {
            action: PolicyAction::WebhookRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_write_tenant_admin_can_access() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_webhook_write_with_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["webhook:write".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::WebhookWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_tenant_service_read_with_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["tenant_service:read".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::TenantServiceRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_tenant_service_write_with_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["tenant_service:write".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::TenantServiceWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_system_config_read_platform_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let tenant_id = StringUuid::new_v4();
        let input = PolicyInput {
            action: PolicyAction::SystemConfigRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_system_config_read_tenant_user() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec![]);
        let input = PolicyInput {
            action: PolicyAction::SystemConfigRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_system_config_read_service_client() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let client = create_service_client(Some(tenant_id));
        let input = PolicyInput {
            action: PolicyAction::SystemConfigRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &client, &input).is_ok());
    }

    #[test]
    fn test_system_config_read_rejects_wrong_tenant() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let wrong_tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec![]);
        let input = PolicyInput {
            action: PolicyAction::SystemConfigRead,
            scope: ResourceScope::Tenant(wrong_tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_system_config_read_rejects_non_admin_identity_token() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let mut user = create_platform_admin();
        user.email = "regular@user.com".to_string();
        let tenant_id = StringUuid::new_v4();
        let input = PolicyInput {
            action: PolicyAction::SystemConfigRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_system_config_write_platform_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let tenant_id = StringUuid::new_v4();
        let input = PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_system_config_write_tenant_admin() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_system_config_write_tenant_owner() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let owner = create_tenant_owner(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &owner, &input).is_ok());
    }

    #[test]
    fn test_system_config_write_rejects_regular_user() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec![]);
        let input = PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_system_config_write_rejects_service_client() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let client = create_service_client(Some(tenant_id));
        let input = PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        let result = enforce(&config, &client, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_system_config_write_rejects_wrong_tenant() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let wrong_tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::SystemConfigWrite,
            scope: ResourceScope::Tenant(wrong_tenant_id),
        };

        let result = enforce(&config, &admin, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_security_alert_read_requires_platform_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let input = PolicyInput {
            action: PolicyAction::SecurityAlertRead,
            scope: ResourceScope::Global,
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_security_alert_resolve_requires_platform_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let input = PolicyInput {
            action: PolicyAction::SecurityAlertResolve,
            scope: ResourceScope::Global,
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_require_tenant_scope_extracts_tenant_id() {
        let tenant_id = StringUuid::new_v4();
        let scope = ResourceScope::Tenant(tenant_id);
        let result = require_tenant_scope(&scope);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tenant_id);
    }

    #[test]
    fn test_require_tenant_scope_rejects_global() {
        let scope = ResourceScope::Global;
        let result = require_tenant_scope(&scope);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_tenant_scope_rejects_user() {
        let user_id = StringUuid::new_v4();
        let scope = ResourceScope::User(user_id);
        let result = require_tenant_scope(&scope);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_platform_admin_accepts_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        assert!(require_platform_admin(&config, &admin).is_ok());
    }

    #[test]
    fn test_require_platform_admin_rejects_non_admin() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let mut user = create_platform_admin();
        user.email = "user@example.com".to_string();
        let result = require_platform_admin(&config, &user);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_platform_admin_rejects_tenant_token() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_admin(tenant_id);
        let result = require_platform_admin(&config, &user);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_platform_admin_rejects_service_client() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let client = create_service_client(None);
        let result = require_platform_admin(&config, &client);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_accepts_admin() {
        let tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let result =
            require_tenant_admin_or_permission(&admin, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_accepts_owner() {
        let tenant_id = StringUuid::new_v4();
        let owner = create_tenant_owner(tenant_id);
        let result =
            require_tenant_admin_or_permission(&owner, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_accepts_with_permission() {
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["test:read".to_string()]);
        let result =
            require_tenant_admin_or_permission(&user, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_rejects_without_permission() {
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["other:read".to_string()]);
        let result =
            require_tenant_admin_or_permission(&user, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_rejects_wrong_tenant() {
        let tenant_id = StringUuid::new_v4();
        let wrong_tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let result = require_tenant_admin_or_permission(
            &admin,
            wrong_tenant_id,
            &["test:read", "test:write"],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_rejects_no_tenant_context() {
        let tenant_id = StringUuid::new_v4();
        let mut user = create_tenant_admin(tenant_id);
        user.tenant_id = None;
        let result =
            require_tenant_admin_or_permission(&user, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_rejects_identity_token() {
        let tenant_id = StringUuid::new_v4();
        let admin = create_platform_admin();
        let result =
            require_tenant_admin_or_permission(&admin, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_tenant_admin_or_permission_rejects_service_client() {
        let tenant_id = StringUuid::new_v4();
        let client = create_service_client(Some(tenant_id));
        let result =
            require_tenant_admin_or_permission(&client, tenant_id, &["test:read", "test:write"]);
        assert!(result.is_err());
    }

    // ============================================================
    // Action Permission Tests
    // ============================================================

    #[test]
    fn test_action_read_platform_admin_can_access() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let tenant_id = StringUuid::new_v4();
        let input = PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_action_read_tenant_admin_can_access() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_action_read_with_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["action:read".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_action_read_with_wildcard_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["action:*".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_action_read_rejects_wrong_tenant() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let wrong_tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["action:read".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(wrong_tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[test]
    fn test_action_read_rejects_without_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec![]);
        let input = PolicyInput {
            action: PolicyAction::ActionRead,
            scope: ResourceScope::Tenant(tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[test]
    fn test_action_write_platform_admin_can_access() {
        let config = create_test_config(vec!["admin@platform.com".to_string()]);
        let admin = create_platform_admin();
        let tenant_id = StringUuid::new_v4();
        let input = PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_action_write_tenant_admin_can_access() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let admin = create_tenant_admin(tenant_id);
        let input = PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &admin, &input).is_ok());
    }

    #[test]
    fn test_action_write_with_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["action:write".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_action_write_with_wildcard_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["action:*".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        assert!(enforce(&config, &user, &input).is_ok());
    }

    #[test]
    fn test_action_write_rejects_wrong_tenant() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let wrong_tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec!["action:write".to_string()]);
        let input = PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(wrong_tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }

    #[test]
    fn test_action_write_rejects_without_permission() {
        let config = create_test_config(vec![]);
        let tenant_id = StringUuid::new_v4();
        let user = create_tenant_user(tenant_id, vec![]);
        let input = PolicyInput {
            action: PolicyAction::ActionWrite,
            scope: ResourceScope::Tenant(tenant_id),
        };

        let result = enforce(&config, &user, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Forbidden(_)));
    }
}
