import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { SSOConnector, SSOTestResult } from "../../types/sso.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("SsoClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockConnector: SSOConnector = {
    id: "conn-1",
    tenantId: "tenant-1",
    name: "Okta SAML",
    protocol: "saml",
    domains: ["example.com"],
    config: { entityId: "https://example.com" },
    enabled: true,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("listConnectors", () => {
    it("sends GET /api/v1/tenants/{id}/sso/connectors", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockConnector] }),
      });

      const result = await client.sso.listConnectors("tenant-1");

      expect(result).toEqual([mockConnector]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/sso/connectors",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("createConnector", () => {
    it("sends POST /api/v1/tenants/{id}/sso/connectors", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockConnector }),
      });

      const result = await client.sso.createConnector("tenant-1", {
        name: "Okta SAML",
        protocol: "saml",
        domains: ["example.com"],
        config: { entityId: "https://example.com" },
      });

      expect(result).toEqual(mockConnector);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/sso/connectors",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("updateConnector", () => {
    it("sends PUT /api/v1/tenants/{id}/sso/connectors/{connectorId}", async () => {
      const updated = { ...mockConnector, name: "Updated SAML" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.sso.updateConnector("tenant-1", "conn-1", {
        name: "Updated SAML",
      });

      expect(result.name).toBe("Updated SAML");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/sso/connectors/conn-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("deleteConnector", () => {
    it("sends DELETE /api/v1/tenants/{id}/sso/connectors/{connectorId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.sso.deleteConnector("tenant-1", "conn-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/sso/connectors/conn-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("testConnector", () => {
    it("sends POST /api/v1/tenants/{id}/sso/connectors/{connectorId}/test", async () => {
      const mockResult: SSOTestResult = {
        success: true,
        message: "Connection successful",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockResult }),
      });

      const result = await client.sso.testConnector("tenant-1", "conn-1");

      expect(result).toEqual(mockResult);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/sso/connectors/conn-1/test",
        expect.objectContaining({ method: "POST" })
      );
    });
  });
});
