import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { Invitation, InvitationValidation } from "../../types/invitation.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("InvitationsClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockInvitation: Invitation = {
    id: "inv-1",
    tenantId: "tenant-1",
    email: "invite@example.com",
    roleIds: ["role-1"],
    invitedBy: "user-1",
    status: "pending",
    expiresAt: "2026-02-01T00:00:00Z",
    createdAt: "2026-01-01T00:00:00Z",
  };

  describe("list", () => {
    it("sends GET /api/v1/tenants/{tenantId}/invitations", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockInvitation] }),
      });

      const result = await client.invitations.list("tenant-1");

      expect(result).toEqual([mockInvitation]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/invitations",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("get", () => {
    it("sends GET /api/v1/invitations/{id}", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockInvitation }),
      });

      const result = await client.invitations.get("inv-1");

      expect(result).toEqual(mockInvitation);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/invitations/inv-1",
        expect.objectContaining({ method: "GET" })
      );
    });

    it("throws NotFoundError on 404", async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 404,
        json: () => Promise.resolve({ code: "not_found", message: "Invitation not found" }),
      });

      await expect(client.invitations.get("missing")).rejects.toThrow("Invitation not found");
    });
  });

  describe("create", () => {
    it("sends POST /api/v1/tenants/{tenantId}/invitations", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockInvitation }),
      });

      const result = await client.invitations.create("tenant-1", {
        email: "invite@example.com",
        roleIds: ["role-1"],
      });

      expect(result).toEqual(mockInvitation);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/tenants/tenant-1/invitations",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("delete", () => {
    it("sends DELETE /api/v1/invitations/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.invitations.delete("inv-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/invitations/inv-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("revoke", () => {
    it("sends POST /api/v1/invitations/{id}/revoke", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.invitations.revoke("inv-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/invitations/inv-1/revoke",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("resend", () => {
    it("sends POST /api/v1/invitations/{id}/resend", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.invitations.resend("inv-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/invitations/inv-1/resend",
        expect.objectContaining({ method: "POST" })
      );
    });
  });

  describe("validate", () => {
    it("sends GET /api/v1/invitations/validate with token param", async () => {
      const mockValidation: InvitationValidation = {
        valid: true,
        invitation: mockInvitation,
        tenantName: "Test Tenant",
      };

      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: mockValidation }),
      });

      const result = await client.invitations.validate("abc123token");

      expect(result).toEqual(mockValidation);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/invitations/validate?token=abc123token",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("accept", () => {
    it("sends POST /api/v1/invitations/accept", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.invitations.accept({
        token: "abc123token",
        email: "newuser@example.com",
        displayName: "New User",
        password: "SecurePass123!", // pragma: allowlist secret
      });

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/invitations/accept",
        expect.objectContaining({ method: "POST" })
      );
    });
  });
});
