# Auth9 Policy Authorization Matrix

## Goal
Centralize authorization decisions in `auth9-core/src/policy/mod.rs` and keep HTTP handlers focused on input/output and business flow.

## Policy Entry Points
- `enforce(config, auth, input)`: stateless checks based on token claims/config.
- `enforce_with_state(state, auth, input)`: state-aware checks with DB fallback (platform admin, owner checks, shared-tenant checks).
- `resolve_tenant_list_mode_with_state(state, auth)`: resolves tenant list visibility mode for tenant listing API.

## Action Matrix
- `PlatformAdmin`: requires platform admin.
- `AuditRead`, `SessionForceLogout`, `SecurityAlertRead`, `SecurityAlertResolve`, `UserWrite`: platform-level privileged actions.
- `WebhookRead`, `WebhookWrite`: tenant-scoped webhook permissions.
- `TenantServiceRead`, `TenantServiceWrite`: tenant service management permissions.
- `SystemConfigRead`, `SystemConfigWrite`: system config access by tenant scope and role.
- `ActionRead`, `ActionWrite`: integration action permissions.
- `TenantRead`, `TenantWrite`: tenant-scoped resource access.
- `TenantSsoRead`, `TenantSsoWrite`: tenant enterprise SSO connector access.
- `ServiceRead`, `ServiceWrite`, `ServiceList`: authorization service API access.
- `RbacRead`, `RbacWrite`, `RbacAssignSelf`: RBAC read/write and self-assignment guard.
- `InvitationRead`, `InvitationWrite`: invitation management authorization.
- `UserManage`, `UserTenantRead`, `UserReadOther`: user management and profile visibility checks.
- `TenantOwner`, `TenantActualOwner`: owner-level tenant operations (with/without platform admin bypass).

## Handler Rule
For new HTTP endpoints:
1. Define/choose a `PolicyAction` and `ResourceScope`.
2. Call `enforce` or `enforce_with_state` before business operation.
3. Keep direct `TokenType` branching out of handlers unless it is non-auth business logic.

## Current Status
- Core tenant/user/invitation/service/role/sso auth flows are policy-driven.
- Remaining `Forbidden` responses in handlers are business constraints (for example: password confirmation failures, public registration disabled), not token-branch authorization logic.
