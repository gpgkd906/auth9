import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
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
  // Loader Tests
  // ============================================================================

  it("loader returns sessions from API", async () => {
    vi.mocked(sessionApi.listMySessions).mockResolvedValue({
      data: [mockCurrentSession, mockOtherSession],
    });

    const response = await loader();

    expect(response).toEqual({
      sessions: [mockCurrentSession, mockOtherSession],
    });
  });

  it("loader returns empty sessions on API error", async () => {
    vi.mocked(sessionApi.listMySessions).mockRejectedValue(new Error("API Error"));

    const response = await loader();

    expect(response).toEqual({
      sessions: [],
      error: "Failed to load sessions",
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
  // Action Tests
  // ============================================================================

  it("action revokes specific session", async () => {
    vi.mocked(sessionApi.revokeSession).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("intent", "revoke");
    formData.append("sessionId", "session-2");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(sessionApi.revokeSession).toHaveBeenCalledWith("session-2", "");
    expect(response).toEqual({ success: true, message: "Session revoked" });
  });

  it("action revokes all other sessions", async () => {
    vi.mocked(sessionApi.revokeOtherSessions).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("intent", "revoke_all");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(sessionApi.revokeOtherSessions).toHaveBeenCalledWith("");
    expect(response).toEqual({ success: true, message: "All other sessions revoked" });
  });

  it("action returns error on API failure", async () => {
    vi.mocked(sessionApi.revokeSession).mockRejectedValue(new Error("Session not found"));

    const formData = new FormData();
    formData.append("intent", "revoke");
    formData.append("sessionId", "invalid-session");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Session not found" });
  });

  it("action returns error for invalid intent", async () => {
    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request("http://localhost/dashboard/settings/sessions", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Invalid action" });
  });
});
