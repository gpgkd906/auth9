import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// In-memory Redis mock store
const redisStore = new Map<string, string>();
const mockRedisGet = vi.fn(async (key: string) => redisStore.get(key) ?? null);
const mockRedisSet = vi.fn(async (key: string, value: string) => {
  redisStore.set(key, value);
  return "OK";
});
const mockRedisDel = vi.fn(async (key: string) => {
  redisStore.delete(key);
  return 1;
});

// Mock Redis client before importing session.server
vi.mock("~/services/redis.server", () => ({
  getRedis: () => ({
    get: mockRedisGet,
    set: mockRedisSet,
    del: mockRedisDel,
  }),
}));

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

// Helper: store session data in Redis and return a cookie-shaped object
function seedSession(sid: string, data: SessionData) {
  const toStore = Object.fromEntries(
    Object.entries(data).filter(([k]) => k !== "_sid"),
  );
  redisStore.set(`portal:session:${sid}`, JSON.stringify(toStore));
}

describe("session.server", () => {
  const mockParse = vi.fn();
  const mockSerialize = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockClear();
    redisStore.clear();

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
      // Cookie now returns { sid }, data lives in Redis
      const sid = "test-session-id";
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc123" },
      });

      const result = await getSession(request);
      expect(result).toMatchObject(sessionData);
      expect(result?.identityAccessToken).toBe("test-token");
      expect(result?._sid).toBe(sid);
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

    it("returns null when cookie has no sid", async () => {
      mockParse.mockResolvedValue({});

      const request = new Request("http://localhost");
      const result = await getSession(request);
      expect(result).toBeNull();
    });

    it("returns null when Redis has no matching session", async () => {
      mockParse.mockResolvedValue({ sid: "nonexistent" });

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });
      const result = await getSession(request);
      expect(result).toBeNull();
    });

    it("returns null when Redis throws", async () => {
      mockParse.mockResolvedValue({ sid: "test-sid" });
      mockRedisGet.mockRejectedValueOnce(new Error("Redis down"));

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });
      const result = await getSession(request);
      expect(result).toBeNull();
    });
  });

  // ============================================================================
  // commitSession
  // ============================================================================

  describe("commitSession", () => {
    it("stores session data in Redis and returns serialized cookie", async () => {
      const sessionData: SessionData = {
        accessToken: "test-token",
        refreshToken: "refresh-token",
      };
      mockSerialize.mockResolvedValue("serialized-cookie");

      const result = await commitSession(sessionData);
      expect(result).toBe("serialized-cookie");
      // Should have called Redis SET
      expect(mockRedisSet).toHaveBeenCalledTimes(1);
      const [key, value] = mockRedisSet.mock.calls[0];
      expect(key).toMatch(/^portal:session:/);
      const stored = JSON.parse(value);
      // Should contain identityAccessToken (normalized) and refreshToken
      expect(stored.identityAccessToken).toBe("test-token");
      expect(stored.refreshToken).toBe("refresh-token");
      // Should NOT contain accessToken or expiresAt (compact aliases)
      expect(stored.accessToken).toBeUndefined();
      expect(stored.expiresAt).toBeUndefined();
      // Should NOT contain _sid
      expect(stored._sid).toBeUndefined();
      // Cookie should be serialized with { sid }
      expect(mockSerialize).toHaveBeenCalledWith(
        expect.objectContaining({ sid: expect.any(String) })
      );
    });

    it("reuses existing _sid when present", async () => {
      const sessionData: SessionData = {
        _sid: "existing-session-id",
        accessToken: "test-token",
      };
      mockSerialize.mockResolvedValue("serialized-cookie");

      await commitSession(sessionData);
      const [key] = mockRedisSet.mock.calls[0];
      expect(key).toBe("portal:session:existing-session-id");
    });

    it("generates new sid when _sid is absent", async () => {
      const sessionData: SessionData = {
        accessToken: "test-token",
      };
      mockSerialize.mockResolvedValue("serialized-cookie");

      await commitSession(sessionData);
      const [key] = mockRedisSet.mock.calls[0];
      expect(key).toMatch(/^portal:session:[0-9a-f-]{36}$/);
    });
  });

  // ============================================================================
  // destroySession
  // ============================================================================

  describe("destroySession", () => {
    it("deletes Redis key and clears cookie", async () => {
      const sid = "test-session-id";
      seedSession(sid, { accessToken: "test-token" });

      const sessionData: SessionData = { _sid: sid, accessToken: "test-token" };
      mockSerialize.mockResolvedValue("destroyed-cookie");

      const result = await destroySession(sessionData);
      expect(result).toBe("destroyed-cookie");
      expect(mockRedisDel).toHaveBeenCalledWith(`portal:session:${sid}`);
      expect(mockSerialize).toHaveBeenCalledWith("", { maxAge: 0 });
    });

    it("clears cookie even when no _sid present", async () => {
      const sessionData: SessionData = { accessToken: "test-token" };
      mockSerialize.mockResolvedValue("destroyed-cookie");

      const result = await destroySession(sessionData);
      expect(result).toBe("destroyed-cookie");
      expect(mockRedisDel).not.toHaveBeenCalled();
      expect(mockSerialize).toHaveBeenCalledWith("", { maxAge: 0 });
    });
  });

  // ============================================================================
  // getAccessToken
  // ============================================================================

  describe("getAccessToken", () => {
    it("returns access token when session is valid and not expired", async () => {
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "valid-token",
        expiresAt: Date.now() + 3600000, // 1 hour from now
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

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
      const sid = "test-sid";
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, { accessToken: "" });

      const request = new Request("http://localhost");
      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("refreshes token when expired and returns new token", async () => {
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "valid-refresh",
        expiresAt: Date.now() - 1000, // Expired
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

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
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "expired-token",
        expiresAt: Date.now() - 1000, // Expired, no refreshToken
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when token is expired and refresh fails", async () => {
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "bad-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

      mockFetch.mockResolvedValue({ ok: false, status: 401 });

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when token has no expiresAt (treated as expired)", async () => {
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "token-no-expiry",
        // No expiresAt - isTokenExpired returns true
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await getAccessToken(request);
      expect(result).toBeNull();
    });

    it("returns null when refresh throws an exception", async () => {
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "valid-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

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
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "valid-token",
        expiresAt: Date.now() + 3600000,
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await requireAuth(request);
      expect(result).toMatchObject(sessionData);
      expect(result.identityAccessToken).toBe("valid-token");
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
      const sid = "test-sid";
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, { accessToken: "" });

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
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "valid-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          access_token: "new-token",
          refresh_token: "new-refresh",
          id_token: "new-id",
          expires_in: 3600,
        }),
      });
      mockSerialize.mockResolvedValue("updated-cookie");

      const request = new Request("http://localhost", {
        headers: { Cookie: "auth9_session=abc" },
      });

      const result = await requireAuth(request);
      expect(result.accessToken).toBe("new-token");
    });

    it("throws redirect when expired and refresh fails", async () => {
      const sid = "test-sid";
      const sessionData: SessionData = {
        accessToken: "expired-token",
        refreshToken: "bad-refresh",
        expiresAt: Date.now() - 1000,
      };
      mockParse.mockResolvedValue({ sid });
      seedSession(sid, sessionData);

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
