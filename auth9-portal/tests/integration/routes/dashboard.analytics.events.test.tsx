import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import LoginEventsPage, { loader } from "~/routes/dashboard.analytics.events";
import { analyticsApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  analyticsApi: {
    listEvents: vi.fn(),
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

const mockEvents = [
  {
    id: "evt-1",
    user_id: "user-1",
    email: "test@example.com",
    event_type: "success",
    ip_address: "192.168.1.1",
    device_type: "desktop",
    created_at: "2024-01-15T10:30:00Z",
  },
  {
    id: "evt-2",
    user_id: "user-2",
    email: "failed@example.com",
    event_type: "failed_password",
    ip_address: "192.168.1.2",
    device_type: "mobile",
    failure_reason: "Invalid password",
    created_at: "2024-01-15T11:00:00Z",
  },
];

const mockPagination = {
  page: 1,
  per_page: 50,
  total: 2,
  total_pages: 1,
};

describe("Login Events Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns events from API", async () => {
    vi.mocked(analyticsApi.listEvents).mockResolvedValue({
      data: mockEvents,
      pagination: mockPagination,
    });

    const request = new Request("http://localhost/dashboard/analytics/events");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({
      events: mockEvents,
      pagination: mockPagination,
    });
    expect(analyticsApi.listEvents).toHaveBeenCalledWith(1, 50, undefined, "mock-access-token");
  });

  it("loader uses page parameter from URL", async () => {
    vi.mocked(analyticsApi.listEvents).mockResolvedValue({
      data: mockEvents,
      pagination: { ...mockPagination, page: 2 },
    });

    const request = new Request(
      "http://localhost/dashboard/analytics/events?page=2"
    );
    const response = await loader({ request, params: {}, context: {} });

    expect(analyticsApi.listEvents).toHaveBeenCalledWith(2, 50, undefined, "mock-access-token");
    expect(response.pagination.page).toBe(2);
  });

  it("loader handles API error gracefully", async () => {
    vi.mocked(analyticsApi.listEvents).mockRejectedValue(new Error("API Error"));

    const request = new Request("http://localhost/dashboard/analytics/events");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({
      events: [],
      pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
      error: "Failed to load events",
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders login events page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Login Events")).toBeInTheDocument();
    });
    expect(
      screen.getByText("Detailed log of all authentication attempts")
    ).toBeInTheDocument();
  });

  it("renders back to analytics link", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("← Back to Analytics")).toBeInTheDocument();
    });
  });

  it("renders events table with headers", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Time")).toBeInTheDocument();
    });
    expect(screen.getByText("Event")).toBeInTheDocument();
    expect(screen.getByText("User")).toBeInTheDocument();
    expect(screen.getByText("IP Address")).toBeInTheDocument();
    expect(screen.getByText("Device")).toBeInTheDocument();
  });

  it("renders event data correctly", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("test@example.com")).toBeInTheDocument();
    });
    expect(screen.getByText("192.168.1.1")).toBeInTheDocument();
    expect(screen.getByText("Login Success")).toBeInTheDocument();
  });

  it("renders failed event with reason", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Wrong Password")).toBeInTheDocument();
    });
    expect(screen.getByText("Invalid password")).toBeInTheDocument();
  });

  it("renders empty state when no events", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("No events found")).toBeInTheDocument();
    });
  });

  it("renders error message", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: [],
          pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
          error: "Failed to load events",
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Failed to load events")).toBeInTheDocument();
    });
  });

  it("renders total count", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: mockEvents,
          pagination: { ...mockPagination, total: 150 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("150 total")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Pagination Tests
  // ============================================================================

  it("renders pagination controls when multiple pages", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: mockEvents,
          pagination: { page: 2, per_page: 50, total: 150, total_pages: 3 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Page 2 of 3")).toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: /previous/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /next/i })).toBeInTheDocument();
  });

  it("does not render pagination when single page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: mockEvents,
          pagination: { page: 1, per_page: 50, total: 2, total_pages: 1 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("2 total")).toBeInTheDocument();
    });
    expect(screen.queryByText(/Page \d+ of \d+/)).not.toBeInTheDocument();
  });

  // ============================================================================
  // Event Type Badge Tests
  // ============================================================================

  it("renders social login event correctly", async () => {
    const socialEvent = {
      id: "evt-3",
      user_id: "user-3",
      email: "social@example.com",
      event_type: "social",
      ip_address: "192.168.1.3",
      device_type: "mobile",
      created_at: "2024-01-15T12:00:00Z",
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: [socialEvent],
          pagination: { page: 1, per_page: 50, total: 1, total_pages: 1 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Social Login")).toBeInTheDocument();
      expect(screen.getByText("social@example.com")).toBeInTheDocument();
    });
  });

  it("renders locked account event correctly", async () => {
    const lockedEvent = {
      id: "evt-4",
      user_id: "user-4",
      email: "locked@example.com",
      event_type: "locked",
      ip_address: "192.168.1.4",
      device_type: "desktop",
      created_at: "2024-01-15T13:00:00Z",
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: [lockedEvent],
          pagination: { page: 1, per_page: 50, total: 1, total_pages: 1 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Account Locked")).toBeInTheDocument();
      expect(screen.getByText("locked@example.com")).toBeInTheDocument();
    });
  });

  it("renders MFA failed event correctly", async () => {
    const mfaFailedEvent = {
      id: "evt-5",
      user_id: "user-5",
      email: "mfa@example.com",
      event_type: "failed_mfa",
      ip_address: "192.168.1.5",
      device_type: "desktop",
      failure_reason: "Invalid code",
      created_at: "2024-01-15T14:00:00Z",
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({
          events: [mfaFailedEvent],
          pagination: { page: 1, per_page: 50, total: 1, total_pages: 1 },
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("MFA Failed")).toBeInTheDocument();
      expect(screen.getByText("Invalid code")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Table Card Tests
  // ============================================================================

  it("renders Recent Events card title", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("Recent Events")).toBeInTheDocument();
    });
  });

  it("renders device type in table row", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/analytics/events",
        Component: LoginEventsPage,
        loader: () => ({ events: mockEvents, pagination: mockPagination }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/analytics/events"]} />);

    await waitFor(() => {
      expect(screen.getByText("desktop")).toBeInTheDocument();
      expect(screen.getByText("mobile")).toBeInTheDocument();
    });
  });
});
