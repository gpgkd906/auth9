import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import AnalyticsPage, { loader } from "~/routes/dashboard.analytics";
import { analyticsApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  analyticsApi: {
    getStats: vi.fn(),
    getDailyTrend: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("mock-access-token"),
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

const mockDailyTrend = [
  { date: "2026-02-08", total: 150, successful: 140, failed: 10 },
  { date: "2026-02-09", total: 180, successful: 170, failed: 10 },
  { date: "2026-02-10", total: 200, successful: 190, failed: 10 },
  { date: "2026-02-11", total: 160, successful: 150, failed: 10 },
  { date: "2026-02-12", total: 190, successful: 180, failed: 10 },
  { date: "2026-02-13", total: 180, successful: 170, failed: 10 },
  { date: "2026-02-14", total: 190, successful: 180, failed: 10 },
];

const mockStats = {
  total_logins: 1250,
  successful_logins: 1180,
  failed_logins: 70,
  unique_users: 342,
  by_event_type: {
    login_success: 1180,
    login_failed: 50,
    account_locked: 20,
  },
  by_device_type: {
    desktop: 800,
    mobile: 400,
    tablet: 50,
  },
};

describe("Analytics Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns stats from API with default 7 days", async () => {
    vi.mocked(analyticsApi.getStats).mockResolvedValue({ data: mockStats });
    vi.mocked(analyticsApi.getDailyTrend).mockResolvedValue({ data: mockDailyTrend });

    const request = new Request("http://localhost/dashboard/analytics");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toMatchObject({
      stats: mockStats,
      dailyTrend: mockDailyTrend,
      days: 7,
      rangeLabel: "Last 7 days",
    });
    expect(analyticsApi.getStats).toHaveBeenCalled();
    expect(analyticsApi.getDailyTrend).toHaveBeenCalledWith(7, "mock-access-token", undefined, undefined);
  });

  it("loader uses custom days parameter", async () => {
    vi.mocked(analyticsApi.getStats).mockResolvedValue({ data: mockStats });
    vi.mocked(analyticsApi.getDailyTrend).mockResolvedValue({ data: mockDailyTrend });

    const request = new Request("http://localhost/dashboard/analytics?days=30");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toMatchObject({
      stats: mockStats,
      dailyTrend: mockDailyTrend,
      days: 30,
      rangeLabel: "Last 30 days",
    });
  });

  it("loader supports custom date range", async () => {
    vi.mocked(analyticsApi.getStats).mockResolvedValue({ data: mockStats });
    vi.mocked(analyticsApi.getDailyTrend).mockResolvedValue({ data: mockDailyTrend });

    const request = new Request("http://localhost/dashboard/analytics?start=2026-01-01&end=2026-01-15");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toMatchObject({
      stats: mockStats,
      dailyTrend: mockDailyTrend,
      customStart: "2026-01-01",
      customEnd: "2026-01-15",
      rangeLabel: "2026-01-01 - 2026-01-15",
    });
    expect(response.days).toBe(14);
  });

  it("loader returns error on API failure", async () => {
    vi.mocked(analyticsApi.getStats).mockRejectedValue(new Error("API Error"));
    vi.mocked(analyticsApi.getDailyTrend).mockRejectedValue(new Error("API Error"));

    const request = new Request("http://localhost/dashboard/analytics");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toMatchObject({
      stats: null,
      dailyTrend: [],
      days: 7,
      rangeLabel: "Last 7 days",
      error: "Failed to load analytics",
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders analytics page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getByText("Analytics")).toBeInTheDocument();
    });
    expect(screen.getByText("Login activity and statistics")).toBeInTheDocument();
  });

  it("renders date range selectors", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getByText("7d")).toBeInTheDocument();
    });
    expect(screen.getByText("14d")).toBeInTheDocument();
    expect(screen.getByText("30d")).toBeInTheDocument();
    expect(screen.getByText("90d")).toBeInTheDocument();
    expect(screen.getByText("Custom")).toBeInTheDocument();
  });

  it("renders key metrics cards", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getByText("Total Logins")).toBeInTheDocument();
    });
    expect(screen.getAllByText("Successful").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("Failed").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Unique Users")).toBeInTheDocument();
  });

  it("renders metrics values formatted correctly", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      // Check formatted numbers (1,250)
      expect(screen.getByText("1,250")).toBeInTheDocument();
    });
    expect(screen.getByText("1,180")).toBeInTheDocument();
    expect(screen.getByText("70")).toBeInTheDocument();
    expect(screen.getByText("342")).toBeInTheDocument();
  });

  it("renders success rate calculation", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      // Success rate: 1180/1250 = 94.4%
      expect(screen.getByText(/94\.4% success rate/)).toBeInTheDocument();
    });
  });

  it("renders breakdown charts", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getByText("By Event Type")).toBeInTheDocument();
    });
    expect(screen.getByText("By Device Type")).toBeInTheDocument();
    expect(screen.getByText("login success")).toBeInTheDocument();
    expect(screen.getByText("desktop")).toBeInTheDocument();
  });

  it("renders link to events page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: mockStats, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getByText("View Login Events")).toBeInTheDocument();
    });
    const link = screen.getByRole("link", { name: /view events/i });
    expect(link).toHaveAttribute("href", "/dashboard/analytics/events");
  });

  it("renders error message when stats loading fails", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: null, dailyTrend: [], days: 7, rangeLabel: "Last 7 days", error: "Failed to load analytics" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getByText("Failed to load analytics")).toBeInTheDocument();
    });
  });

  it("handles empty breakdown data", async () => {
    const statsWithEmptyBreakdown = {
      ...mockStats,
      by_event_type: {},
      by_device_type: {},
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics",
        Component: AnalyticsPage,
        loader: () => ({ stats: statsWithEmptyBreakdown, dailyTrend: mockDailyTrend, days: 7, rangeLabel: "Last 7 days" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics"]} />);

    await waitFor(() => {
      expect(screen.getAllByText("No data available")).toHaveLength(2);
    });
  });
});
