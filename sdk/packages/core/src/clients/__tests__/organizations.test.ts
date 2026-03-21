import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { Organization } from "../../types/organization.js";
import type { Tenant } from "../../types/tenant.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("OrganizationsClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockOrganization: Organization = {
    id: "org-1",
    name: "Acme Corp",
    slug: "acme-corp",
    domain: "acme.example.com",
    status: "pending",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  const mockTenant: Tenant = {
    id: "tenant-1",
    name: "Acme Corp",
    slug: "acme-corp",
    settings: {},
    status: "active",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("create", () => {
    it("sends POST /api/v1/organizations", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockOrganization }),
      });

      const result = await client.organizations.create({
        name: "Acme Corp",
        slug: "acme-corp",
        domain: "acme.example.com",
      });

      expect(result).toEqual(mockOrganization);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/organizations",
        expect.objectContaining({ method: "POST" }),
      );
    });
  });

  describe("getMyTenants", () => {
    it("sends GET /api/v1/users/me/tenants", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockTenant] }),
      });

      const result = await client.organizations.getMyTenants();

      expect(result).toEqual([mockTenant]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/tenants",
        expect.objectContaining({ method: "GET" }),
      );
    });

    it("sends GET with service_id query param", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockTenant] }),
      });

      await client.organizations.getMyTenants("svc-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/tenants?service_id=svc-1",
        expect.objectContaining({ method: "GET" }),
      );
    });
  });
});
