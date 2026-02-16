import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// Mock react-router before importing session.server
vi.mock("react-router", () => ({
  createCookie: vi.fn(() => ({
    parse: vi.fn(),
    serialize: vi.fn(),
  })),
  redirect: vi.fn((url: string, init?: ResponseInit) => {
    const response = new Response(null, {
      status: 302,
      headers: { Location: url, ...init?.headers },
    });
    return response;
  }),
}));

// We need to import after mocking
import { redirect } from "react-router";
import {
  getSession,
  commitSession,
  destroySession,
  getAccessToken,
  requireAuth,
  sessionCookie,
  type SessionData,
} from "~/services/session.server";

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe("session.server", () => {
  const mockParse = vi.fn();
  const mockSerialize = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockClear();

    // Setup cookie mock methods
    (sessionCookie.parse as ReturnType<typeof vi.fn>) = mockParse;
    (sessionCookie.serialize as ReturnType<typeof vi.fn>) = mockSerialize;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ============================================================================
  // getSession
  // ============================================================================

  describe("getSession", () => {
    it("returns session data when cookie exists", async () => {
      const sessionData: SessionData = {
        accessToken: "test-token",
        refreshToken: "refresh-token",
        expiresAt: Date.now() + 3600000,
      };
      mockParse.mockResolvedValue(sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc123" },
      });

      const result = await getSession(request);
      expect(result).toEqual(sessionData);
      expect(mockParse).toHaveBeenCalled();
    });

    it("returns null when no cookie exists", async () => {
      mockParse.mockResolvedValue(null);

      const request = new Request("http://localhost");
      const result = await getSession(request);
      expect(result).toBeNull();
    });

    it("returns null when cookie is empty/falsy", async () => {
      mockParse.mockResolvedValue("");

      const request = new Request("http://localhost");
      const result = await getSession(request);
      expect(result).toBeNull();
    });
  });

  // ============================================================================
  // commitSession
  // ============================================================================

  describe("commitSession", () => {
    it("serializes session data to cookie", async () => {
      const sessionData: SessionData = {
        accessToken: "test-token",
        refreshToken: "refresh-token",
      };
      mockSerialize.mockResolvedValue("serialized-cookie");

      const result = await commitSession(sessionData);
      expect(result).toBe("serialized-cookie");
      expect(mockSerialize).toHaveBeenCalledWith(sessionData);
    });
  });

  // ============================================================================
  // destroySession
  // ============================================================================

  describe("destroySession", () => {
    it("serializes empty string with maxAge 0", async () => {
      const sessionData: SessionData = { accessToken: "test-token" };
      mockSerialize.mockResolvedValue("destroyed-cookie");

      const result = await destroySession(sessionData);
      expect(result).toBe("destroyed-cookie");
      expect(mockSerialize).toHaveBeenCalledWith("", { maxAge: 0 });
    });
  });

  // ============================================================================
  // getAccessToken
  // ============================================================================

  describe("getAccessToken", () => {
    it("returns access token when session is valid and not expired", async () => {
      const sessionData: SessionData = {
        accessToken: "valid-token",
        expiresAt: Date.now() + 3600000, // 1 hour from now
      };
      mockParse.mockResolvedValue(sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBe("valid-token");
    });

    it("returns null when no session exists", async () => {
      mockParse.mockResolvedValue(null);

      const request = new Request("http://localhost");
      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when session has no accessToken", async () => {
      mockParse.mockResolvedValue({ accessToken: "" });

      const request = new Request("http://localhost");
      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("refreshes token when expired and returns new token", async () => {
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "valid-refresh",
        expiresAt: Date.now() - 1000, // Expired
      };
      mockParse.mockResolvedValue(sessionData);

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          access_token: "new-token",
          refresh_token: "new-refresh",
          id_token: "new-id",
          expires_in: 3600,
        }),
      });

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBe("new-token");
    });

    it("returns null when token is expired and no refresh token", async () => {
      const sessionData: SessionData = {
        accessToken: "expired-token",
        expiresAt: Date.now() - 1000, // Expired, no refreshToken
      };
      mockParse.mockResolvedValue(sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when token is expired and refresh fails", async () => {
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "bad-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue(sessionData);

      mockFetch.mockResolvedValue({ ok: false, status: 401 });

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when token has no expiresAt (treated as expired)", async () => {
      const sessionData: SessionData = {
        accessToken: "token-no-expiry",
        // No expiresAt - isTokenExpired returns true
      };
      mockParse.mockResolvedValue(sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when refresh throws an exception", async () => {
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "valid-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue(sessionData);

      mockFetch.mockRejectedValue(new Error("Network error"));

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });
  });

  // ============================================================================
  // requireAuth
  // ============================================================================

  describe("requireAuth", () => {
    it("returns session when valid and not expired", async () => {
      const sessionData: SessionData = {
        accessToken: "valid-token",
        expiresAt: Date.now() + 3600000,
      };
      mockParse.mockResolvedValue(sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await requireAuth(request);
      expect(result).toEqual(sessionData);
    });

    it("throws redirect when no session", async () => {
      mockParse.mockResolvedValue(null);

      const request = new Request("http://localhost");

      await expect(requireAuth(request)).rejects.toBeDefined();
      expect(redirect).toHaveBeenCalledWith(
        "/login",
        expect.objectContaining({
          headers: expect.objectContaining({
            "Cache-Control": "no-store, no-cache, must-revalidate, private",
            Pragma: "no-cache",
            Expires: "0",
          }),
        })
      );
    });

    it("throws redirect when session has no accessToken", async () => {
      mockParse.mockResolvedValue({ accessToken: "" });

      const request = new Request("http://localhost");

      await expect(requireAuth(request)).rejects.toBeDefined();
      expect(redirect).toHaveBeenCalledWith(
        "/login",
        expect.objectContaining({
          headers: expect.objectContaining({
            "Cache-Control": "no-store, no-cache, must-revalidate, private",
            Pragma: "no-cache",
            Expires: "0",
          }),
        })
      );
    });

    it("returns refreshed session when expired and refresh succeeds", async () => {
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "valid-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue(sessionData);

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          access_token: "new-token",
          refresh_token: "new-refresh",
          id_token: "new-id",
          expires_in: 3600,
        }),
      });

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await requireAuth(request);
      expect(result.accessToken).toBe("new-token");
    });

    it("throws redirect when expired and refresh fails", async () => {
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "bad-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue(sessionData);

      mockFetch.mockResolvedValue({ ok: false, status: 401 });

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      await expect(requireAuth(request)).rejects.toBeDefined();
      expect(redirect).toHaveBeenCalledWith(
        "/login",
        expect.objectContaining({
          headers: expect.objectContaining({
            "Cache-Control": "no-store, no-cache, must-revalidate, private",
            Pragma: "no-cache",
            Expires: "0",
          }),
        })
      );
    });
  });
});
