import { Auth9HttpClient } from "./http-client.js";
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
} from "./types/action.js";

export interface Auth9ClientConfig {
  baseUrl: string;
  apiKey: string;
  tenantId?: string;
}

export class Auth9Client {
  private http: Auth9HttpClient;
  private tenantId?: string;

  constructor(config: Auth9ClientConfig) {
    this.http = new Auth9HttpClient({
      baseUrl: config.baseUrl,
      accessToken: config.apiKey,
    });
    this.tenantId = config.tenantId;
  }

  setTenantId(tenantId: string) {
    this.tenantId = tenantId;
  }

  get actions() {
    if (!this.tenantId) {
      throw new Error("tenantId must be set to use actions API");
    }
    const tenantId = this.tenantId;

    return {
      list: async (triggerId?: string) => {
        const params: Record<string, string> = {};
        if (triggerId) params.trigger_id = triggerId;
        const result = await this.http.get<{ data: Action[] }>(
          `/api/v1/tenants/${tenantId}/actions`,
          params
        );
        return result.data;
      },
      get: async (id: string) => {
        const result = await this.http.get<{ data: Action }>(
          `/api/v1/tenants/${tenantId}/actions/${id}`
        );
        return result.data;
      },
      create: async (input: CreateActionInput) => {
        const result = await this.http.post<{ data: Action }>(
          `/api/v1/tenants/${tenantId}/actions`,
          input
        );
        return result.data;
      },
      update: async (id: string, input: UpdateActionInput) => {
        const result = await this.http.patch<{ data: Action }>(
          `/api/v1/tenants/${tenantId}/actions/${id}`,
          input
        );
        return result.data;
      },
      delete: async (id: string) => {
        await this.http.delete(`/api/v1/tenants/${tenantId}/actions/${id}`);
      },
      test: async (id: string, context: ActionContext) => {
        const result = await this.http.post<{ data: TestActionResponse }>(
          `/api/v1/tenants/${tenantId}/actions/${id}/test`,
          { context }
        );
        return result.data;
      },
      batchUpsert: async (actions: UpsertActionInput[]) => {
        const result = await this.http.post<{ data: BatchUpsertResponse }>(
          `/api/v1/tenants/${tenantId}/actions/batch`,
          { actions }
        );
        return result.data;
      },
      logs: async (options?: { actionId?: string; success?: boolean; limit?: number }) => {
        const params: Record<string, string> = {};
        if (options?.actionId) params.action_id = options.actionId;
        if (options?.success !== undefined) params.success = String(options.success);
        if (options?.limit) params.limit = String(options.limit);
        
        const result = await this.http.get<{ data: ActionExecution[] }>(
          `/api/v1/tenants/${tenantId}/actions/logs`,
          params
        );
        return result.data;
      },
      stats: async (id: string) => {
        const result = await this.http.get<{ data: ActionStats }>(
          `/api/v1/tenants/${tenantId}/actions/${id}/stats`
        );
        return result.data;
      },
    };
  }
}
