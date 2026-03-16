// Types - Claims
export type {
  IdentityClaims,
  TenantAccessClaims,
  ServiceClientClaims,
  Auth9Claims,
  TokenType,
} from "./types/claims.js";
export { getTokenType } from "./types/claims.js";

// Types - Responses
export type {
  DataResponse,
  PaginatedResponse,
  Pagination,
} from "./types/responses.js";

// Types - Domain
export type {
  Tenant,
  CreateTenantInput,
  UpdateTenantInput,
  MaliciousIpBlacklistEntry,
  UpdateMaliciousIpBlacklistInput,
  TenantUser,
} from "./types/tenant.js";
export type {
  User,
  CreateUserInput,
  UpdateUserInput,
  AddUserToTenantInput,
  UpdateUserRoleInput,
} from "./types/user.js";
export type {
  Role,
  Permission,
  CreateRoleInput,
  CreatePermissionInput,
  UpdateRoleInput,
  RoleWithPermissions,
  AssignRolesInput,
  UserRolesInTenant,
} from "./types/rbac.js";
export type {
  Service,
  CreateServiceInput,
  UpdateServiceInput,
  ServiceIntegration,
  Client,
  ClientWithSecret,
  CreateClientInput,
  ServiceWithStatus,
} from "./types/service.js";
export type { SessionInfo } from "./types/session.js";
export type {
  Invitation,
  InvitationStatus,
  CreateInvitationInput,
  InvitationValidation,
  AcceptInvitationInput,
} from "./types/invitation.js";
export type {
  Webhook,
  CreateWebhookInput,
  UpdateWebhookInput,
  WebhookTestResult,
} from "./types/webhook.js";
export type {
  IdentityProvider,
  CreateIdentityProviderInput,
  UpdateIdentityProviderInput,
  IdentityProviderTemplate,
  LinkedIdentity,
} from "./types/identity-provider.js";
export type {
  SSOConnector,
  CreateSSOConnectorInput,
  UpdateSSOConnectorInput,
  SSOTestResult,
} from "./types/sso.js";
export type {
  SamlApplication,
  CreateSamlApplicationInput,
  UpdateSamlApplicationInput,
  SamlCertificateInfo,
} from "./types/saml.js";
export type {
  AbacPolicy,
  AbacRule,
  CreateAbacPolicyInput,
  UpdateAbacPolicyInput,
  SimulateAbacInput,
  AbacSimulationResult,
} from "./types/abac.js";
export type {
  ScimToken,
  ScimTokenWithValue,
  CreateScimTokenInput,
  ScimLog,
  ScimLogQuery,
  ScimGroupMapping,
} from "./types/scim.js";
export type {
  TenantServiceInfo,
  ToggleTenantServiceInput,
} from "./types/tenant-service.js";
export type {
  LoginStats,
  LoginEvent,
  AuditLog,
  SecurityAlert,
} from "./types/analytics.js";
export type {
  Action,
  CreateActionInput,
  UpdateActionInput,
  ActionContext,
  ActionContextUser,
  ActionContextTenant,
  ActionContextRequest,
  TestActionResponse,
  ActionExecution,
  ActionStats,
  UpsertActionInput,
  BatchUpsertResponse,
  BatchError,
  LogQueryFilter,
} from "./types/action.js";
export { ActionTrigger } from "./types/action.js";

// HTTP Client
export { Auth9HttpClient } from "./http-client.js";
export type { HttpClientConfig } from "./http-client.js";
export { Auth9Client } from "./auth9-client.js";
export type { Auth9ClientConfig } from "./auth9-client.js";

// Sub-Clients
export { TenantsClient } from "./clients/tenants.js";
export { UsersClient } from "./clients/users.js";
export { ServicesClient } from "./clients/services.js";
export { RolesClient } from "./clients/roles.js";
export { PermissionsClient } from "./clients/permissions.js";
export { RbacClient } from "./clients/rbac.js";
export { InvitationsClient } from "./clients/invitations.js";
export { IdentityProvidersClient } from "./clients/identity-providers.js";
export { SsoClient } from "./clients/sso.js";
export { SamlClient } from "./clients/saml.js";
export { AbacClient } from "./clients/abac.js";
export { SessionsClient } from "./clients/sessions.js";
export { WebhooksClient } from "./clients/webhooks.js";
export { ScimClient } from "./clients/scim.js";
export { TenantServicesClient } from "./clients/tenant-services.js";

// Errors
export {
  Auth9Error,
  NotFoundError,
  UnauthorizedError,
  ForbiddenError,
  ValidationError,
  ConflictError,
  RateLimitError,
  BadRequestError,
  createErrorFromStatus,
} from "./errors.js";

// Utils
export { toSnakeCase, toCamelCase } from "./utils.js";
