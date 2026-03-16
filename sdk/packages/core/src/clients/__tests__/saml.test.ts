import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { SamlApplication, SamlCertificateInfo } from "../../types/saml.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("SamlClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockApp: SamlApplication = {
    id: "saml-1",
    tenantId: "tenant-1",
    name: "Salesforce",
    entityId: "https://salesforce.com/sp",
    acsUrl: "https://salesforce.com/acs",
    enabled: true,
    config: {},
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/tenants/{id}/saml-apps", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockApp] }),
      });

      const result = await client.saml.list("tenant-1");

      expect(result).toEqual([mockApp]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/tenants/{id}/saml-apps/{appId}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockApp }),
      });

      const result = await client.saml.get("tenant-1", "saml-1");

      expect(result).toEqual(mockApp);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps/saml-1",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/tenants/{id}/saml-apps", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockApp }),
      });

      const result = await client.saml.create("tenant-1", {
        name: "Salesforce",
        entityId: "https://salesforce.com/sp",
        acsUrl: "https://salesforce.com/acs",
      });

      expect(result).toEqual(mockApp);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("update", () => {
    it("sends PUT /api/v1/tenants/{id}/saml-apps/{appId}", async () => {
      const updated = { ...mockApp, name: "Salesforce SSO" };
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: updated }),
      });

      const result = await client.saml.update("tenant-1", "saml-1", {
        name: "Salesforce SSO",
      });

      expect(result.name).toBe("Salesforce SSO");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps/saml-1",
        expect.objectContaining({ method: "PUT" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/tenants/{id}/saml-apps/{appId}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.saml.delete("tenant-1", "saml-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps/saml-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("getMetadata", () => {
    it("sends GET /api/v1/tenants/{id}/saml-apps/{appId}/metadata", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: "<xml>metadata</xml>" }),
      });

      const result = await client.saml.getMetadata("tenant-1", "saml-1");

      expect(result).toBe("<xml>metadata</xml>");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps/saml-1/metadata",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("getCertificate", () => {
    it("sends GET /api/v1/tenants/{id}/saml-apps/{appId}/certificate", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: "-----BEGIN CERTIFICATE-----\n..." }),
      });

      const result = await client.saml.getCertificate("tenant-1", "saml-1");

      expect(result).toBe("-----BEGIN CERTIFICATE-----\n...");
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps/saml-1/certificate",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("getCertificateInfo", () => {
    it("sends GET /api/v1/tenants/{id}/saml-apps/{appId}/certificate-info", async () => {
      const mockCertInfo: SamlCertificateInfo = {
        subject: "CN=auth9",
        issuer: "CN=auth9",
        validFrom: "2026-01-01T00:00:00Z",
        validTo: "2027-01-01T00:00:00Z",
        fingerprint: "AB:CD:EF",
        serialNumber: "123456",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockCertInfo }),
      });

      const result = await client.saml.getCertificateInfo("tenant-1", "saml-1");

      expect(result).toEqual(mockCertInfo);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/saml-apps/saml-1/certificate-info",
        expect.objectContaining({ method: "GET" })
      );
    });
  });
});
