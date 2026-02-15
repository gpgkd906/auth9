import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9HttpClient } from "../http-client.js";
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

  const tenantId = "tenant-123";

  describe("CRUD Operations", () => {
    it("creates an action with POST", async () => {
      const mockAction: Action = {
        id: "action-1",
        tenantId: tenantId,
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
        `/api/v1/tenants/${tenantId}/actions`,
        input
      );

      expect(result.data.id).toBe("action-1");
      expect(result.data.name).toBe("Add department claim");
      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/tenants/${tenantId}/actions`,
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
        `/api/v1/tenants/${tenantId}/actions`
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/tenants/${tenantId}/actions`,
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
        `/api/v1/tenants/${tenantId}/actions/${actionId}`
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/tenants/${tenantId}/actions/${actionId}`,
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
        `/api/v1/tenants/${tenantId}/actions/${actionId}`,
        input
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/tenants/${tenantId}/actions/${actionId}`,
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
        `/api/v1/tenants/${tenantId}/actions/${actionId}`
      );

      expect(mockFetch).toHaveBeenCalledWith(
        `https://auth9.example.com/api/v1/tenants/${tenantId}/actions/${actionId}`,
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
            tenantId: tenantId,
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
        `/api/v1/tenants/${tenantId}/actions/batch`,
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
        `/api/v1/tenants/${tenantId}/actions/${actionId}/test`,
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
        `/api/v1/tenants/${tenantId}/actions/${actionId}/stats`
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
          tenantId: tenantId,
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
        `/api/v1/tenants/${tenantId}/actions/logs`,
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
              {
                id: ActionTrigger.PostLogin,
                name: "Post Login",
                description: "Triggered after successful login",
              },
            ],
          }),
      });

      await client.get("/api/v1/triggers");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/triggers",
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
