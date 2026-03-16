import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { Tenant, MaliciousIpBlacklistEntry } from "../../types/tenant.js";
import type { User } from "../../types/user.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("TenantsClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockTenant: Tenant = {
    id: "tenant-1",
    name: "Test Tenant",
    slug: "test-tenant",
    settings: {},
    status: "active",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/tenants", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockTenant] }),
      });

      const result = await client.tenants.list();

      expect(result).toEqual([mockTenant]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/tenants/{id}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockTenant }),
      });

      const result = await client.tenants.get("tenant-1");

      expect(result).toEqual(mockTenant);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1",
        expect.objectContaining({ method: "GET" })
      );
    });

    it("throws NotFoundError on 404", async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        json: () => Promise.resolve({ code: "not_found", message: "Tenant not found" }),
      });

      await expect(client.tenants.get("missing")).rejects.toThrow("Tenant not found");
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/tenants", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockTenant }),
      });

      const result = await client.tenants.create({
        name: "Test Tenant",
        slug: "test-tenant",
      });

      expect(result).toEqual(mockTenant);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/tenants/{id}", async () => {
      const updated = { ...mockTenant, name: "Updated" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.tenants.update("tenant-1", { name: "Updated" });

      expect(result.name).toBe("Updated");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/tenants/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.tenants.delete("tenant-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("listUsers", () => {
    it("sends GET /api/v1/tenants/{id}/users", async () => {
      const mockUser: User = {
        id: "user-1",
        email: "user@example.com",
        mfaEnabled: false,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockUser] }),
      });

      const result = await client.tenants.listUsers("tenant-1");

      expect(result).toEqual([mockUser]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/users",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("getMaliciousIpBlacklist", () => {
    it("sends GET /api/v1/tenants/{id}/security/malicious-ip-blacklist", async () => {
      const mockEntry: MaliciousIpBlacklistEntry = {
        id: "entry-1",
        tenantId: "tenant-1",
        ipAddress: "192.168.1.1",
        reason: "Brute force",
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockEntry] }),
      });

      const result = await client.tenants.getMaliciousIpBlacklist("tenant-1");

      expect(result).toEqual([mockEntry]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/security/malicious-ip-blacklist",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("updateMaliciousIpBlacklist", () => {
    it("sends PUT /api/v1/tenants/{id}/security/malicious-ip-blacklist", async () => {
      const mockEntry: MaliciousIpBlacklistEntry = {
        id: "entry-1",
        tenantId: "tenant-1",
        ipAddress: "10.0.0.1",
        reason: "Suspicious",
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockEntry] }),
      });

      const result = await client.tenants.updateMaliciousIpBlacklist("tenant-1", {
        entries: [{ ipAddress: "10.0.0.1", reason: "Suspicious" }],
      });

      expect(result).toEqual([mockEntry]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/security/malicious-ip-blacklist",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });
});
