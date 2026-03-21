import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import Logout, { loader } from "~/routes/logout";

// Mock fetch globally
const mockFetch = vi.fn();
global.fetch = mockFetch;

// Mock session.server
vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn(),
  getSession: vi.fn(),
  destroySession: vi.fn().mockResolvedValue("destroyed-cookie"),
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

import { getAccessToken, getSession, destroySession } from "~/services/session.server";

describe("Logout Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockClear();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader redirects to /login after logout", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(null);

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    const location = response.headers.get("Location");
    expect(location).toBe("/login");
  });

  it("calls hosted-login logout API when identity token exists", async () => {
    vi.mocked(getAccessToken).mockResolvedValue("valid-token");
    vi.mocked(getSession).mockResolvedValue({
      accessToken: "valid-token",
      identityAccessToken: "identity-token",
      refreshToken: "refresh",
    });
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ message: "Logged out successfully." }),
    });

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/hosted-login/logout"),
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          Authorization: "Bearer identity-token",
        }),
      })
    );
    expect(response.status).toBe(302);
    expect(response.headers.get("Location")).toBe("/login");
  });

  it("destroys session cookie when session exists", async () => {
    const mockSession = { accessToken: "token" };
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(mockSession);
    vi.mocked(destroySession).mockResolvedValue("destroyed-cookie");

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(destroySession).toHaveBeenCalledWith(mockSession);
    expect(response.status).toBe(302);
  });

  it("handles backend logout API error gracefully", async () => {
    vi.mocked(getAccessToken).mockResolvedValue("valid-token");
    vi.mocked(getSession).mockResolvedValue({
      identityAccessToken: "identity-token",
    });
    mockFetch.mockRejectedValue(new Error("Network error"));
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    const request = new Request("http://localhost:3000/logout");
    // Should not throw, should still redirect to /login
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    expect(response.headers.get("Location")).toBe("/login");
    consoleSpy.mockRestore();
  });

  it("does not call backend API when no access token", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(null);

    const request = new Request("http://localhost:3000/logout");
    await loader({ request, params: {}, context: {} });

    expect(mockFetch).not.toHaveBeenCalled();
  });

  it("Logout component renders null", () => {
    const { container } = render(<Logout />);
    // Component renders null, so there's minimal content
    expect(container.innerHTML).toBe("");
  });

  it("sets cache control headers to prevent caching", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(null);

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.headers.get("Cache-Control")).toBe(
      "no-store, no-cache, must-revalidate, private"
    );
    expect(response.headers.get("Pragma")).toBe("no-cache");
    expect(response.headers.get("Expires")).toBe("0");
  });

  it("falls back to accessToken when identityAccessToken is not set", async () => {
    vi.mocked(getAccessToken).mockResolvedValue("fallback-token");
    vi.mocked(getSession).mockResolvedValue({
      accessToken: "fallback-token",
    });
    mockFetch.mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ message: "Logged out successfully." }),
    });

    const request = new Request("http://localhost:3000/logout");
    await loader({ request, params: {}, context: {} });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/hosted-login/logout"),
      expect.objectContaining({
        headers: expect.objectContaining({
          Authorization: "Bearer fallback-token",
        }),
      })
    );
  });
});
