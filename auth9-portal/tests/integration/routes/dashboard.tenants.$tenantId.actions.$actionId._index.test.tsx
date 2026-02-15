import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ActionDetailPage, { loader } from "~/routes/dashboard.tenants.$tenantId.actions.$actionId._index";
import { ActionTrigger } from "@auth9/core";

// Mock the session module
vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

// Mock the auth9-client module
vi.mock("~/lib/auth9-client", () => ({
  getAuth9Client: vi.fn((token) => ({
    actions: mockActionsApi,
  })),
  withTenant: vi.fn((client, tenantId) => ({
    actions: mockActionsApi,
  })),
}));

const mockActionsApi = {
  get: vi.fn(),
  logs: vi.fn(),
  stats: vi.fn(),
};

const mockAction = {
  id: "action-1",
  tenantId: "tenant-1",
  name: "Add Custom Claims",
  description: "Adds department and tier claims",
  triggerId: ActionTrigger.PostLogin,
  script: 'context.claims = { department: "engineering", tier: "premium" };\ncontext;',
  enabled: true,
  strictMode: false,
  executionOrder: 0,
  timeoutMs: 3000,
  executionCount: 100,
  errorCount: 5,
  lastExecutedAt: "2024-01-15T10:00:00Z",
  lastError: null,
  createdAt: "2024-01-01T00:00:00Z",
  updatedAt: "2024-01-15T00:00:00Z",
};

const mockLogs = [
  {
    id: "log-1",
    actionId: "action-1",
    userId: "user-1",
    success: true,
    durationMs: 150,
    executedAt: "2024-01-15T10:00:00Z",
    errorMessage: null,
  },
  {
    id: "log-2",
    actionId: "action-1",
    userId: "user-2",
    success: false,
    durationMs: 50,
    executedAt: "2024-01-15T09:00:00Z",
    errorMessage: "User not found",
  },
];

const mockStats = {
  executionCount: 100,
  errorCount: 5,
  avgDurationMs: 125,
  last24hCount: 25,
};

