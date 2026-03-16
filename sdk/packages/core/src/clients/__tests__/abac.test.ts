import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { AbacPolicy, AbacSimulationResult } from "../../types/abac.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("AbacClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockPolicy: AbacPolicy = {
    id: "policy-1",
    tenantId: "tenant-1",
    name: "Admin Access",
    versionId: "v-1",
    status: "draft",
    rules: [
      {
        effect: "allow",
        subjects: { role: "admin" },
        resources: { type: "document" },
        actions: ["read", "write"],
      },
    ],
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("listPolicies", () => {
    it("sends GET /api/v1/tenants/{id}/abac/policies", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockPolicy] }),
      });

      const result = await client.abac.listPolicies("tenant-1");

      expect(result).toEqual([mockPolicy]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/abac/policies",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("createPolicy", () => {
    it("sends POST /api/v1/tenants/{id}/abac/policies", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockPolicy }),
      });

      const result = await client.abac.createPolicy("tenant-1", {
        name: "Admin Access",
        rules: mockPolicy.rules,
      });

      expect(result).toEqual(mockPolicy);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/abac/policies",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("updatePolicy", () => {
    it("sends PUT /api/v1/tenants/{id}/abac/policies/{versionId}", async () => {
      const updated = { ...mockPolicy, name: "Updated Policy" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.abac.updatePolicy("tenant-1", "v-1", {
        name: "Updated Policy",
      });

      expect(result.name).toBe("Updated Policy");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/abac/policies/v-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("publishPolicy", () => {
    it("sends POST /api/v1/tenants/{id}/abac/policies/{versionId}/publish", async () => {
      const published = { ...mockPolicy, status: "published" as const };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: published }),
      });

      const result = await client.abac.publishPolicy("tenant-1", "v-1");

      expect(result.status).toBe("published");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/abac/policies/v-1/publish",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("rollbackPolicy", () => {
    it("sends POST /api/v1/tenants/{id}/abac/policies/{versionId}/rollback", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockPolicy }),
      });

      const result = await client.abac.rollbackPolicy("tenant-1", "v-1");

      expect(result).toEqual(mockPolicy);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/abac/policies/v-1/rollback",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("simulate", () => {
    it("sends POST /api/v1/tenants/{id}/abac/simulate", async () => {
      const mockResult: AbacSimulationResult = {
        allowed: true,
        matchedPolicies: ["policy-1"],
        reason: "Matched admin rule",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockResult }),
      });

      const result = await client.abac.simulate("tenant-1", {
        subject: { role: "admin" },
        resource: { type: "document" },
        action: "read",
      });

      expect(result).toEqual(mockResult);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/abac/simulate",
        expect.objectContaining({ method: "POST" })
      );
    });
  });
});
