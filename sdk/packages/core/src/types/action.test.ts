import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9HttpClient } from "../http-client.js";
import { Auth9Client } from "../auth9-client.js";
import type {
  Action,
  CreateActionInput,
  UpdateActionInput,
  BatchUpsertResponse,
  TestActionResponse,
  ActionExecution,
  ActionStats,
  UpsertActionInput,
} from "./action.js";
import { ActionTrigger } from "./action.js";

// Mock global fetch
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("Action Types with HTTP Client", () => {
  const client = new Auth9HttpClient({
    baseUrl: "https://auth9.example.com",
    accessToken: "test-token",
  });

  const serviceId = "service-123";
  const tenantId = "tenant-123";

  describe("CRUD Operations", () => {
    it("creates an action with POST", async () => {
      const mockAction: Action = {
        id: "action-1",
        serviceId: serviceId,
        name: "Add department claim",
        triggerId: ActionTrigger.PostLogin,
        script: 'context.claims.department = "engineering"; context;',
        enabled: true,
        strictMode: false,
        executionOrder: 0,
        timeoutMs: 3000,
        executionCount: 0,
        errorCount: 0,
        createdAt: "2026-02-12T10:00:00Z",
        updatedAt: "2026-02-12T10:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockAction }),
      });

      const input: CreateActionInput = {
        name: "Add department claim",
        triggerId: ActionTrigger.PostLogin,
        script: 'context.claims.department = "engineering"; context;',
        enabled: true,
        executionOrder: 0,
        timeoutMs: 3000,
      };

      const result = await client.post<{ data: Action }>(
        `/api/v1/services/${serviceId}/actions`,
        input
      );

      expect(result.data.id).toBe("action-1");
      expect(result.data.name).toBe("Add department claim");
      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/services/${serviceId}/actions`,
        expect.objectContaining({
          method: "POST",
        })
      );
    });

    it("lists actions with GET", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [] }),
      });

      await client.get<{ data: Action[] }>(
        `/api/v1/services/${serviceId}/actions`
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/services/${serviceId}/actions`,
        expect.objectContaining({
          method: "GET",
        })
      );
    });

    it("gets single action by ID with GET", async () => {
      const actionId = "action-1";
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: { id: actionId } }),
      });

      await client.get<{ data: Action }>(
        `/api/v1/services/${serviceId}/actions/${actionId}`
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/services/${serviceId}/actions/${actionId}`,
        expect.objectContaining({
          method: "GET",
        })
      );
    });

    it("updates action with PATCH", async () => {
      const actionId = "action-1";
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({ data: { id: actionId, enabled: false } }),
      });

      const input: UpdateActionInput = {
        enabled: false,
      };

      await client.patch<{ data: Action }>(
        `/api/v1/services/${serviceId}/actions/${actionId}`,
        input
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/services/${serviceId}/actions/${actionId}`,
        expect.objectContaining({
          method: "PATCH",
        })
      );
    });

    it("deletes action with DELETE", async () => {
      const actionId = "action-1";
      mockFetch.mockResolvedValue({
        ok: true,
        status: 204,
      });

      await client.delete(
        `/api/v1/services/${serviceId}/actions/${actionId}`
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/services/${serviceId}/actions/${actionId}`,
        expect.objectContaining({
          method: "DELETE",
        })
      );
    });
  });

  describe("Batch Operations", () => {
    it("batch upserts actions with POST", async () => {
      const mockResponse: BatchUpsertResponse = {
        created: [
          {
            id: "action-1",
            serviceId: serviceId,
            name: "Action 1",
            triggerId: ActionTrigger.PostLogin,
            script: "context;",
            enabled: true,
            strictMode: false,
            executionOrder: 0,
            timeoutMs: 3000,
            executionCount: 0,
            errorCount: 0,
            createdAt: "2026-02-12T10:00:00Z",
            updatedAt: "2026-02-12T10:00:00Z",
          },
        ],
        updated: [],
        errors: [],
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockResponse }),
      });

      const inputs: UpsertActionInput[] = [
        {
          name: "Action 1",
          triggerId: ActionTrigger.PostLogin,
          script: "context;",
          enabled: true,
          strictMode: false,
          executionOrder: 0,
          timeoutMs: 3000,
        },
      ];

      const result = await client.post<{ data: BatchUpsertResponse }>(
        `/api/v1/services/${serviceId}/actions/batch`,
        { actions: inputs }
      );

      expect(result.data.created).toHaveLength(1);
      expect(result.data.errors).toHaveLength(0);
    });
  });

  describe("Test and Stats", () => {
    it("tests action with POST", async () => {
      const actionId = "action-1";
      const mockResponse: TestActionResponse = {
        success: true,
        durationMs: 15,
        modifiedContext: {
          user: {
            id: "user-1",
            email: "test@example.com",
            mfaEnabled: false,
          },
          tenant: {
            id: tenantId,
            slug: "test-tenant",
            name: "Test Tenant",
          },
          request: {
            ip: "127.0.0.1",
            timestamp: "2026-02-12T10:00:00Z",
          },
          claims: {
            department: "engineering",
          },
        },
        consoleLogs: ["Test log"],
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockResponse }),
      });

      const result = await client.post<{ data: TestActionResponse }>(
        `/api/v1/services/${serviceId}/actions/${actionId}/test`,
        {
          context: {
            user: {
              id: "user-1",
              email: "test@example.com",
              mfaEnabled: false,
            },
            tenant: {
              id: tenantId,
              slug: "test-tenant",
              name: "Test Tenant",
            },
            request: {
              ip: "127.0.0.1",
              timestamp: "2026-02-12T10:00:00Z",
            },
          },
        }
      );

      expect(result.data.success).toBe(true);
      expect(result.data.durationMs).toBe(15);
      expect(result.data.modifiedContext?.claims?.department).toBe(
        "engineering"
      );
    });

    it("gets action stats with GET", async () => {
      const actionId = "action-1";
      const mockStats: ActionStats = {
        executionCount: 100,
        errorCount: 5,
        avgDurationMs: 12,
        last24hCount: 25,
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockStats }),
      });

      const result = await client.get<{ data: ActionStats }>(
        `/api/v1/services/${serviceId}/actions/${actionId}/stats`
      );

      expect(result.data.executionCount).toBe(100);
      expect(result.data.errorCount).toBe(5);
      expect(result.data.avgDurationMs).toBe(12);
    });
  });

  describe("Logs and Triggers", () => {
    it("queries action logs with GET", async () => {
      const mockLogs: ActionExecution[] = [
        {
          id: "exec-1",
          actionId: "action-1",
          serviceId: serviceId,
          triggerId: ActionTrigger.PostLogin,
          userId: "user-1",
          success: true,
          durationMs: 10,
          executedAt: "2026-02-12T10:00:00Z",
        },
      ];

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockLogs }),
      });

      const result = await client.get<{ data: ActionExecution[] }>(
        `/api/v1/services/${serviceId}/actions/logs`,
        { success: "true", limit: "100" }
      );

      expect(result.data).toHaveLength(1);
      expect(result.data[0].success).toBe(true);
    });

    it("gets available triggers with GET", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            data: [
              ActionTrigger.PostLogin,
              ActionTrigger.PreUserRegistration,
              ActionTrigger.PostUserRegistration,
              ActionTrigger.PostChangePassword,
              ActionTrigger.PostEmailVerification,
              ActionTrigger.PreTokenRefresh,
            ],
          }),
      });

      await client.get("/api/v1/actions/triggers");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/actions/triggers",
        expect.objectContaining({
          method: "GET",
        })
      );
    });
  });

  describe("ActionTrigger enum", () => {
    it("has all expected triggers", () => {
      expect(ActionTrigger.PostLogin).toBe("post-login");
      expect(ActionTrigger.PreUserRegistration).toBe("pre-user-registration");
      expect(ActionTrigger.PostUserRegistration).toBe("post-user-registration");
      expect(ActionTrigger.PostChangePassword).toBe("post-change-password");
      expect(ActionTrigger.PostEmailVerification).toBe("post-email-verification");
      expect(ActionTrigger.PreTokenRefresh).toBe("pre-token-refresh");
    });
  });
});

describe("Auth9Client Actions API", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
    serviceId: "service-123",
  });

  const serviceId = "service-123";

  describe("getTriggers", () => {
    it("sends GET /api/v1/actions/triggers", async () => {
      const triggers = [
        ActionTrigger.PostLogin,
        ActionTrigger.PreUserRegistration,
        ActionTrigger.PostUserRegistration,
        ActionTrigger.PostChangePassword,
        ActionTrigger.PostEmailVerification,
        ActionTrigger.PreTokenRefresh,
      ];

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: triggers }),
      });

      const result = await client.actions.getTriggers();

      expect(result).toEqual(triggers);
      expect(result).toHaveLength(6);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/actions/triggers",
        expect.objectContaining({ method: "GET" }),
      );
    });
  });

  describe("getLog", () => {
    it("sends GET /api/v1/services/{serviceId}/actions/logs/{logId}", async () => {
      const mockLog: ActionExecution = {
        id: "exec-1",
        actionId: "action-1",
        serviceId: serviceId,
        triggerId: ActionTrigger.PostLogin,
        userId: "user-1",
        success: true,
        durationMs: 10,
        executedAt: "2026-02-12T10:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockLog }),
      });

      const result = await client.actions.getLog("exec-1");

      expect(result.id).toBe("exec-1");
      expect(result.success).toBe(true);
      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/services/${serviceId}/actions/logs/exec-1`,
        expect.objectContaining({ method: "GET" }),
      );
    });
  });

  describe("logs with full filter", () => {
    it("sends all query params from LogQueryFilter", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            data: [],
            pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          }),
      });

      await client.actions.logs({
        actionId: "action-1",
        triggerId: "post-login",
        userId: "user-1",
        success: true,
        from: "2026-01-01T00:00:00Z",
        to: "2026-01-31T00:00:00Z",
        limit: 100,
        offset: 50,
      });

      const calledUrl = mockFetch.mock.calls[0][0] as string;
      expect(calledUrl).toContain("action_id=action-1");
      expect(calledUrl).toContain("trigger_id=post-login");
      expect(calledUrl).toContain("user_id=user-1");
      expect(calledUrl).toContain("success=true");
      expect(calledUrl).toContain("from=2026-01-01T00%3A00%3A00Z");
      expect(calledUrl).toContain("to=2026-01-31T00%3A00%3A00Z");
      expect(calledUrl).toContain("limit=100");
      expect(calledUrl).toContain("offset=50");
    });

    it("returns PaginatedResponse", async () => {
      const mockLog: ActionExecution = {
        id: "exec-1",
        actionId: "action-1",
        serviceId: serviceId,
        triggerId: ActionTrigger.PostLogin,
        success: true,
        durationMs: 10,
        executedAt: "2026-02-12T10:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () =>
          Promise.resolve({
            data: [mockLog],
            pagination: { page: 1, per_page: 50, total: 1, total_pages: 1 },
          }),
      });

      const result = await client.actions.logs();

      expect(result.data).toHaveLength(1);
      expect(result.pagination).toBeDefined();
      expect(result.pagination.total).toBe(1);
    });
  });
});