describe("Action Detail Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader fetches action, logs, and stats", async () => {
    mockActionsApi.get.mockResolvedValue({ data: mockAction });
    mockActionsApi.logs.mockResolvedValue({ data: mockLogs });
    mockActionsApi.stats.mockResolvedValue({ data: mockStats });

    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/actions/action-1"),
      params: { tenantId: "tenant-1", actionId: "action-1" },
      context: {},
    });

    expect(response).toEqual({
      tenantId: "tenant-1",
      action: mockAction,
      logs: mockLogs,
      stats: mockStats,
    });
    expect(mockActionsApi.get).toHaveBeenCalledWith("action-1");
    expect(mockActionsApi.logs).toHaveBeenCalledWith({ actionId: "action-1", limit: 50 });
    expect(mockActionsApi.stats).toHaveBeenCalledWith("action-1");
  });

  it("loader handles stats API failure gracefully", async () => {
    mockActionsApi.get.mockResolvedValue({ data: mockAction });
    mockActionsApi.logs.mockResolvedValue({ data: [] });
    mockActionsApi.stats.mockRejectedValue(new Error("Stats not available"));

    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/actions/action-1"),
      params: { tenantId: "tenant-1", actionId: "action-1" },
      context: {},
    });

    expect(response.stats).toBeNull();
  });

  it("loader throws when tenantId is missing", async () => {
    await expect(
      loader({
        request: new Request("http://localhost/dashboard/tenants//actions/action-1"),
        params: { actionId: "action-1" },
        context: {},
      })
    ).rejects.toThrow("Tenant ID and Action ID are required");
  });

  it("loader throws when actionId is missing", async () => {
    await expect(
      loader({
        request: new Request("http://localhost/dashboard/tenants/tenant-1/actions/"),
        params: { tenantId: "tenant-1" },
        context: {},
      })
    ).rejects.toThrow("Tenant ID and Action ID are required");
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders action name and status badges", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });
    expect(screen.getByText("Enabled")).toBeInTheDocument();
    expect(screen.getByText("Post Login")).toBeInTheDocument();
  });

  it("renders action description", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("Adds department and tier claims")).toBeInTheDocument();
    });
  });

  it("renders statistics cards when stats are available", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: mockStats,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("100")).toBeInTheDocument(); // Total Executions
    });
    expect(screen.getByText("125ms")).toBeInTheDocument(); // Avg Duration
    expect(screen.getByText("25")).toBeInTheDocument(); // Last 24h count
  });

  it("renders success rate with green icon when >= 95%", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: mockStats,
        }),
      },
    ]);

    const { container } = render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      const greenIcon = container.querySelector(".text-green-500");
      expect(greenIcon).toBeInTheDocument();
    });
    expect(screen.getByText("95.0%")).toBeInTheDocument();
  });

  it("renders success rate with red icon when < 95%", async () => {
    const lowSuccessStats = {
      ...mockStats,
      executionCount: 100,
      errorCount: 10, // 90% success rate
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: lowSuccessStats,
        }),
      },
    ]);

    const { container } = render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      const redIcon = container.querySelector(".text-red-500");
      expect(redIcon).toBeInTheDocument();
    });
    expect(screen.getByText("90.0%")).toBeInTheDocument();
  });

  it("renders script code in Script tab", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText(/context.claims/)).toBeInTheDocument();
    });
    expect(screen.getByText("TypeScript Code")).toBeInTheDocument();
  });

  it("renders execution order and timeout", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("0")).toBeInTheDocument(); // Execution order
    });
    expect(screen.getByText("3000ms")).toBeInTheDocument(); // Timeout
  });

  it("renders execution logs in Logs tab", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: mockLogs,
          stats: null,
        }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    // Click the Logs tab by text
    const logsTab = screen.getByText(/execution logs/i);
    await user.click(logsTab);

    await waitFor(() => {
      expect(screen.getByText("Success")).toBeInTheDocument();
    });
    expect(screen.getByText("Failed")).toBeInTheDocument();
    expect(screen.getByText("150ms")).toBeInTheDocument();
    expect(screen.getByText("User not found")).toBeInTheDocument();
  });

  it("renders success log with green background", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: mockLogs,
          stats: null,
        }),
      },
    ]);

    const user = userEvent.setup();
    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const logsTab = screen.getByText(/execution logs/i);
    await user.click(logsTab);

    await waitFor(() => {
      const greenBg = container.querySelector(".bg-green-50");
      expect(greenBg).toBeInTheDocument();
    });
  });

  it("renders failed log with red background and error message", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: mockLogs,
          stats: null,
        }),
      },
    ]);

    const user = userEvent.setup();
    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const logsTab = screen.getByText(/execution logs/i);
    await user.click(logsTab);

    await waitFor(() => {
      const redBg = container.querySelector(".bg-red-50");
      expect(redBg).toBeInTheDocument();
    });
    expect(screen.getByText("User not found")).toBeInTheDocument();
  });

  it("renders empty state when no logs", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const logsTab = screen.getByText(/execution logs/i);
    await user.click(logsTab);

    await waitFor(() => {
      expect(screen.getByText("No executions yet")).toBeInTheDocument();
    });
  });

  it("renders metadata section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      expect(screen.getByText("Metadata")).toBeInTheDocument();
    });
    expect(screen.getByText("action-1")).toBeInTheDocument();
    expect(screen.getByText("tenant-1")).toBeInTheDocument();
  });

  it("renders Edit button linking to edit page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/:actionId",
        Component: ActionDetailPage,
        loader: () => ({
          tenantId: "tenant-1",
          action: mockAction,
          logs: [],
          stats: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/action-1"]} />);

    await waitFor(() => {
      const editButton = screen.getByRole("link", { name: /edit/i });
      expect(editButton).toHaveAttribute("href", "/dashboard/tenants/tenant-1/actions/action-1/edit");
    });
  });
});
