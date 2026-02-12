import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ActionsListPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId.actions._index";
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
  getTriggers: vi.fn(() => Promise.resolve({ data: Object.values(ActionTrigger) })),
}));

import { getAccessToken } from "~/services/session.server";

const mockActionsApi = {
  list: vi.fn(),
  update: vi.fn(),
  delete: vi.fn(),
};

const mockAction1 = {
  id: "action-1",
  tenantId: "tenant-1",
  name: "Add Custom Claims",
  description: "Adds department and tier claims",
  triggerId: ActionTrigger.PostLogin,
  script: "context.claims = { department: 'eng' }; context;",
  enabled: true,
  executionOrder: 0,
  timeoutMs: 3000,
  executionCount: 100,
  errorCount: 5,
  lastExecutedAt: "2024-01-15T10:00:00Z",
  lastError: null,
  createdAt: "2024-01-01T00:00:00Z",
  updatedAt: "2024-01-15T00:00:00Z",
};

const mockAction2 = {
  id: "action-2",
  tenantId: "tenant-1",
  name: "Block Spam Domains",
  description: "Prevents registration from spam domains",
  triggerId: ActionTrigger.PreUserRegistration,
  script: "if (context.user.email.includes('spam')) throw new Error('blocked'); context;",
  enabled: false,
  executionOrder: 1,
  timeoutMs: 5000,
  executionCount: 50,
  errorCount: 10,
  lastExecutedAt: null,
  lastError: "Domain not allowed",
  createdAt: "2024-01-02T00:00:00Z",
  updatedAt: "2024-01-10T00:00:00Z",
};

