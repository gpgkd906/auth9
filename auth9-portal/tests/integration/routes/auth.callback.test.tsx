import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { loader } from "~/routes/auth.callback";

// Mock session.server
vi.mock("~/services/session.server", () => ({
  commitSession: vi.fn().mockResolvedValue("mocked-cookie"),
    requireAuthWithUpdate: vi.fn().mockResolvedValue({
        session: {
            accessToken: "test-token",
            refreshToken: "test-refresh-token",
            idToken: "test-id-token",
            expiresAt: Date.now() + 3600000,
        },
        headers: undefined,
    }),
}));

import { commitSession } from "~/services/session.server";

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

describe("Auth Callback", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe("loader", () => {
    it("redirects to /login with error when error param is present", async () => {
      const request = new Request(
        "http://localhost/auth/callback?error=access_denied&error_description=User+denied"
      );

      const response = await loader({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe(
        "/login?error=access_denied"
      );
    });

    it("redirects to /login when access_token query is present without code", async () => {
      const request = new Request(
        "http://localhost/auth/callback?access_token=my-token&expires_in=3600"
      );

      const response = await loader({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe("/login");
      expect(commitSession).not.toHaveBeenCalled();
    });

    it("redirects to /login when only access_token is present", async () => {
      const request = new Request(
        "http://localhost/auth/callback?access_token=my-token"
      );

      const response = await loader({ request, params: {}, context: {} });
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe("/login");
    });

    it("redirects to /login when no code or access_token", async () => {
      const request = new Request("http://localhost/auth/callback");

      const response = await loader({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe("/login");
    });

    it("exchanges auth code successfully and redirects to dashboard", async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({
          access_token: "new-access-token",
          refresh_token: "new-refresh-token",
          id_token: "new-id-token",
          expires_in: 3600,
        }),
      });

      const request = new Request(
        "http://localhost/auth/callback?code=auth-code-123"
      );

      const response = await loader({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe("/dashboard");
      expect(commitSession).toHaveBeenCalledWith(
        expect.objectContaining({ accessToken: "new-access-token" })
      );

      // Verify fetch was called with correct params
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining("/api/v1/auth/token"),
        expect.objectContaining({
          method: "POST",
          body: expect.stringContaining("authorization_code"),
        })
      );
    });

    it("redirects to /login on token exchange failure", async () => {
      mockFetch.mockResolvedValue({
        ok: false,
        status: 400,
        text: async () => "Bad Request",
      });

      const request = new Request(
        "http://localhost/auth/callback?code=bad-code"
      );

      const response = await loader({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe(
        "/login?error=token_exchange_failed"
      );
    });

    it("redirects to /login on fetch exception", async () => {
      mockFetch.mockRejectedValue(new Error("Network error"));

      const request = new Request(
        "http://localhost/auth/callback?code=auth-code"
      );

      const response = await loader({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe(
        "/login?error=callback_exception"
      );
    });
  });
});
