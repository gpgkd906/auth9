import { Auth9HttpClient } from "@auth9/core";
import type {
  Action,
  CreateActionInput,
  UpdateActionInput,
  ActionContext,
  TestActionResponse,
  ActionExecution,
  ActionStats,
} from "@auth9/core";

/**
 * Get configured Auth9 SDK client
 */
export function getAuth9Client(accessToken?: string) {
  const baseUrl = process.env.AUTH9_CORE_URL || "http://localhost:8080";

  return new Auth9HttpClient({
    baseUrl,
    accessToken: accessToken || "",
  });
}

/**
 * Get available action triggers
 */
export function getTriggers(client: Auth9HttpClient) {
  return client.get<{ data: string[] }>("/api/v1/actions/triggers");
}

/**
 * Helper to make API requests with tenant context
 */
export function withTenant(client: Auth9HttpClient, tenantId: string) {
  return {
    actions: {
      list: (trigger?: string) => {
        const query = trigger ? `?trigger_id=${trigger}` : "";
        return client.get<{ data: Action[] }>(
          `/api/v1/tenants/${tenantId}/actions${query}`
        );
      },
      get: (id: string) =>
        client.get<{ data: Action }>(
          `/api/v1/tenants/${tenantId}/actions/${id}`
        ),
      create: (input: CreateActionInput) =>
        client.post<{ data: Action }>(
          `/api/v1/tenants/${tenantId}/actions`,
          input
        ),
      update: (id: string, input: UpdateActionInput) =>
        client.patch<{ data: Action }>(
          `/api/v1/tenants/${tenantId}/actions/${id}`,
          input
        ),
      delete: (id: string) =>
        client.delete(`/api/v1/tenants/${tenantId}/actions/${id}`),
      test: (id: string, context: ActionContext) =>
        client.post<{ data: TestActionResponse }>(
          `/api/v1/tenants/${tenantId}/actions/${id}/test`,
          { context }
        ),
      getLog: (logId: string) =>
        client.get<{ data: ActionExecution }>(
          `/api/v1/tenants/${tenantId}/actions/logs/${logId}`
        ),
      logs: (options?: { actionId?: string; success?: boolean; limit?: number }) => {
        const params = new URLSearchParams();
        if (options?.actionId) params.append("action_id", options.actionId);
        if (options?.success !== undefined) params.append("success", String(options.success));
        if (options?.limit) params.append("limit", String(options.limit));
        const query = params.toString();
        return client.get<{ data: ActionExecution[]; pagination: { page: number; per_page: number; total: number; total_pages: number } }>(
          `/api/v1/tenants/${tenantId}/actions/logs${query ? `?${query}` : ""}`
        );
      },
      stats: (id: string) =>
        client.get<{ data: ActionStats }>(
          `/api/v1/tenants/${tenantId}/actions/${id}/stats`
        ),
    },
  };
}
