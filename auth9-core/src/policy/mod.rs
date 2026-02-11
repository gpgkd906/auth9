//! Centralized authorization policy engine for HTTP handlers.

use crate::config::Config;
use crate::domain::StringUuid;
use crate::error::AppError;
use crate::middleware::auth::{AuthUser, TokenType};

pub type PolicyResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
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
    match input.action {
        PolicyAction::AuditRead
        | PolicyAction::SessionForceLogout
        | PolicyAction::SecurityAlertRead
        | PolicyAction::SecurityAlertResolve => require_platform_admin(config, auth),
        PolicyAction::WebhookRead => {
            if auth.token_type == TokenType::Identity {
                return require_platform_admin(config, auth);
            }
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["webhook:read", "webhook:*"])
        }
        PolicyAction::WebhookWrite => {
            if auth.token_type == TokenType::Identity {
                return require_platform_admin(config, auth);
            }
            let tenant_id = require_tenant_scope(&input.scope)?;
            require_tenant_admin_or_permission(auth, tenant_id, &["webhook:write", "webhook:*"])
        }
        PolicyAction::TenantServiceRead => {
            if auth.token_type == TokenType::Identity {
                return require_platform_admin(config, auth);
            }
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
            if auth.token_type == TokenType::Identity {
                return require_platform_admin(config, auth);
            }
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
    match auth.token_type {
        TokenType::Identity => {
            if config.is_platform_admin_email(&auth.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden("Platform admin required".to_string()))
            }
        }
        TokenType::TenantAccess | TokenType::ServiceClient => Err(AppError::Forbidden(
            "Platform admin required".to_string(),
        )),
    }
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

fn require_system_config_read(config: &Config, auth: &AuthUser, tenant_id: StringUuid) -> PolicyResult<()> {
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
