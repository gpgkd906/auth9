import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type {
  Service,
  Client,
  ClientWithSecret,
  ServiceIntegration,
} from "../../types/service.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ServicesClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockService: Service = {
    id: "svc-1",
    name: "Test Service",
    redirectUris: ["https://example.com/callback"],
    logoutUris: [],
    status: "active",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/services", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockService] }),
      });

      const result = await client.services.list();

      expect(result).toEqual([mockService]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/services/{id}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockService }),
      });

      const result = await client.services.get("svc-1");

      expect(result).toEqual(mockService);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/services", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockService }),
      });

      const result = await client.services.create({
        name: "Test Service",
        redirectUris: ["https://example.com/callback"],
      });

      expect(result).toEqual(mockService);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/services/{id}", async () => {
      const updated = { ...mockService, name: "Updated" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.services.update("svc-1", { name: "Updated" });

      expect(result.name).toBe("Updated");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/services/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.services.delete("svc-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("getIntegrationInfo", () => {
    it("sends GET /api/v1/services/{id}/integration", async () => {
      const mockIntegration: ServiceIntegration = {
        serviceId: "svc-1",
        clientId: "client-id",
        issuerUrl: "https://auth.example.com/realms/test",
        authorizationEndpoint: "https://auth.example.com/realms/test/protocol/openid-connect/auth",
        tokenEndpoint: "https://auth.example.com/realms/test/protocol/openid-connect/token",
        userinfoEndpoint: "https://auth.example.com/realms/test/protocol/openid-connect/userinfo",
        jwksUri: "https://auth.example.com/realms/test/protocol/openid-connect/certs",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockIntegration }),
      });

      const result = await client.services.getIntegrationInfo("svc-1");

      expect(result).toEqual(mockIntegration);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/integration",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("listClients", () => {
    it("sends GET /api/v1/services/{id}/clients", async () => {
      const mockClient: Client = {
        id: "client-1",
        serviceId: "svc-1",
        clientId: "my-client",
        createdAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockClient] }),
      });

      const result = await client.services.listClients("svc-1");

      expect(result).toEqual([mockClient]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/clients",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("createClient", () => {
    it("sends POST /api/v1/services/{id}/clients", async () => {
      const mockClientWithSecret: ClientWithSecret = {
        id: "client-1",
        serviceId: "svc-1",
        clientId: "my-client",
        clientSecret: "secret-value", // pragma: allowlist secret
        createdAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockClientWithSecret }),
      });

      const result = await client.services.createClient("svc-1", { name: "My Client" });

      expect(result.clientSecret).toBe("secret-value");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/clients",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("deleteClient", () => {
    it("sends DELETE /api/v1/services/{serviceId}/clients/{clientId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.services.deleteClient("svc-1", "client-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/clients/client-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("regenerateClientSecret", () => {
    it("sends POST /api/v1/services/{serviceId}/clients/{clientId}/regenerate-secret", async () => {
      const mockClientWithSecret: ClientWithSecret = {
        id: "client-1",
        serviceId: "svc-1",
        clientId: "my-client",
        clientSecret: "new-secret", // pragma: allowlist secret
        createdAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockClientWithSecret }),
      });

      const result = await client.services.regenerateClientSecret("svc-1", "client-1");

      expect(result.clientSecret).toBe("new-secret");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/services/svc-1/clients/client-1/regenerate-secret",
        expect.objectContaining({ method: "POST" })
      );
    });
  });
});
