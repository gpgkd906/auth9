import { Auth9HttpClient } from "./http-client.js";
import { TenantsClient } from "./clients/tenants.js";
import { UsersClient } from "./clients/users.js";
import { ServicesClient } from "./clients/services.js";
import { RolesClient } from "./clients/roles.js";
import { PermissionsClient } from "./clients/permissions.js";
import { RbacClient } from "./clients/rbac.js";
import { InvitationsClient } from "./clients/invitations.js";
import { IdentityProvidersClient } from "./clients/identity-providers.js";
import { SsoClient } from "./clients/sso.js";
import { SamlClient } from "./clients/saml.js";
import { AbacClient } from "./clients/abac.js";
import { SessionsClient } from "./clients/sessions.js";
import { WebhooksClient } from "./clients/webhooks.js";
import { ScimClient } from "./clients/scim.js";
import { TenantServicesClient } from "./clients/tenant-services.js";
import { PasswordClient } from "./clients/password.js";
import { PasskeysClient } from "./clients/passkeys.js";
import { EmailOtpClient } from "./clients/email-otp.js";
import { AuthClient } from "./clients/auth.js";
import { OrganizationsClient } from "./clients/organizations.js";
import { AuditLogsClient } from "./clients/audit-logs.js";
import { AnalyticsClient } from "./clients/analytics.js";
import { SecurityAlertsClient } from "./clients/security-alerts.js";
import { SystemClient } from "./clients/system.js";
import { EmailTemplatesClient } from "./clients/email-templates.js";
import { BrandingClient } from "./clients/branding.js";
import type {
  Action,
  CreateActionInput,
  UpdateActionInput,
  ActionContext,
  TestActionResponse,
  ActionExecution,
  ActionStats,
  UpsertActionInput,
  BatchUpsertResponse,
  LogQueryFilter,
} from "./types/action.js";
import { ActionTrigger } from "./types/action.js";
import type { PaginatedResponse } from "./types/responses.js";

export interface Auth9ClientConfig {
  baseUrl: string;
  apiKey: string;
  tenantId?: string;
  serviceId?: string;
}

export class Auth9Client {
  private http: Auth9HttpClient;
  private tenantId?: string;
  private serviceId?: string;

  private _tenants?: TenantsClient;
  private _users?: UsersClient;
  private _services?: ServicesClient;
  private _roles?: RolesClient;
  private _permissions?: PermissionsClient;
  private _rbac?: RbacClient;
  private _invitations?: InvitationsClient;
  private _identityProviders?: IdentityProvidersClient;
  private _sso?: SsoClient;
  private _saml?: SamlClient;
  private _abac?: AbacClient;
  private _sessions?: SessionsClient;
  private _webhooks?: WebhooksClient;
  private _scim?: ScimClient;
  private _tenantServices?: TenantServicesClient;
  private _password?: PasswordClient;
  private _passkeys?: PasskeysClient;
  private _emailOtp?: EmailOtpClient;
  private _auth?: AuthClient;
  private _organizations?: OrganizationsClient;
  private _auditLogs?: AuditLogsClient;
  private _analytics?: AnalyticsClient;
  private _securityAlerts?: SecurityAlertsClient;
  private _system?: SystemClient;
  private _emailTemplates?: EmailTemplatesClient;
  private _branding?: BrandingClient;

  constructor(config: Auth9ClientConfig) {
    this.http = new Auth9HttpClient({
      baseUrl: config.baseUrl,
      accessToken: config.apiKey,
    });
    this.tenantId = config.tenantId;
    this.serviceId = config.serviceId;
  }

  setTenantId(tenantId: string) {
    this.tenantId = tenantId;
  }

  setServiceId(serviceId: string) {
    this.serviceId = serviceId;
  }

  private requireServiceId(): string {
    if (!this.serviceId) {
      throw new Error("serviceId must be set to use actions API");
    }
    return this.serviceId;
  }

  get tenants(): TenantsClient {
    return (this._tenants ??= new TenantsClient(this.http));
  }

  get users(): UsersClient {
    return (this._users ??= new UsersClient(this.http));
  }

  get services(): ServicesClient {
    return (this._services ??= new ServicesClient(this.http));
  }

  get roles(): RolesClient {
    return (this._roles ??= new RolesClient(this.http));
  }

  get permissions(): PermissionsClient {
    return (this._permissions ??= new PermissionsClient(this.http));
  }

  get rbac(): RbacClient {
    return (this._rbac ??= new RbacClient(this.http));
  }

  get invitations(): InvitationsClient {
    return (this._invitations ??= new InvitationsClient(this.http));
  }

  get identityProviders(): IdentityProvidersClient {
    return (this._identityProviders ??= new IdentityProvidersClient(this.http));
  }

  get sso(): SsoClient {
    return (this._sso ??= new SsoClient(this.http));
  }

