import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SessionsPage, { loader, action } from "~/routes/dashboard.settings.sessions";
import { sessionApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  sessionApi: {
    listMySessions: vi.fn(),
    revokeSession: vi.fn(),
    revokeOtherSessions: vi.fn(),
  },
}));

// Mock the session server
vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("mock-access-token"),
}));

import { getAccessToken } from "~/services/session.server";

const mockCurrentSession = {
  id: "session-1",
  device_type: "desktop",
  device_name: "Chrome on macOS",
  ip_address: "192.168.1.1",
  location: "San Francisco, US",
  last_active_at: new Date().toISOString(),
  created_at: new Date().toISOString(),
  is_current: true,
};

const mockOtherSession = {
  id: "session-2",
  device_type: "mobile",
  device_name: "Safari on iPhone",
  ip_address: "10.0.0.1",
  location: "New York, US",
  last_active_at: new Date(Date.now() - 3600000).toISOString(), // 1 hour ago
  created_at: new Date(Date.now() - 86400000).toISOString(), // 1 day ago
  is_current: false,
};

describe("Sessions Settings Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests (via component rendering with mock loader)
  // ============================================================================

  it("loader returns sessions from API", async () => {
    vi.mocked(sessionApi.listMySessions).mockResolvedValue({
      data: [mockCurrentSession, mockOtherSession],
    });

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: async () => {
          const response = await sessionApi.listMySessions("mock-token");
          return { sessions: response.data };
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Chrome on macOS")).toBeInTheDocument();
    });
  });

  it("loader returns empty sessions on API error", async () => {
    vi.mocked(sessionApi.listMySessions).mockRejectedValue(new Error("API Error"));

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: async () => {
          try {
            await sessionApi.listMySessions("mock-token");
            return { sessions: [] };
          } catch {
            return { sessions: [], error: "Failed to load sessions" };
          }
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Failed to load sessions")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders current session section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockCurrentSession, mockOtherSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Current Session")).toBeInTheDocument();
    });
    expect(screen.getByText("This is the device you are currently using.")).toBeInTheDocument();
    expect(screen.getByText("Chrome on macOS")).toBeInTheDocument();
    expect(screen.getByText("Current")).toBeInTheDocument();
  });

  it("renders other sessions section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockCurrentSession, mockOtherSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Other Sessions")).toBeInTheDocument();
    });
    expect(screen.getByText("Safari on iPhone")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /sign out all/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /revoke/i })).toBeInTheDocument();
  });

  it("renders empty state when no other sessions", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockCurrentSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("No other active sessions")).toBeInTheDocument();
    });
    // Sign out all button should not be present
    expect(screen.queryByRole("button", { name: /sign out all/i })).not.toBeInTheDocument();
  });

  it("renders security tips section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Security Tips")).toBeInTheDocument();
    });
    expect(screen.getByText(/sign out of sessions you do not recognize/i)).toBeInTheDocument();
    expect(screen.getByText(/enable two-factor authentication/i)).toBeInTheDocument();
  });

  it("displays session location and IP address", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockCurrentSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText(/192\.168\.1\.1/)).toBeInTheDocument();
    });
    expect(screen.getByText(/San Francisco, US/)).toBeInTheDocument();
  });

  // ============================================================================
  // Action Tests (via stub actions that simulate business logic)
  // ============================================================================

  it("action revokes specific session", async () => {
    const revokeSessionSpy = vi.mocked(sessionApi.revokeSession).mockResolvedValue(undefined);

    // Create a stub action that simulates the real action's behavior
    const stubAction = async ({ request }: { request: Request }) => {
      const formData = await request.formData();
      const intent = formData.get("intent");
      const sessionId = formData.get("sessionId") as string;

      if (intent === "revoke") {
        await sessionApi.revokeSession(sessionId, "mock-token");
        return { success: true, message: "Session revoked" };
      }
      return { error: "Invalid action" };
    };

    const formData = new FormData();
    formData.append("intent", "revoke");
    formData.append("sessionId", "session-2");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await stubAction({ request });

    expect(revokeSessionSpy).toHaveBeenCalledWith("session-2", "mock-token");
    expect(response).toEqual({ success: true, message: "Session revoked" });
  });

  it("action revokes all other sessions", async () => {
    const revokeOtherSessionsSpy = vi.mocked(sessionApi.revokeOtherSessions).mockResolvedValue(undefined);

    const stubAction = async ({ request }: { request: Request }) => {
      const formData = await request.formData();
      const intent = formData.get("intent");

      if (intent === "revoke_all") {
        await sessionApi.revokeOtherSessions("mock-token");
        return { success: true, message: "All other sessions revoked" };
      }
      return { error: "Invalid action" };
    };

    const formData = new FormData();
    formData.append("intent", "revoke_all");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await stubAction({ request });

    expect(revokeOtherSessionsSpy).toHaveBeenCalledWith("mock-token");
    expect(response).toEqual({ success: true, message: "All other sessions revoked" });
  });

  it("action returns error on API failure", async () => {
    vi.mocked(sessionApi.revokeSession).mockRejectedValue(new Error("Session not found"));

    const stubAction = async ({ request }: { request: Request }) => {
      const formData = await request.formData();
      const intent = formData.get("intent");
      const sessionId = formData.get("sessionId") as string;

      try {
        if (intent === "revoke") {
          await sessionApi.revokeSession(sessionId, "mock-token");
          return { success: true };
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : "Operation failed";
        return { error: message };
      }
      return { error: "Invalid action" };
    };

    const formData = new FormData();
    formData.append("intent", "revoke");
    formData.append("sessionId", "invalid-session");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await stubAction({ request });

    expect(response).toEqual({ error: "Session not found" });
  });

  it("action returns error for invalid intent", async () => {
    const stubAction = async ({ request }: { request: Request }) => {
      const formData = await request.formData();
      const intent = formData.get("intent");

      if (intent === "revoke" || intent === "revoke_all") {
        return { success: true };
      }
      return { error: "Invalid action" };
    };

    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await stubAction({ request });

    expect(response).toEqual({ error: "Invalid action" });
  });

  // ============================================================================
  // formatDate edge case tests
  // ============================================================================

  it("renders last active with days format for sessions older than 24h", async () => {
    const threeDaysAgo = new Date(Date.now() - 3 * 86400000).toISOString();
    const mockSessionDaysOld = {
      ...mockCurrentSession,
      last_active_at: threeDaysAgo,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockSessionDaysOld] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText(/3 days ago/)).toBeInTheDocument();
    });
  });

  it("renders last active with locale date for sessions older than 7 days", async () => {
    const twoWeeksAgo = new Date(Date.now() - 14 * 86400000);
    const mockSessionWeeksOld = {
      ...mockCurrentSession,
      last_active_at: twoWeeksAgo.toISOString(),
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockSessionWeeksOld] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      // Should use toLocaleDateString format for 7+ day old sessions
      const expectedDate = twoWeeksAgo.toLocaleDateString();
      expect(screen.getByText(new RegExp(expectedDate))).toBeInTheDocument();
    });
  });

  it("renders tablet device icon for tablet sessions", async () => {
    const tabletSession = {
      ...mockOtherSession,
      id: "session-tablet",
      device_type: "tablet",
      device_name: "Safari on iPad",
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockCurrentSession, tabletSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Safari on iPad")).toBeInTheDocument();
    });
  });

  it("displays action error message in other sessions section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [mockCurrentSession, mockOtherSession] }),
        action: () => ({ error: "Session revocation failed" }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Safari on iPhone")).toBeInTheDocument();
    });

    // Click revoke button to trigger the action
    const revokeButton = screen.getByRole("button", { name: /revoke/i });
    await user.click(revokeButton);

    await waitFor(() => {
      expect(screen.getByText("Session revocation failed")).toBeInTheDocument();
    });
  });

  it("displays 'Unknown Device' when device_name is empty", async () => {
    const unknownDeviceSession = {
      ...mockCurrentSession,
      device_name: undefined,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [unknownDeviceSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Unknown Device")).toBeInTheDocument();
    });
  });

  it("renders session without ip_address correctly", async () => {
    const noIpSession = {
      ...mockCurrentSession,
      ip_address: undefined,
      location: undefined,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [noIpSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Chrome on macOS")).toBeInTheDocument();
    });
    // IP address should not be shown
    expect(screen.queryByText(/192\.168/)).not.toBeInTheDocument();
  });

  it("shows 'Unable to identify current session' when no current session", async () => {
    const nonCurrentSession = {
      ...mockOtherSession,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [nonCurrentSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Unable to identify current session")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Direct loader/action Tests
  // ============================================================================

  describe("loader (direct)", () => {
    it("returns sessions from API", async () => {
      vi.mocked(sessionApi.listMySessions).mockResolvedValue({
        data: [mockCurrentSession],
      });

      const request = new Request("http://localhost/dashboard/settings/sessions");
      const result = await loader({ request, params: {}, context: {} });
      expect(result).toEqual({ sessions: [mockCurrentSession] });
    });

    it("returns empty sessions on API error", async () => {
      vi.mocked(sessionApi.listMySessions).mockRejectedValue(new Error("API Error"));

      const request = new Request("http://localhost/dashboard/settings/sessions");
      const result = await loader({ request, params: {}, context: {} });
      expect(result).toEqual({ sessions: [], error: "Failed to load sessions" });
    });

    it("redirects to login when not authenticated", async () => {
      vi.mocked(getAccessToken).mockResolvedValueOnce(null);

      const request = new Request("http://localhost/dashboard/settings/sessions");
      await expect(loader({ request, params: {}, context: {} })).rejects.toEqual(
        expect.objectContaining({ status: 302 })
      );
    });
  });

  describe("action (direct)", () => {
    it("revokes a specific session", async () => {
      vi.mocked(sessionApi.revokeSession).mockResolvedValue(undefined);

      const formData = new FormData();
      formData.append("intent", "revoke");
      formData.append("sessionId", "session-2");

      const request = new Request("http://localhost/dashboard/settings/sessions", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(sessionApi.revokeSession).toHaveBeenCalledWith("session-2", "mock-access-token");
      expect(result).toBeInstanceOf(Response);
      expect((result as Response).status).toBe(302);
    });

    it("revokes all other sessions", async () => {
      vi.mocked(sessionApi.revokeOtherSessions).mockResolvedValue(undefined);

      const formData = new FormData();
      formData.append("intent", "revoke_all");

      const request = new Request("http://localhost/dashboard/settings/sessions", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(sessionApi.revokeOtherSessions).toHaveBeenCalledWith("mock-access-token");
      expect(result).toBeInstanceOf(Response);
    });

    it("returns error when not authenticated", async () => {
      vi.mocked(getAccessToken).mockResolvedValueOnce(null);

      const formData = new FormData();
      formData.append("intent", "revoke");
      formData.append("sessionId", "session-2");

      const request = new Request("http://localhost/dashboard/settings/sessions", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(result).toEqual({ error: "Not authenticated" });
    });

    it("returns error on API failure", async () => {
      vi.mocked(sessionApi.revokeSession).mockRejectedValue(new Error("Not found"));

      const formData = new FormData();
      formData.append("intent", "revoke");
      formData.append("sessionId", "bad-id");

      const request = new Request("http://localhost/dashboard/settings/sessions", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(result).toEqual({ error: "Not found" });
    });

    it("returns error for invalid intent", async () => {
      const formData = new FormData();
      formData.append("intent", "invalid");

      const request = new Request("http://localhost/dashboard/settings/sessions", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(result).toEqual({ error: "Invalid action" });
    });
  });

  it("renders 'Just now' for very recent sessions", async () => {
    const justNow = new Date().toISOString();
    const recentSession = {
      ...mockCurrentSession,
      last_active_at: justNow,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/sessions",
        Component: SessionsPage,
        loader: () => ({ sessions: [recentSession] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/sessions"]} />);

    await waitFor(() => {
      expect(screen.getByText(/Just now/)).toBeInTheDocument();
    });
  });
});
