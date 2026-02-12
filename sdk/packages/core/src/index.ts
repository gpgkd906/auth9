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
export type { Tenant, CreateTenantInput, TenantUser } from "./types/tenant.js";
export type { User, CreateUserInput } from "./types/user.js";
export type {
  Role,
  Permission,
  CreateRoleInput,
  CreatePermissionInput,
  RoleWithPermissions,
  AssignRolesInput,
  UserRolesInTenant,
} from "./types/rbac.js";
export type {
  Service,
  CreateServiceInput,
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
} from "./types/invitation.js";
export type {
  Webhook,
  CreateWebhookInput,
  WebhookTestResult,
} from "./types/webhook.js";
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