describe("Actions List Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns all actions when no filter is applied", async () => {
    mockActionsApi.list.mockResolvedValue({ data: [mockAction1, mockAction2] });

    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/actions"),
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(response).toEqual({
      tenantId: "tenant-1",
      actions: [mockAction1, mockAction2],
      triggers: Object.values(ActionTrigger),
      currentTrigger: null,
    });
    expect(mockActionsApi.list).toHaveBeenCalledWith(undefined);
  });

  it("loader filters actions by trigger type", async () => {
    mockActionsApi.list.mockResolvedValue({ data: [mockAction1] });

    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/actions?trigger=post_login"),
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(response.currentTrigger).toBe("post_login");
    expect(mockActionsApi.list).toHaveBeenCalledWith("post_login");
  });

  it("loader throws when tenantId is missing", async () => {
    await expect(
      loader({
        request: new Request("http://localhost/dashboard/tenants//actions"),
        params: {},
        context: {},
      })
    ).rejects.toThrow("Tenant ID is required");
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders page header and create button", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({ tenantId: "tenant-1", actions: [], triggers: [], currentTrigger: null }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Actions")).toBeInTheDocument();
    });
    expect(screen.getByText(/manage authentication flow actions/i)).toBeInTheDocument();
    expect(screen.getByText("New Action")).toBeInTheDocument();
  });

  it("renders empty state when no actions", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({ tenantId: "tenant-1", actions: [], triggers: [], currentTrigger: null }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("No actions found")).toBeInTheDocument();
    });
    expect(screen.getByText(/get started by creating your first action/i)).toBeInTheDocument();
  });

  it("renders action list with status indicators", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1, mockAction2],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });
    expect(screen.getByText("Block Spam Domains")).toBeInTheDocument();
    expect(screen.getByText("Enabled")).toBeInTheDocument();
    expect(screen.getByText("Disabled")).toBeInTheDocument();
  });

  it("renders action statistics", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("100")).toBeInTheDocument(); // Execution count
    });
    expect(screen.getByText("95.0%")).toBeInTheDocument(); // Success rate
  });

  it("renders success rate with green icon when >= 95%", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    const { container } = render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      const greenIcon = container.querySelector(".text-green-500");
      expect(greenIcon).toBeInTheDocument();
    });
  });

  it("renders success rate with red icon when < 95%", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction2],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    const { container } = render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      const redIcon = container.querySelector(".text-red-500");
      expect(redIcon).toBeInTheDocument();
    });
  });

  it("renders last executed time", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText(/1\/15\/2024/)).toBeInTheDocument();
    });
  });

  it("renders 'Never' when action has not been executed", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction2],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Never")).toBeInTheDocument();
    });
  });

  it("renders last error when present", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction2],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Domain not allowed")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Interaction Tests
  // ============================================================================

  it("filters actions by search query", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1, mockAction2],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText("Search actions...");
    await user.type(searchInput, "spam");

    await waitFor(() => {
      expect(screen.getByText("Block Spam Domains")).toBeInTheDocument();
      expect(screen.queryByText("Add Custom Claims")).not.toBeInTheDocument();
    });
  });

  it("shows empty message when search returns no results", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1],
          triggers: [],
          currentTrigger: null,
        }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText("Search actions...");
    await user.type(searchInput, "nonexistent");

    await waitFor(() => {
      expect(screen.getByText("No actions found")).toBeInTheDocument();
      expect(screen.getByText(/try adjusting your search query/i)).toBeInTheDocument();
    });
  });

  it("renders toggle switch with correct state", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1, mockAction2],
          triggers: [],
          currentTrigger: null,
        }),
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const switches = screen.getAllByRole("switch");

    // First action is enabled
    expect(switches[0]).toBeChecked();

    // Second action is disabled
    expect(switches[1]).not.toBeChecked();
  });

  it("delete button shows confirmation and deletes action", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1],
          triggers: [],
          currentTrigger: null,
        }),
        action,
      },
    ]);

    const user = userEvent.setup();
    mockActionsApi.delete.mockResolvedValue({ data: {} });

    // Mock window.confirm
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const deleteButton = screen.getByRole("button", { name: /delete/i });
    await user.click(deleteButton);

    expect(confirmSpy).toHaveBeenCalledWith('Are you sure you want to delete "Add Custom Claims"?');

    await waitFor(() => {
      expect(mockActionsApi.delete).toHaveBeenCalledWith("action-1");
    });

    confirmSpy.mockRestore();
  });

  it("cancel delete confirmation does not delete", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions",
        Component: ActionsListPage,
        loader: () => ({
          tenantId: "tenant-1",
          actions: [mockAction1],
          triggers: [],
          currentTrigger: null,
        }),
        action,
      },
    ]);

    const user = userEvent.setup();

    // Mock window.confirm to return false
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions"]} />);

    await waitFor(() => {
      expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
    });

    const deleteButton = screen.getByRole("button", { name: /delete/i });
    await user.click(deleteButton);

    expect(confirmSpy).toHaveBeenCalled();
    expect(mockActionsApi.delete).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
  });

  // ============================================================================
  // Action Handler Tests
  // ============================================================================

  describe("action", () => {
    beforeEach(() => {
      vi.mocked(getAccessToken).mockResolvedValue("test-token");
    });

    function createFormRequest(data: Record<string, string>) {
      const formData = new FormData();
      for (const [key, value] of Object.entries(data)) {
        formData.append(key, value);
      }
      return new Request("http://localhost/dashboard/tenants/tenant-1/actions", {
        method: "POST",
        body: formData,
      });
    }

    it("handles toggle intent", async () => {
      mockActionsApi.update.mockResolvedValue({ data: { ...mockAction1, enabled: false } });

      const request = createFormRequest({
        intent: "toggle",
        actionId: "action-1",
        enabled: "false",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(mockActionsApi.update).toHaveBeenCalledWith("action-1", { enabled: false });
      expect(response).toEqual({ success: true });
    });

    it("handles delete intent", async () => {
      mockActionsApi.delete.mockResolvedValue({ data: {} });

      const request = createFormRequest({
        intent: "delete",
        actionId: "action-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(mockActionsApi.delete).toHaveBeenCalledWith("action-1");
      expect(response).toEqual({ success: true });
    });

    it("returns error when tenantId is missing", async () => {
      const request = createFormRequest({ intent: "toggle", actionId: "action-1" });

      const response = await action({ request, params: {}, context: {} });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Tenant ID required" });
    });

    it("returns error for invalid intent", async () => {
      const request = createFormRequest({ intent: "invalid", actionId: "action-1" });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Invalid intent" });
    });

    it("returns error when API call fails", async () => {
      mockActionsApi.update.mockRejectedValue(new Error("Network error"));

      const request = createFormRequest({
        intent: "toggle",
        actionId: "action-1",
        enabled: "true",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Network error" });
    });
  });
});
