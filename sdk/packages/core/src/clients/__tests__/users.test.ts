import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { User } from "../../types/user.js";
import type { Tenant } from "../../types/tenant.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("UsersClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockUser: User = {
    id: "user-1",
    email: "user@example.com",
    displayName: "Test User",
    mfaEnabled: false,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/users", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockUser] }),
      });

      const result = await client.users.list();

      expect(result).toEqual([mockUser]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/users/{id}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockUser }),
      });

      const result = await client.users.get("user-1");

      expect(result).toEqual(mockUser);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1",
        expect.objectContaining({ method: "GET" })
      );
    });

    it("throws NotFoundError on 404", async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        json: () => Promise.resolve({ code: "not_found", message: "User not found" }),
      });

      await expect(client.users.get("missing")).rejects.toThrow("User not found");
    });
  });

  describe("getMe", () => {
    it("sends GET /api/v1/users/me", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockUser }),
      });

      const result = await client.users.getMe();

      expect(result).toEqual(mockUser);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("updateMe", () => {
    it("sends PUT /api/v1/users/me", async () => {
      const updated = { ...mockUser, displayName: "New Name" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.users.updateMe({ displayName: "New Name" });

      expect(result.displayName).toBe("New Name");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/users", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockUser }),
      });

      const result = await client.users.create({ email: "user@example.com" });

      expect(result).toEqual(mockUser);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/users/{id}", async () => {
      const updated = { ...mockUser, displayName: "Updated" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.users.update("user-1", { displayName: "Updated" });

      expect(result.displayName).toBe("Updated");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/users/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.users.delete("user-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("enableMfa", () => {
    it("sends POST /api/v1/users/{id}/mfa", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.users.enableMfa("user-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/mfa",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("disableMfa", () => {
    it("sends DELETE /api/v1/users/{id}/mfa", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.users.disableMfa("user-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/mfa",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("getTenants", () => {
    it("sends GET /api/v1/users/{id}/tenants", async () => {
      const mockTenant: Tenant = {
        id: "tenant-1",
        name: "Test Tenant",
        slug: "test-tenant",
        settings: {},
        status: "active",
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockTenant] }),
      });

      const result = await client.users.getTenants("user-1");

      expect(result).toEqual([mockTenant]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("addToTenant", () => {
    it("sends POST /api/v1/users/{id}/tenants", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.users.addToTenant("user-1", {
        tenantId: "tenant-1",
        roleInTenant: "member",
      });

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("removeFromTenant", () => {
    it("sends DELETE /api/v1/users/{userId}/tenants/{tenantId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.users.removeFromTenant("user-1", "tenant-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants/tenant-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("updateRoleInTenant", () => {
    it("sends PUT /api/v1/users/{userId}/tenants/{tenantId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.users.updateRoleInTenant("user-1", "tenant-1", {
        roleInTenant: "admin",
      });

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants/tenant-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });
});
