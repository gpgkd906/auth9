export enum ActionTrigger {
  PostLogin = "post-login",
  PreUserRegistration = "pre-user-registration",
  PostUserRegistration = "post-user-registration",
  PostChangePassword = "post-change-password",
  PostEmailVerification = "post-email-verification",
  PreTokenRefresh = "pre-token-refresh",
}

export interface Action {
  id: string;
  tenantId: string;
  name: string;
  description?: string;
  triggerId: string;
  script: string;
  enabled: boolean;
  strictMode: boolean;
  executionOrder: number;
  timeoutMs: number;
  lastExecutedAt?: string;
  executionCount: number;
  errorCount: number;
  lastError?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateActionInput {
  name: string;
  description?: string;
  triggerId: string;
  script: string;
  enabled?: boolean;
  strictMode?: boolean;
  executionOrder?: number;
  timeoutMs?: number;
}

export interface UpdateActionInput {
  name?: string;
  description?: string;
  script?: string;
  enabled?: boolean;
  strictMode?: boolean;
  executionOrder?: number;
  timeoutMs?: number;
}

export interface ActionContextUser {
  id: string;
  email: string;
  displayName?: string;
  mfaEnabled: boolean;
}

export interface ActionContextTenant {
  id: string;
  slug: string;
  name: string;
}

export interface ActionContextRequest {
  ip?: string;
  userAgent?: string;
  timestamp: string;
}

export interface ActionContext {
  user: ActionContextUser;
  tenant: ActionContextTenant;
  request: ActionContextRequest;
  claims?: Record<string, unknown>;
}

export interface TestActionResponse {
  success: boolean;
  durationMs: number;
  modifiedContext?: ActionContext;
  errorMessage?: string;
  consoleLogs: string[];
}

export interface ActionExecution {
  id: string;
  actionId: string;
  tenantId: string;
  triggerId: string;
  userId?: string;
  success: boolean;
  durationMs: number;
  errorMessage?: string;
  executedAt: string;
}

export interface ActionStats {
  executionCount: number;
  errorCount: number;
  avgDurationMs: number;
  last24hCount: number;
}

export interface UpsertActionInput {
  id?: string;
  name: string;
  description?: string;
  triggerId: string;
  script: string;
  enabled: boolean;
  strictMode: boolean;
  executionOrder: number;
  timeoutMs: number;
}

export interface BatchError {
  inputIndex: number;
  name: string;
  error: string;
}

export interface BatchUpsertResponse {
  created: Action[];
  updated: Action[];
  errors: BatchError[];
}

export interface LogQueryFilter {
  actionId?: string;
  userId?: string;
  success?: boolean;
  from?: string;
  to?: string;
  limit?: number;
  offset?: number;
}
