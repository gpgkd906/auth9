import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type {
  ScimToken,
  ScimTokenWithValue,
  ScimLog,
  ScimGroupMapping,
} from "../../types/scim.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("ScimClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const basePath =
    "https://auth9.example.com/api/v1/tenants/tenant-1/sso/connectors/conn-1/scim";

  describe("listTokens", () => {
    it("sends GET .../scim/tokens", async () => {
      const mockToken: ScimToken = {
        id: "token-1",
        connectorId: "conn-1",
        name: "Provisioning Token",
        createdAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockToken] }),
      });

      const result = await client.scim.listTokens("tenant-1", "conn-1");

      expect(result).toEqual([mockToken]);
      expect(mockFetch).toHaveBeenCalledWith(
        `${basePath}/tokens`,
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("createToken", () => {
    it("sends POST .../scim/tokens", async () => {
      const mockTokenWithValue: ScimTokenWithValue = {
        id: "token-1",
        connectorId: "conn-1",
        name: "Provisioning Token",
        token: "scim-bearer-token-value", // pragma: allowlist secret
        createdAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockTokenWithValue }),
      });

      const result = await client.scim.createToken("tenant-1", "conn-1", {
        name: "Provisioning Token",
      });

      expect(result).toEqual(mockTokenWithValue);
      expect(mockFetch).toHaveBeenCalledWith(
        `${basePath}/tokens`,
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("revokeToken", () => {
    it("sends DELETE .../scim/tokens/{tokenId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.scim.revokeToken("tenant-1", "conn-1", "token-1");

      expect(mockFetch).toHaveBeenCalledWith(
        `${basePath}/tokens/token-1`,
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("listLogs", () => {
    it("sends GET .../scim/logs", async () => {
      const mockLog: ScimLog = {
        id: "log-1",
        connectorId: "conn-1",
        operation: "create",
        resourceType: "User",
        resourceId: "user-1",
        status: "success",
        createdAt: "2026-01-01T00:00:00Z",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockLog] }),
      });

      const result = await client.scim.listLogs("tenant-1", "conn-1");

      expect(result).toEqual([mockLog]);
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining(`${basePath}/logs`),
        expect.objectContaining({ method: "GET" })
      );
    });

    it("passes query params when provided", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [] }),
      });

      await client.scim.listLogs("tenant-1", "conn-1", {
        operation: "create",
        status: "error",
        limit: 10,
      });

      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining(`${basePath}/logs`),
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("listGroupMappings", () => {
    it("sends GET .../scim/group-mappings", async () => {
      const mockMapping: ScimGroupMapping = {
        id: "map-1",
        connectorId: "conn-1",
        scimGroupId: "scim-grp-1",
        scimGroupName: "Engineering",
        roleId: "role-1",
        roleName: "engineer",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockMapping] }),
      });

      const result = await client.scim.listGroupMappings("tenant-1", "conn-1");

      expect(result).toEqual([mockMapping]);
      expect(mockFetch).toHaveBeenCalledWith(
        `${basePath}/group-mappings`,
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("updateGroupMappings", () => {
    it("sends PUT .../scim/group-mappings", async () => {
      const mockMapping: ScimGroupMapping = {
        id: "map-1",
        connectorId: "conn-1",
        scimGroupId: "scim-grp-1",
        scimGroupName: "Engineering",
        roleId: "role-2",
        roleName: "admin",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockMapping] }),
      });

      const result = await client.scim.updateGroupMappings(
        "tenant-1",
        "conn-1",
        [mockMapping]
      );

      expect(result).toEqual([mockMapping]);
      expect(mockFetch).toHaveBeenCalledWith(
        `${basePath}/group-mappings`,
        expect.objectContaining({ method: "PUT" })
      );
    });
  });
});
