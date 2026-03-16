import { describe, it, expect, vi, beforeEach } from "vitest";
import { Auth9Client } from "../../auth9-client.js";
import type { SessionInfo } from "../../types/session.js";

const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

beforeEach(() => {
  vi.clearAllMocks();
});

describe("SessionsClient", () => {
  const client = new Auth9Client({
    baseUrl: "https://auth9.example.com",
    apiKey: "test-token", // pragma: allowlist secret
  });

  const mockSession: SessionInfo = {
    id: "sess-1",
    deviceType: "desktop",
    deviceName: "Chrome on macOS",
    ipAddress: "192.168.1.1",
    location: "San Francisco, US",
    lastActiveAt: "2026-01-01T12:00:00Z",
    createdAt: "2026-01-01T00:00:00Z",
    isCurrent: true,
  };

  describe("listMy", () => {
    it("sends GET /api/v1/users/me/sessions", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        status: 200,
        json: () => Promise.resolve({ data: [mockSession] }),
      });

      const result = await client.sessions.listMy();

      expect(result).toEqual([mockSession]);
      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/sessions",
        expect.objectContaining({ method: "GET" })
      );
    });
  });

  describe("revoke", () => {
    it("sends DELETE /api/v1/users/me/sessions/{id}", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.sessions.revoke("sess-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/sessions/sess-1",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("revokeAllOther", () => {
    it("sends DELETE /api/v1/users/me/sessions", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.sessions.revokeAllOther();

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/users/me/sessions",
        expect.objectContaining({ method: "DELETE" })
      );
    });
  });

  describe("forceLogout", () => {
    it("sends POST /api/v1/admin/users/{id}/logout", async () => {
      mockFetch.mockResolvedValue({ ok: true, status: 204 });

      await client.sessions.forceLogout("user-1");

      expect(mockFetch).toHaveBeenCalledWith(
        "https://auth9.example.com/api/v1/admin/users/user-1/logout",
        expect.objectContaining({ method: "POST" })
      );
    });
  });
});
