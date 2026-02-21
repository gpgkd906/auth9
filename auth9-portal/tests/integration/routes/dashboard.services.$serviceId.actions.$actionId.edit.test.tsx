import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import EditActionPage, { loader, action } from "~/routes/dashboard.services.$serviceId.actions.$actionId.edit";
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
  withService: vi.fn(() => ({
    actions: mockActionsApi,
  })),
  getTriggers: vi.fn(() => Promise.resolve({ data: Object.values(ActionTrigger) })),
}));

import { getAccessToken } from "~/services/session.server";

const mockActionsApi = {
  get: vi.fn(),
  update: vi.fn(),
};

const mockAction = {
  id: "action-1",
  serviceId: "service-1",
  name: "Add Custom Claims",
  description: "Adds department and tier claims",
  triggerId: ActionTrigger.PostLogin,
  script: 'context.claims = { department: "engineering" };\ncontext;',
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

describe("Edit Action Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader fetches action and triggers", async () => {
    mockActionsApi.get.mockResolvedValue({ data: mockAction });

    const response = await loader({
      request: new Request("http://localhost/dashboard/services/service-1/actions/action-1/edit"),
      params: { serviceId: "service-1", actionId: "action-1" },
      context: {},
    });

    expect(response).toEqual({
      serviceId: "service-1",
      action: mockAction,
      triggers: Object.values(ActionTrigger),
    });
    expect(mockActionsApi.get).toHaveBeenCalledWith("action-1");
  });

  it("loader throws when serviceId is missing", async () => {
    await expect(
      loader({
        request: new Request("http://localhost/dashboard/services//actions/action-1/edit"),
        params: { actionId: "action-1" },
        context: {},
      })
    ).rejects.toThrow("Service ID and Action ID are required");
  });

  it("loader throws when actionId is missing", async () => {
    await expect(
      loader({
        request: new Request("http://localhost/dashboard/services/service-1/actions//edit"),
        params: { serviceId: "service-1" },
        context: {},
      })
    ).rejects.toThrow("Service ID and Action ID are required");
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders edit page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      expect(screen.getByText("Edit Action")).toBeInTheDocument();
    });
    expect(screen.getByText("Add Custom Claims")).toBeInTheDocument();
  });

  it("renders form with pre-filled data", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      const nameInput = screen.getByLabelText(/name/i) as HTMLInputElement;
      expect(nameInput).toHaveValue("Add Custom Claims");
    });

    const descInput = screen.getByLabelText(/description/i) as HTMLInputElement;
    expect(descInput).toHaveValue("Adds department and tier claims");

    const scriptTextarea = screen.getByLabelText(/typescript code/i) as HTMLTextAreaElement;
    expect(scriptTextarea).toHaveValue('context.claims = { department: "engineering" };\ncontext;');

    const orderInput = screen.getByLabelText(/execution order/i) as HTMLInputElement;
    expect(orderInput).toHaveValue(0);

    const timeoutInput = screen.getByLabelText(/timeout/i) as HTMLInputElement;
    expect(timeoutInput).toHaveValue(3000);

    const switches = screen.getAllByRole("switch");
    const enabledSwitch = switches[0];
    const strictModeSwitch = switches[1];
    expect(enabledSwitch).toBeChecked();
    expect(strictModeSwitch).not.toBeChecked();
  });

  it("renders trigger as read-only", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      expect(screen.getByText("Post Login")).toBeInTheDocument();
    });
    expect(screen.getByText(/trigger cannot be changed after creation/i)).toBeInTheDocument();
  });

  it("renders execution statistics", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      expect(screen.getByText("Execution Statistics")).toBeInTheDocument();
    });
    expect(screen.getByText("100")).toBeInTheDocument(); // Total Executions
    expect(screen.getByText("5")).toBeInTheDocument(); // Errors
  });

  it("renders last error when present", async () => {
    const actionWithError = {
      ...mockAction,
      lastError: "Execution timeout",
    };

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: actionWithError,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      expect(screen.getByText("Last Error")).toBeInTheDocument();
    });
    expect(screen.getByText("Execution timeout")).toBeInTheDocument();
  });

  // ============================================================================
  // Form Validation Tests
  // ============================================================================

  it("requires name field", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      const nameInput = screen.getByLabelText(/name/i);
      expect(nameInput).toHaveAttribute("required");
    });
  });

  it("requires script field", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      const scriptTextarea = screen.getByLabelText(/typescript code/i);
      expect(scriptTextarea).toHaveAttribute("required");
    });
  });

  it("validates timeout range (100-30000ms)", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

    await waitFor(() => {
      const timeoutInput = screen.getByLabelText(/timeout/i);
      expect(timeoutInput).toHaveAttribute("min", "100");
      expect(timeoutInput).toHaveAttribute("max", "30000");
    });
  });

  it("validates execution order is non-negative", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/services/:serviceId/actions/:actionId/edit",
        Component: EditActionPage,
        loader: () => ({
          serviceId: "service-1",
          action: mockAction,
          triggers: Object.values(ActionTrigger),
        }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

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
      return new Request("http://localhost/dashboard/services/service-1/actions/action-1/edit", {
        method: "POST",
        body: formData,
      });
    }

    it("updates action successfully", async () => {
      mockActionsApi.update.mockResolvedValue({ data: { ...mockAction, name: "Updated Action" } });

      const request = createFormRequest({
        name: "Updated Action",
        description: "Updated description",
        script: 'context.claims = { role: "admin" }; context;',
        enabled: "on",
        execution_order: "1",
        timeout_ms: "5000",
      });

      const response = await action({
        request,
        params: { serviceId: "service-1", actionId: "action-1" },
        context: {},
      });

      expect(mockActionsApi.update).toHaveBeenCalledWith("action-1", {
        name: "Updated Action",
        description: "Updated description",
        script: 'context.claims = { role: "admin" }; context;',
        enabled: true,
        strictMode: false,
        executionOrder: 1,
        timeoutMs: 5000,
      });

      // Should redirect
      expect(response).toBeInstanceOf(Response);
      expect((response as Response).status).toBe(302);
      expect((response as Response).headers.get("Location")).toBe(
        "/dashboard/services/service-1/actions/action-1"
      );
    });

    it("handles disabled action", async () => {
      mockActionsApi.update.mockResolvedValue({ data: mockAction });

      const request = createFormRequest({
        name: "Test Action",
        script: "context;",
        execution_order: "0",
        timeout_ms: "3000",
        // enabled is NOT "on", so it's false
      });

      await action({
        request,
        params: { serviceId: "service-1", actionId: "action-1" },
        context: {},
      });

      expect(mockActionsApi.update).toHaveBeenCalledWith(
        "action-1",
        expect.objectContaining({ enabled: false })
      );
    });

    it("handles empty description", async () => {
      mockActionsApi.update.mockResolvedValue({ data: mockAction });

      const request = createFormRequest({
        name: "Test Action",
        description: "",
        script: "context;",
        enabled: "on",
        execution_order: "0",
        timeout_ms: "3000",
      });

      await action({
        request,
        params: { serviceId: "service-1", actionId: "action-1" },
        context: {},
      });

      expect(mockActionsApi.update).toHaveBeenCalledWith(
        "action-1",
        expect.objectContaining({ description: undefined })
      );
    });

    it("returns error when serviceId is missing", async () => {
      const request = createFormRequest({
        name: "Test",
        script: "context;",
        execution_order: "0",
        timeout_ms: "3000",
      });

      const response = await action({
        request,
        params: { actionId: "action-1" },
        context: {},
      });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "IDs required" });
    });

    it("returns error when actionId is missing", async () => {
      const request = createFormRequest({
        name: "Test",
        script: "context;",
        execution_order: "0",
        timeout_ms: "3000",
      });

      const response = await action({
        request,
        params: { serviceId: "service-1" },
        context: {},
      });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "IDs required" });
    });

    it("returns error when API call fails", async () => {
      mockActionsApi.update.mockRejectedValue(new Error("Validation failed"));

      const request = createFormRequest({
        name: "Test Action",
        script: "invalid script",
        enabled: "on",
        execution_order: "0",
        timeout_ms: "3000",
      });

      const response = await action({
        request,
        params: { serviceId: "service-1", actionId: "action-1" },
        context: {},
      });

      expect(response).toEqual({ error: "Validation failed" });
    });

    it("displays error message on page when action fails", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/services/:serviceId/actions/:actionId/edit",
          Component: EditActionPage,
          loader: () => ({
            serviceId: "service-1",
            action: mockAction,
            triggers: Object.values(ActionTrigger),
          }),
          action: () => ({ error: "Update failed" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/services/service-1/actions/action-1/edit"]} />);

      await waitFor(() => {
        expect(screen.getByText("Edit Action")).toBeInTheDocument();
      });

      const submitButton = screen.getByRole("button", { name: /save changes/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(screen.getByText("Update failed")).toBeInTheDocument();
      });
    });
  });
});
