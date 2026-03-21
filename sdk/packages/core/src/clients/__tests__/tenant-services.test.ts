import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { TenantServiceInfo } from "../../types/tenant-service.js";
import type { Service } from "../../types/service.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("TenantServicesClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  describe("list", () => {
    it("sends GET /api/v1/tenants/{id}/services", async () => {
      const mockInfo: TenantServiceInfo = {
        serviceId: "svc-1",
        serviceName: "User Management",
        enabled: true,
        enabledAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockInfo] }),
      });

      const result = await client.tenantServices.list("tenant-1");

      expect(result).toEqual([mockInfo]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/services",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("toggle", () => {
    it("sends POST /api/v1/tenants/{id}/services", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.tenantServices.toggle("tenant-1", {
        serviceId: "svc-1",
        enabled: false,
      });

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/services",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("getEnabled", () => {
    it("sends GET /api/v1/tenants/{id}/services/enabled", async () => {
      const mockService: Service = {
        id: "svc-1",
        name: "User Management",
        redirectUris: [],
        logoutUris: [],
        status: "active",
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockService] }),
      });

      const result = await client.tenantServices.getEnabled("tenant-1");

      expect(result).toEqual([mockService]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/services/enabled",
        expect.objectContaining({ method: "GET" })
      );
    });
  });
});