  get saml(): SamlClient {
    return (this._saml ??= new SamlClient(this.http));
  }

  get abac(): AbacClient {
    return (this._abac ??= new AbacClient(this.http));
  }

  get sessions(): SessionsClient {
    return (this._sessions ??= new SessionsClient(this.http));
  }

  get webhooks(): WebhooksClient {
    return (this._webhooks ??= new WebhooksClient(this.http));
  }

  get scim(): ScimClient {
    return (this._scim ??= new ScimClient(this.http));
  }

  get tenantServices(): TenantServicesClient {
    return (this._tenantServices ??= new TenantServicesClient(this.http));
  }

  get password(): PasswordClient {
    return (this._password ??= new PasswordClient(this.http));
  }

  get passkeys(): PasskeysClient {
    return (this._passkeys ??= new PasskeysClient(this.http));
  }

  get emailOtp(): EmailOtpClient {
    return (this._emailOtp ??= new EmailOtpClient(this.http));
  }

  get auth(): AuthClient {
    return (this._auth ??= new AuthClient(this.http, this.http.getBaseUrl()));
  }

  get organizations(): OrganizationsClient {
    return (this._organizations ??= new OrganizationsClient(this.http));
  }

  get auditLogs(): AuditLogsClient {
    return (this._auditLogs ??= new AuditLogsClient(this.http));
  }

  get analytics(): AnalyticsClient {
    return (this._analytics ??= new AnalyticsClient(this.http));
  }

  get securityAlerts(): SecurityAlertsClient {
    return (this._securityAlerts ??= new SecurityAlertsClient(this.http));
  }

  get system(): SystemClient {
    return (this._system ??= new SystemClient(this.http));
  }

  get emailTemplates(): EmailTemplatesClient {
    return (this._emailTemplates ??= new EmailTemplatesClient(this.http));
  }

  get branding(): BrandingClient {
    return (this._branding ??= new BrandingClient(this.http));
  }

  get actions() {
    const serviceId = this.requireServiceId();

    return {
      list: async (triggerId?: string) => {
        const params: Record<string, string> = {};
        if (triggerId) params.trigger_id = triggerId;
        const result = await this.http.get<{ data: Action[] }>(
          `/api/v1/services/${serviceId}/actions`,
          params
        );
        return result.data;
      },
      get: async (id: string) => {
        const result = await this.http.get<{ data: Action }>(
          `/api/v1/services/${serviceId}/actions/${id}`
        );
        return result.data;
      },
      create: async (input: CreateActionInput) => {
        const result = await this.http.post<{ data: Action }>(
          `/api/v1/services/${serviceId}/actions`,
          input
        );
        return result.data;
      },
      update: async (id: string, input: UpdateActionInput) => {
        const result = await this.http.patch<{ data: Action }>(
          `/api/v1/services/${serviceId}/actions/${id}`,
          input
        );
        return result.data;
      },
      delete: async (id: string) => {
        await this.http.delete(`/api/v1/services/${serviceId}/actions/${id}`);
      },
      test: async (id: string, context: ActionContext) => {
        const result = await this.http.post<{ data: TestActionResponse }>(
          `/api/v1/services/${serviceId}/actions/${id}/test`,
          { context }
        );
        return result.data;
      },
      batchUpsert: async (actions: UpsertActionInput[]) => {
        const result = await this.http.post<{ data: BatchUpsertResponse }>(
          `/api/v1/services/${serviceId}/actions/batch`,
          { actions }
        );
        return result.data;
      },
      logs: async (options?: LogQueryFilter) => {
        const params: Record<string, string> = {};
        if (options?.actionId) params.action_id = options.actionId;
        if (options?.triggerId) params.trigger_id = options.triggerId;
        if (options?.userId) params.user_id = options.userId;
        if (options?.success !== undefined) params.success = String(options.success);
        if (options?.from) params.from = options.from;
        if (options?.to) params.to = options.to;
        if (options?.limit) params.limit = String(options.limit);
        if (options?.offset) params.offset = String(options.offset);

        return this.http.get<PaginatedResponse<ActionExecution>>(
          `/api/v1/services/${serviceId}/actions/logs`,
          params
        );
      },
      getLog: async (logId: string) => {
        const result = await this.http.get<{ data: ActionExecution }>(
          `/api/v1/services/${serviceId}/actions/logs/${logId}`
        );
        return result.data;
      },
      stats: async (id: string) => {
        const result = await this.http.get<{ data: ActionStats }>(
          `/api/v1/services/${serviceId}/actions/${id}/stats`
        );
        return result.data;
      },
      getTriggers: async () => {
        const result = await this.http.get<{ data: ActionTrigger[] }>(
          `/api/v1/actions/triggers`
        );
        return result.data;
      },
    };
  }
}
