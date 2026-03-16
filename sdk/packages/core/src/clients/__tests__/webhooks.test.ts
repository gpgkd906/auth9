import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { Webhook, WebhookTestResult } from "../../types/webhook.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("WebhooksClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockWebhook: Webhook = {
    id: "wh-1",
    tenantId: "tenant-1",
    name: "User Created",
    url: "https://example.com/webhooks",
    events: ["user.created", "user.updated"],
    enabled: true,
    failureCount: 0,
    createdAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/tenants/{id}/webhooks", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockWebhook] }),
      });

      const result = await client.webhooks.list("tenant-1");

      expect(result).toEqual([mockWebhook]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/tenants/{id}/webhooks/{webhookId}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockWebhook }),
      });

      const result = await client.webhooks.get("tenant-1", "wh-1");

      expect(result).toEqual(mockWebhook);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks/wh-1",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/tenants/{id}/webhooks", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockWebhook }),
      });

      const result = await client.webhooks.create("tenant-1", {
        name: "User Created",
        url: "https://example.com/webhooks",
        events: ["user.created", "user.updated"],
      });

      expect(result).toEqual(mockWebhook);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/tenants/{id}/webhooks/{webhookId}", async () => {
      const updated = { ...mockWebhook, name: "Updated Webhook" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.webhooks.update("tenant-1", "wh-1", {
        name: "Updated Webhook",
      });

      expect(result.name).toBe("Updated Webhook");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks/wh-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/tenants/{id}/webhooks/{webhookId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.webhooks.delete("tenant-1", "wh-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks/wh-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("test", () => {
    it("sends POST /api/v1/tenants/{id}/webhooks/{webhookId}/test", async () => {
      const mockResult: WebhookTestResult = {
        success: true,
        statusCode: 200,
        responseTimeMs: 150,
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockResult }),
      });

      const result = await client.webhooks.test("tenant-1", "wh-1");

      expect(result).toEqual(mockResult);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks/wh-1/test",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("regenerateSecret", () => {
    it("sends POST /api/v1/tenants/{id}/webhooks/{webhookId}/regenerate-secret", async () => {
      const withSecret = { ...mockWebhook, secret: "new-secret" }; // pragma: allowlist secret
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: withSecret }),
      });

      const result = await client.webhooks.regenerateSecret("tenant-1", "wh-1");

      expect(result.secret).toBe("new-secret");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/webhooks/wh-1/regenerate-secret",
        expect.objectContaining({ method: "POST" })
      );
    });
  });
});
