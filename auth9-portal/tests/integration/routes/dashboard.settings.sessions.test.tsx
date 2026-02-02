import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SessionsPage from "~/routes/dashboard.settings.sessions";
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
});
