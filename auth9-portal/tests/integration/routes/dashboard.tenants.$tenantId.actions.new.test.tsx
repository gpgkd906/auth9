import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import NewActionPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId.actions.new";
import { ActionTrigger } from "@auth9/core";

// Mock the session module
vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

// Mock the auth9-client module
vi.mock("~/lib/auth9-client", () => ({
  getAuth9Client: vi.fn(() => ({
    actions: mockActionsApi,
  })),
  withTenant: vi.fn(() => ({
    actions: mockActionsApi,
  })),
  getTriggers: vi.fn(() => Promise.resolve({ data: Object.values(ActionTrigger) })),
}));

import { getAccessToken } from "~/services/session.server";

const mockActionsApi = {
  create: vi.fn(),
};

describe("New Action Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader fetches available triggers", async () => {
    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/actions/new"),
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(response).toEqual({
      tenantId: "tenant-1",
      triggers: Object.values(ActionTrigger),
    });
  });

  it("loader throws when tenantId is missing", async () => {
    await expect(
      loader({
        request: new Request("http://localhost/dashboard/tenants//actions/new"),
        params: {},
        context: {},
      })
    ).rejects.toThrow("Tenant ID is required");
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders new action page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      expect(screen.getByText("New Action")).toBeInTheDocument();
    });
    expect(screen.getByText(/create a new authentication flow action/i)).toBeInTheDocument();
  });

  it("renders empty form with default values", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const nameInput = screen.getByLabelText(/name/i) as HTMLInputElement;
      expect(nameInput).toHaveValue("");
    });

    const descInput = screen.getByLabelText(/description/i) as HTMLInputElement;
    expect(descInput).toHaveValue("");

    const orderInput = screen.getByLabelText(/execution order/i) as HTMLInputElement;
    expect(orderInput).toHaveValue(0);

    const timeoutInput = screen.getByLabelText(/timeout/i) as HTMLInputElement;
    expect(timeoutInput).toHaveValue(3000);

    const switches = screen.getAllByRole("switch");
    const enabledSwitch = switches[0];
    const strictModeSwitch = switches[1];
    expect(enabledSwitch).toBeChecked(); // Default is enabled
    expect(strictModeSwitch).not.toBeChecked(); // Default strict mode is off
  });

  it("renders trigger dropdown with all options", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      expect(screen.getByText(/select a trigger/i)).toBeInTheDocument();
    });

    // Open the trigger dropdown - it's the first combobox
    const comboboxes = screen.getAllByRole("combobox");
    await user.click(comboboxes[0]);

    await waitFor(() => {
      expect(screen.getByText("Post Login")).toBeInTheDocument();
      expect(screen.getByText("Pre Registration")).toBeInTheDocument();
      expect(screen.getByText("Post Registration")).toBeInTheDocument();
      expect(screen.getByText("Post Password Change")).toBeInTheDocument();
      expect(screen.getByText("Post Email Verification")).toBeInTheDocument();
      expect(screen.getByText("Pre Token Refresh")).toBeInTheDocument();
    });
  });

  it("renders script template selector", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      expect(screen.getByText("Script Templates")).toBeInTheDocument();
    });
    expect(screen.getByText(/choose a template \(optional\)/i)).toBeInTheDocument();
  });

  it("renders default script placeholder", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const scriptTextarea = screen.getByLabelText(/typescript code/i) as HTMLTextAreaElement;
      expect(scriptTextarea).toHaveValue("// Your TypeScript code here\ncontext;");
    });
  });

  // ============================================================================
  // Template Selection Tests
  // ============================================================================

  it("renders template selector", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      expect(screen.getByText("Script Templates")).toBeInTheDocument();
      expect(screen.getByText(/choose a template \(optional\)/i)).toBeInTheDocument();
    });
  });


  // ============================================================================
  // Form Validation Tests
  // ============================================================================

  it("requires name field", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const nameInput = screen.getByLabelText(/name/i);
      expect(nameInput).toHaveAttribute("required");
    });
  });

  it("requires trigger selection", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const comboboxes = screen.getAllByRole("combobox");
      // First combobox is the trigger selector
      expect(comboboxes[0]).toHaveAttribute("aria-required", "true");
    });
  });

  it("requires script field", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const scriptTextarea = screen.getByLabelText(/typescript code/i);
      expect(scriptTextarea).toHaveAttribute("required");
    });
  });

  it("validates timeout range (100-30000ms)", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const timeoutInput = screen.getByLabelText(/timeout/i);
      expect(timeoutInput).toHaveAttribute("min", "100");
      expect(timeoutInput).toHaveAttribute("max", "30000");
    });
  });

  it("validates execution order is non-negative", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/actions/new",
        Component: NewActionPage,
        loader: () => ({
          tenantId: "tenant-1",
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/actions/new"]} />);

    await waitFor(() => {
      const orderInput = screen.getByLabelText(/execution order/i);
      expect(orderInput).toHaveAttribute("min", "0");
    });
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
      return new Request("http://localhost/dashboard/tenants/tenant-1/actions/new", {
        method: "POST",
        body: formData,
      });
    }

    it("creates action successfully and redirects", async () => {
      mockActionsApi.create.mockResolvedValue({ data: { id: "new-action-id" } });

      const request = createFormRequest({
        name: "My New Action",
        description: "This is a test action",
        trigger_id: ActionTrigger.PostLogin,
        script: "context;",
        enabled: "on",
        execution_order: "0",
        timeout_ms: "3000",
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(mockActionsApi.create).toHaveBeenCalledWith({
        name: "My New Action",
        description: "This is a test action",
        triggerId: ActionTrigger.PostLogin,
        script: "context;",
        enabled: true,
        strictMode: false,
        executionOrder: 0,
        timeoutMs: 3000,
      });

      // Should redirect to detail page
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe(
        "/dashboard/tenants/tenant-1/actions/new-action-id"
      );
    });

    it("handles disabled action", async () => {
      mockActionsApi.create.mockResolvedValue({ data: { id: "new-action-id" } });

      const request = createFormRequest({
        name: "Disabled Action",
        trigger_id: ActionTrigger.PreUserRegistration,
        script: "context;",
        execution_order: "0",
        timeout_ms: "3000",
        // enabled is NOT "on", so it's false
      });

      await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(mockActionsApi.create).toHaveBeenCalledWith(
        expect.objectContaining({ enabled: false })
      );
    });

    it("handles empty description", async () => {
      mockActionsApi.create.mockResolvedValue({ data: { id: "new-action-id" } });

      const request = createFormRequest({
        name: "Test Action",
        description: "",
        trigger_id: ActionTrigger.PostLogin,
        script: "context;",
        enabled: "on",
        execution_order: "0",
        timeout_ms: "3000",
      });

      await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(mockActionsApi.create).toHaveBeenCalledWith(
        expect.objectContaining({ description: undefined })
      );
    });

    it("returns error when tenantId is missing", async () => {
      const request = createFormRequest({
        name: "Test",
        trigger_id: ActionTrigger.PostLogin,
        script: "context;",
        execution_order: "0",
        timeout_ms: "3000",
      });

      const response = await action({
        request,
        params: {},
        context: {},
      });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Tenant ID required" });
    });

    it("returns error when API call fails", async () => {
      mockActionsApi.create.mockRejectedValue(new Error("Script validation failed"));

      const request = createFormRequest({
        name: "Test Action",
        trigger_id: ActionTrigger.PostLogin,
        script: "invalid script",
        enabled: "on",
        execution_order: "0",
        timeout_ms: "3000",
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(response).toEqual({ error: "Script validation failed" });
    });

  });
});
