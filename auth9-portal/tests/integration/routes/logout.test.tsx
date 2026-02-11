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

  it("loader redirects to auth9 logout endpoint", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(null);

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    const location = response.headers.get("Location");
    expect(location).toContain("/api/v1/auth/logout");
    expect(location).toContain("post_logout_redirect_uri=");
  });

  it("loader includes portal URL in redirect", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(null);

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    const location = response.headers.get("Location");
    expect(location).toContain("post_logout_redirect_uri=http");
  });

  it("calls backend logout API when access token exists", async () => {
    vi.mocked(getAccessToken).mockResolvedValue("valid-token");
    vi.mocked(getSession).mockResolvedValue({
      accessToken: "valid-token",
      refreshToken: "refresh",
    });
    mockFetch.mockResolvedValue({ ok: true, status: 302 });

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining("/api/v1/auth/logout"),
      expect.objectContaining({
        method: "GET",
        headers: expect.objectContaining({
          Authorization: "Bearer valid-token",
        }),
        redirect: "manual",
      })
    );
    expect(response.status).toBe(302);
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
    vi.mocked(getSession).mockResolvedValue(null);
    mockFetch.mockRejectedValue(new Error("Network error"));

    const request = new Request("http://localhost:3000/logout");
    // Should not throw, should still redirect
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    expect(response.headers.get("Location")).toContain("/api/v1/auth/logout");
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

  it("logs error when backend returns non-redirect status", async () => {
    vi.mocked(getAccessToken).mockResolvedValue("valid-token");
    vi.mocked(getSession).mockResolvedValue(null);
    mockFetch.mockResolvedValue({ ok: false, status: 500 });
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    expect(consoleSpy).toHaveBeenCalledWith(
      "[logout] Backend logout API returned non-redirect status:",
      500
    );
    consoleSpy.mockRestore();
  });

  // ============================================================================
  // id_token_hint Tests
  // ============================================================================

  it("includes id_token_hint in redirect URL when session has idToken", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue({
      accessToken: "token",
      idToken: "my-id-token-value",
    });
    vi.mocked(destroySession).mockResolvedValue("destroyed-cookie");

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    const location = response.headers.get("Location");
    expect(location).toContain("id_token_hint=my-id-token-value");
  });

  it("does not include id_token_hint when session has no idToken", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue({
      accessToken: "token",
    });
    vi.mocked(destroySession).mockResolvedValue("destroyed-cookie");

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    const location = response.headers.get("Location");
    expect(location).not.toContain("id_token_hint");
  });

  it("does not include id_token_hint when session is null", async () => {
    vi.mocked(getAccessToken).mockResolvedValue(null);
    vi.mocked(getSession).mockResolvedValue(null);

    const request = new Request("http://localhost:3000/logout");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.status).toBe(302);
    const location = response.headers.get("Location");
    expect(location).not.toContain("id_token_hint");
  });
});
