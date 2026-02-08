import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SecurityAlertsPage, { loader, action } from "~/routes/dashboard.security.alerts";
import { securityAlertApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  securityAlertApi: {
    list: vi.fn(),
    resolve: vi.fn(),
  },
}));

const mockAlertCritical = {
  id: "alert-1",
  alert_type: "brute_force",
  severity: "critical",
  user_id: "user-12345678-abcd",
  created_at: "2024-01-15T10:00:00Z",
  resolved_at: null,
  details: { attempts: 50, ip: "192.168.1.100" },
};

const mockAlertResolved = {
  id: "alert-2",
  alert_type: "new_device",
  severity: "medium",
  user_id: "user-87654321-dcba",
  created_at: "2024-01-14T08:30:00Z",
  resolved_at: "2024-01-14T09:00:00Z",
  details: null,
};

const mockPagination = {
  page: 1,
  per_page: 50,
  total: 2,
  total_pages: 1,
};

describe("Security Alerts Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns alerts from API", async () => {
    vi.mocked(securityAlertApi.list).mockResolvedValue({
      data: [mockAlertCritical, mockAlertResolved],
      pagination: mockPagination,
    });

    const request = new Request("http://localhost/dashboard/security/alerts");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({
      alerts: [mockAlertCritical, mockAlertResolved],
      pagination: mockPagination,
      unresolvedOnly: false,
    });
    expect(securityAlertApi.list).toHaveBeenCalledWith(1, 50, false, undefined);
  });

  it("loader respects unresolved filter", async () => {
    vi.mocked(securityAlertApi.list).mockResolvedValue({
      data: [mockAlertCritical],
      pagination: { ...mockPagination, total: 1 },
    });

    const request = new Request("http://localhost/dashboard/security/alerts?unresolved=true");
    const response = await loader({ request, params: {}, context: {} });

    expect(response.unresolvedOnly).toBe(true);
    expect(securityAlertApi.list).toHaveBeenCalledWith(1, 50, true, undefined);
  });

  it("loader returns error on API failure", async () => {
    vi.mocked(securityAlertApi.list).mockRejectedValue(new Error("API Error"));

    const request = new Request("http://localhost/dashboard/security/alerts");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({
      alerts: [],
      pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
      unresolvedOnly: false,
      error: "Failed to load security alerts",
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders security alerts page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("Security Alerts")).toBeInTheDocument();
    });
    expect(screen.getByText("Monitor and respond to security threats")).toBeInTheDocument();
  });

  it("renders filter buttons", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("All")).toBeInTheDocument();
    });
    expect(screen.getByText(/unresolved/i)).toBeInTheDocument();
  });

  it("renders alert list with correct severity styling", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("CRITICAL")).toBeInTheDocument();
    });
    expect(screen.getByText("Brute Force Attack")).toBeInTheDocument();
  });

  it("renders resolve button for unresolved alerts", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /resolve/i })).toBeInTheDocument();
    });
  });

  it("renders resolved badge for resolved alerts", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertResolved],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("Resolved")).toBeInTheDocument();
    });
    // No resolve button for already resolved alerts
    expect(screen.queryByRole("button", { name: /resolve/i })).not.toBeInTheDocument();
  });

  it("renders empty state when no alerts", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("All clear!")).toBeInTheDocument();
    });
    expect(screen.getByText("No security alerts found.")).toBeInTheDocument();
  });

  it("renders unresolved-specific empty state", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          unresolvedOnly: true,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts?unresolved=true"]} />);

    await waitFor(() => {
      expect(screen.getByText("No unresolved security alerts.")).toBeInTheDocument();
    });
  });

  it("renders security recommendations", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("Security Recommendations")).toBeInTheDocument();
    });
    expect(screen.getByText(/review and resolve critical alerts/i)).toBeInTheDocument();
    expect(screen.getByText(/enable mfa for all admin accounts/i)).toBeInTheDocument();
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action resolves alert", async () => {
    vi.mocked(securityAlertApi.resolve).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("intent", "resolve");
    formData.append("alertId", "alert-1");

    const request = new Request("http://localhost/dashboard/security/alerts", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(securityAlertApi.resolve).toHaveBeenCalledWith("alert-1", undefined);
    expect(response).toEqual({ success: true, message: "Alert resolved" });
  });

  it("action returns error on API failure", async () => {
    vi.mocked(securityAlertApi.resolve).mockRejectedValue(new Error("Alert not found"));

    const formData = new FormData();
    formData.append("intent", "resolve");
    formData.append("alertId", "invalid-id");

    const request = new Request("http://localhost/dashboard/security/alerts", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Alert not found" });
  });

  it("action returns error for invalid intent", async () => {
    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request("http://localhost/dashboard/security/alerts", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Invalid action" });
  });

  it("action returns generic error for non-Error throw", async () => {
    vi.mocked(securityAlertApi.resolve).mockRejectedValue("unexpected");

    const formData = new FormData();
    formData.append("intent", "resolve");
    formData.append("alertId", "alert-1");

    const request = new Request("http://localhost/dashboard/security/alerts", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "Operation failed" });
  });

  // ============================================================================
  // Severity & Alert Type Coverage
  // ============================================================================

  it("renders high severity alert", async () => {
    const highAlert = {
      id: "alert-h",
      alert_type: "impossible_travel",
      severity: "high",
      user_id: "user-abc",
      created_at: "2024-01-15T10:00:00Z",
      resolved_at: null,
      details: null,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [highAlert],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("HIGH")).toBeInTheDocument();
      expect(screen.getByText("Impossible Travel")).toBeInTheDocument();
    });
  });

  it("renders low severity alert", async () => {
    const lowAlert = {
      id: "alert-l",
      alert_type: "suspicious_ip",
      severity: "low",
      user_id: null,
      created_at: "2024-01-15T10:00:00Z",
      resolved_at: null,
      details: null,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [lowAlert],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("LOW")).toBeInTheDocument();
      expect(screen.getByText("Suspicious IP")).toBeInTheDocument();
    });
  });

  it("renders unknown severity/type alert", async () => {
    const unknownAlert = {
      id: "alert-u",
      alert_type: "custom_alert_type",
      severity: "info",
      user_id: null,
      created_at: "2024-01-15T10:00:00Z",
      resolved_at: null,
      details: null,
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [unknownAlert],
          pagination: mockPagination,
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("INFO")).toBeInTheDocument();
      expect(screen.getByText("custom alert type")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Pagination Tests
  // ============================================================================

  it("renders pagination with Previous and Next buttons", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: { page: 2, per_page: 50, total: 150, total_pages: 3 },
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts?page=2"]} />);

    await waitFor(() => {
      expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /Previous/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Next/i })).toBeInTheDocument();
  });

  it("renders only Next button on first page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: { page: 1, per_page: 50, total: 100, total_pages: 2 },
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);

    await waitFor(() => {
      expect(screen.getByText("Page 1 of 2")).toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: /Previous/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Next/i })).toBeInTheDocument();
  });

  it("renders only Previous button on last page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [mockAlertCritical],
          pagination: { page: 3, per_page: 50, total: 150, total_pages: 3 },
          unresolvedOnly: false,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts?page=3"]} />);

    await waitFor(() => {
      expect(screen.getByText("Page 3 of 3")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /Previous/i })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Next/i })).not.toBeInTheDocument();
  });

  // ============================================================================
  // Messages Display
  // ============================================================================

  it("renders action success message", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          unresolvedOnly: false,
        }),
        action: () => ({ success: true, message: "Alert resolved" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);
    await waitFor(() => {
      expect(screen.getByText("All clear!")).toBeInTheDocument();
    });
  });

  it("renders load error message", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/security/alerts",
        Component: SecurityAlertsPage,
        loader: () => ({
          alerts: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          unresolvedOnly: false,
          error: "Failed to load security alerts",
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/security/alerts"]} />);
    await waitFor(() => {
      expect(screen.getByText("Failed to load security alerts")).toBeInTheDocument();
    });
  });

  it("loader passes page parameter to API", async () => {
    vi.mocked(securityAlertApi.list).mockResolvedValue({
      data: [],
      pagination: { page: 3, per_page: 50, total: 0, total_pages: 0 },
    });

    const request = new Request("http://localhost/dashboard/security/alerts?page=3");
    await loader({ request, params: {}, context: {} });

    expect(securityAlertApi.list).toHaveBeenCalledWith(3, 50, false, undefined);
  });
});
