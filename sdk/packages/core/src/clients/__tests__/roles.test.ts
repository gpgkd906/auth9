import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type {
  Role,
  Permission,
  RoleWithPermissions,
  UserRolesInTenant,
} from "../../types/rbac.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

const client = new Auth9Client({
  baseUrl: "https://auth9.example.com",
  apiKey: "test-token", // pragma: allowlist secret
});

const mockRole: Role = {
  id: "role-1",
  serviceId: "svc-1",
  name: "Admin",
  description: "Administrator role",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const mockPermission: Permission = {
  id: "perm-1",
  serviceId: "svc-1",
  code: "users:read",
  name: "Read Users",
};

describe("RolesClient", () => {
  describe("list", () => {
    it("sends GET /api/v1/services/{serviceId}/roles", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockRole] }),
      });

      const result = await client.roles.list("svc-1");

      expect(result).toEqual([mockRole]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/roles",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/roles/{id} and returns RoleWithPermissions", async () => {
      const roleWithPerms: RoleWithPermissions = {
        ...mockRole,
        permissions: [mockPermission],
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: roleWithPerms }),
      });

      const result = await client.roles.get("role-1");

      expect(result.permissions).toHaveLength(1);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/roles/role-1",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/roles", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockRole }),
      });

      const result = await client.roles.create({
        name: "Admin",
        description: "Administrator role",
      });

      expect(result).toEqual(mockRole);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/roles",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/roles/{id}", async () => {
      const updated = { ...mockRole, name: "Super Admin" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.roles.update("role-1", { name: "Super Admin" });

      expect(result.name).toBe("Super Admin");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/roles/role-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/roles/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.roles.delete("role-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/roles/role-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("assignPermission", () => {
    it("sends POST /api/v1/roles/{roleId}/permissions", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.roles.assignPermission("role-1", "perm-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/roles/role-1/permissions",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("removePermission", () => {
    it("sends DELETE /api/v1/roles/{roleId}/permissions/{permissionId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.roles.removePermission("role-1", "perm-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/roles/role-1/permissions/perm-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });
});

describe("PermissionsClient", () => {
  describe("list", () => {
    it("sends GET /api/v1/services/{serviceId}/permissions", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockPermission] }),
      });

      const result = await client.permissions.list("svc-1");

      expect(result).toEqual([mockPermission]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/permissions",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/permissions", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockPermission }),
      });

      const result = await client.permissions.create({
        serviceId: "svc-1",
        code: "users:read",
        name: "Read Users",
      });

      expect(result).toEqual(mockPermission);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/permissions",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/permissions/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.permissions.delete("perm-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/permissions/perm-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });
});

describe("RbacClient", () => {
  describe("assignRoles", () => {
    it("sends POST /api/v1/rbac/assign", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.rbac.assignRoles({
        userId: "user-1",
        tenantId: "tenant-1",
        roles: ["role-1", "role-2"],
      });

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/rbac/assign",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("getUserRoles", () => {
    it("sends GET /api/v1/users/{userId}/tenants/{tenantId}/roles", async () => {
      const mockUserRoles: UserRolesInTenant = {
        userId: "user-1",
        tenantId: "tenant-1",
        roles: ["admin"],
        permissions: ["users:read", "users:write"],
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockUserRoles }),
      });

      const result = await client.rbac.getUserRoles("user-1", "tenant-1");

      expect(result).toEqual(mockUserRoles);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants/tenant-1/roles",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("getUserAssignedRoles", () => {
    it("sends GET /api/v1/users/{userId}/tenants/{tenantId}/assigned-roles", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockRole] }),
      });

      const result = await client.rbac.getUserAssignedRoles("user-1", "tenant-1");

      expect(result).toEqual([mockRole]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants/tenant-1/assigned-roles",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("unassignRole", () => {
    it("sends DELETE /api/v1/users/{userId}/tenants/{tenantId}/roles/{roleId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.rbac.unassignRole("user-1", "tenant-1", "role-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/user-1/tenants/tenant-1/roles/role-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });
});
