import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type {
  IdentityProvider,
  IdentityProviderTemplate,
  LinkedIdentity,
} from "../../types/identity-provider.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("IdentityProvidersClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockProvider: IdentityProvider = {
    alias: "google",
    displayName: "Google",
    providerId: "google",
    enabled: true,
    config: { clientId: "123" },
    createdAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/identity-providers", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockProvider] }),
      });

      const result = await client.identityProviders.list();

      expect(result).toEqual([mockProvider]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/identity-providers",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/identity-providers/{alias}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockProvider }),
      });

      const result = await client.identityProviders.get("google");

      expect(result).toEqual(mockProvider);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/identity-providers/google",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/identity-providers", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockProvider }),
      });

      const result = await client.identityProviders.create({
        alias: "google",
        displayName: "Google",
        providerId: "google",
        config: { clientId: "123" },
      });

      expect(result).toEqual(mockProvider);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/identity-providers",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/identity-providers/{alias}", async () => {
      const updated = { ...mockProvider, displayName: "Google SSO" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.identityProviders.update("google", {
        displayName: "Google SSO",
      });

      expect(result.displayName).toBe("Google SSO");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/identity-providers/google",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/identity-providers/{alias}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.identityProviders.delete("google");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/identity-providers/google",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("getTemplates", () => {
    it("sends GET /api/v1/identity-providers/templates", async () => {
      const mockTemplate: IdentityProviderTemplate = {
        id: "google",
        name: "Google",
        providerId: "google",
        config: { clientId: "", clientSecret: "" },
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockTemplate] }),
      });

      const result = await client.identityProviders.getTemplates();

      expect(result).toEqual([mockTemplate]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/identity-providers/templates",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("listMyLinkedIdentities", () => {
    it("sends GET /api/v1/users/me/linked-identities", async () => {
      const mockIdentity: LinkedIdentity = {
        id: "li-1",
        provider: "google",
        providerUserId: "goog-123",
        email: "user@example.com",
        linkedAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockIdentity] }),
      });

      const result = await client.identityProviders.listMyLinkedIdentities();

      expect(result).toEqual([mockIdentity]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/linked-identities",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("unlinkIdentity", () => {
    it("sends DELETE /api/v1/users/me/linked-identities/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.identityProviders.unlinkIdentity("li-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/linked-identities/li-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });
});
