import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import WebhooksPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId.webhooks";
import { webhookApi, tenantApi } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";

// Mock the session module
vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue(null),
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

import { getAccessToken } from "~/services/session.server";

// Mock the API
vi.mock("~/services/api", () => ({
  webhookApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    test: vi.fn(),
    regenerateSecret: vi.fn(),
  },
  tenantApi: {
    get: vi.fn(),
  },
}));

const mockWebhook = {
  id: "webhook-1",
  name: "Login Notifications",
  url: "https://example.com/webhook",
  secret: "my-secret",
  events: ["login.success", "login.failed"],
  enabled: true,
  failure_count: 0,
  last_triggered_at: "2024-01-15T10:00:00Z",
};

const mockWebhookDisabled = {
  id: "webhook-2",
  name: "Audit Webhook",
  url: "https://audit.example.com/hook",
  secret: null,
  events: ["user.created", "user.deleted"],
  enabled: false,
  failure_count: 3,
  last_triggered_at: null,
};

function WrappedPage() {
  return (
    <ConfirmProvider>
      <WebhooksPage />
    </ConfirmProvider>
  );
}

describe("Webhooks Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tenantApi.get).mockResolvedValue({ data: { id: "tenant-1", name: "Test" } } as ReturnType<typeof tenantApi.get>);
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns webhooks from API", async () => {
    vi.mocked(webhookApi.list).mockResolvedValue({
      data: [mockWebhook, mockWebhookDisabled],
    });

    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/webhooks"),
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(response).toEqual({
      webhooks: [mockWebhook, mockWebhookDisabled],
      tenantId: "tenant-1",
    });
    expect(webhookApi.list).toHaveBeenCalledWith("tenant-1", undefined);
  });

  it("loader returns error when tenantId is missing", async () => {
    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants//webhooks"),
      params: {},
      context: {},
    });

    expect(response).toEqual({
      webhooks: [],
      error: "Tenant ID is required",
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders webhooks page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByText("Webhooks")).toBeInTheDocument();
    });
    expect(
      screen.getByText(/receive real-time notifications for events/i)
    ).toBeInTheDocument();
  });

  it("renders add webhook button", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /add webhook/i })).toBeInTheDocument();
    });
  });

  it("renders empty state when no webhooks", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByText("No webhooks configured")).toBeInTheDocument();
    });
    expect(
      screen.getByText(/add a webhook to receive real-time event notifications/i)
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
  });

  it("renders webhook list with status indicators", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [mockWebhook, mockWebhookDisabled], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByText("Login Notifications")).toBeInTheDocument();
    });
    expect(screen.getByText("Audit Webhook")).toBeInTheDocument();
    expect(screen.getByText("https://example.com/webhook")).toBeInTheDocument();
    expect(screen.getByText("https://audit.example.com/hook")).toBeInTheDocument();
  });

  it("renders event count and failure count", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [mockWebhook, mockWebhookDisabled], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      // Both webhooks have 2 events each
      expect(screen.getAllByText(/2 events/).length).toBe(2);
    });
    expect(screen.getByText(/3 failures/)).toBeInTheDocument();
  });

  it("renders test button for enabled webhooks", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      const testButton = screen.getByRole("button", { name: /test/i });
      expect(testButton).toBeInTheDocument();
      expect(testButton).not.toBeDisabled();
    });
  });

  it("renders last triggered timestamp", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByText(/last triggered:/i)).toBeInTheDocument();
    });
  });

  it("renders 'Never triggered' for new webhooks", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WrappedPage,
        loader: () => ({ webhooks: [mockWebhookDisabled], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByText(/never triggered/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Component Interaction Tests
  // ============================================================================

  describe("create webhook dialog", () => {
    it("opens create dialog when 'Add webhook' header button is clicked", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add Webhook")).toBeInTheDocument();
        expect(screen.getByText("Configure a new webhook endpoint.")).toBeInTheDocument();
      });
    });

    it("opens create dialog when 'Add your first webhook' button is clicked in empty state", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("No webhooks configured")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add Webhook")).toBeInTheDocument();
      });
    });

    it("shows empty form fields in create dialog", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const nameInput = screen.getByLabelText("Name");
      const urlInput = screen.getByLabelText("Endpoint URL");
      const secretInput = screen.getByLabelText("Secret (optional)");

      expect(nameInput).toHaveValue("");
      expect(urlInput).toHaveValue("");
      expect(secretInput).toHaveValue("");
    });

    it("allows typing in name, url, and secret fields", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const nameInput = screen.getByLabelText("Name");
      const urlInput = screen.getByLabelText("Endpoint URL");
      const secretInput = screen.getByLabelText("Secret (optional)");

      await user.type(nameInput, "My Webhook");
      await user.type(urlInput, "https://example.com/hook");
      await user.type(secretInput, "my-secret");

      expect(nameInput).toHaveValue("My Webhook");
      expect(urlInput).toHaveValue("https://example.com/hook");
      expect(secretInput).toHaveValue("my-secret");
    });

    it("toggles event checkboxes", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // All checkboxes start unchecked
      const loginSuccessCheckbox = screen.getByRole("checkbox", { name: /login success/i });
      const userCreatedCheckbox = screen.getByRole("checkbox", { name: /user created/i });
      expect(loginSuccessCheckbox).not.toBeChecked();
      expect(userCreatedCheckbox).not.toBeChecked();

      // Toggle on
      await user.click(loginSuccessCheckbox);
      expect(loginSuccessCheckbox).toBeChecked();

      await user.click(userCreatedCheckbox);
      expect(userCreatedCheckbox).toBeChecked();

      // Toggle off
      await user.click(loginSuccessCheckbox);
      expect(loginSuccessCheckbox).not.toBeChecked();
      expect(userCreatedCheckbox).toBeChecked();
    });

    it("closes create dialog when Cancel button is clicked", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /cancel/i }));

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });

    it("submits create form via the dialog", async () => {
      vi.mocked(webhookApi.create).mockResolvedValue({ data: mockWebhook });

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.type(screen.getByLabelText("Name"), "New Hook");
      await user.type(screen.getByLabelText("Endpoint URL"), "https://new.example.com/hook");

      // Select at least one event so submit button is enabled
      await user.click(screen.getByRole("checkbox", { name: /login success/i }));

      const addButton = screen.getByRole("button", { name: /add webhook$/i });
      expect(addButton).not.toBeDisabled();

      await user.click(addButton);

      await waitFor(() => {
        expect(webhookApi.create).toHaveBeenCalled();
      });
    });

    it("disables submit button when no events are selected", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // No events selected, submit should be disabled
      const addButton = screen.getByRole("button", { name: /add webhook$/i });
      expect(addButton).toBeDisabled();
    });
  });

  describe("edit webhook dialog", () => {
    it("opens edit dialog with pre-filled data when edit button is clicked", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      // Find the edit button (pencil icon)
      const editButtons = screen.getAllByRole("button");
      // The edit button contains a Pencil2Icon, look for it among the action buttons
      // The edit button is the one after regenerate
      // The button order in each webhook row: Test, Regenerate Secret (title), Edit (no title), Delete (red)
      // Let's just click the 4th button-like element in the action area

      // Actually let's find all ghost buttons and pick the edit one.
      // Test button is variant="outline", Regenerate has title="Regenerate Secret"
      // Edit and Delete are both ghost variant
      // Let's find buttons by their SVG content or position
      // The simplest approach: the pencil icon button has no title attribute and is not red
      const pencilButton = editButtons.find(
        (btn) => !btn.textContent?.trim() && !btn.getAttribute("title") && !btn.className.includes("accent-red") && btn.closest(".flex.items-center.gap-2")
      );

      if (pencilButton) {
        await user.click(pencilButton);
      }

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Edit Webhook")).toBeInTheDocument();
        expect(screen.getByText("Update the webhook configuration.")).toBeInTheDocument();
      });

      // Verify pre-filled form data
      expect(screen.getByLabelText("Name")).toHaveValue("Login Notifications");
      expect(screen.getByLabelText("Endpoint URL")).toHaveValue("https://example.com/webhook");
      expect(screen.getByLabelText("Secret (optional)")).toHaveValue("my-secret");

      // Verify selected events
      expect(screen.getByRole("checkbox", { name: /login success/i })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: /login failed/i })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: /user created/i })).not.toBeChecked();
    });

    it("shows 'Save changes' button text in edit mode", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      // Click edit
      const editButtons = screen.getAllByRole("button");
      const pencilButton = editButtons.find(
        (btn) => !btn.textContent?.trim() && !btn.getAttribute("title") && !btn.className.includes("accent-red") && btn.closest(".flex.items-center.gap-2")
      );
      if (pencilButton) await user.click(pencilButton);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      expect(screen.getByRole("button", { name: /save changes/i })).toBeInTheDocument();
    });

    it("submits update form via edit dialog", async () => {
      vi.mocked(webhookApi.update).mockResolvedValue({ data: mockWebhook });

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      // Click edit
      const editButtons = screen.getAllByRole("button");
      const pencilButton = editButtons.find(
        (btn) => !btn.textContent?.trim() && !btn.getAttribute("title") && !btn.className.includes("accent-red") && btn.closest(".flex.items-center.gap-2")
      );
      if (pencilButton) await user.click(pencilButton);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Modify the name
      const nameInput = screen.getByLabelText("Name");
      await user.clear(nameInput);
      await user.type(nameInput, "Updated Webhook");

      await user.click(screen.getByRole("button", { name: /save changes/i }));

      await waitFor(() => {
        expect(webhookApi.update).toHaveBeenCalled();
      });
    });

    it("closes edit dialog when Cancel button is clicked and resets form", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      // Click edit
      const editButtons = screen.getAllByRole("button");
      const pencilButton = editButtons.find(
        (btn) => !btn.textContent?.trim() && !btn.getAttribute("title") && !btn.className.includes("accent-red") && btn.closest(".flex.items-center.gap-2")
      );
      if (pencilButton) await user.click(pencilButton);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: /cancel/i }));

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });
  });

  describe("delete webhook", () => {
    it("shows confirm dialog when delete button is clicked", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      // The delete button has the accent-red class in its className
      const allButtons = screen.getAllByRole("button");
      const deleteButton = allButtons.find(
        (btn) => btn.className.includes("accent-red")
      );
      expect(deleteButton).toBeDefined();
      await user.click(deleteButton!);

      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Webhook");
      });
      expect(screen.getByText(/are you sure you want to delete this webhook/i)).toBeInTheDocument();
    });

    it("submits delete after confirming", async () => {
      vi.mocked(webhookApi.delete).mockResolvedValue(undefined);

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const allButtons = screen.getAllByRole("button");
      const deleteButton = allButtons.find(
        (btn) => btn.className.includes("accent-red")
      );
      await user.click(deleteButton!);

      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Webhook");
      });

      // Confirm deletion
      await user.click(screen.getByTestId("confirm-dialog-action"));

      await waitFor(() => {
        expect(webhookApi.delete).toHaveBeenCalledWith("tenant-1", "webhook-1", undefined);
      });
    });

    it("does not submit delete when cancel is clicked on confirm dialog", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const allButtons = screen.getAllByRole("button");
      const deleteButton = allButtons.find(
        (btn) => btn.className.includes("accent-red")
      );
      await user.click(deleteButton!);

      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Webhook");
      });

      // Cancel deletion
      await user.click(screen.getByTestId("confirm-dialog-cancel"));

      await waitFor(() => {
        expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
      });

      expect(webhookApi.delete).not.toHaveBeenCalled();
    });
  });

  describe("regenerate secret", () => {
    it("shows confirm dialog when regenerate secret button is clicked", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const regenerateButton = screen.getByRole("button", { name: /regenerate secret/i });
      await user.click(regenerateButton);

      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Regenerate Secret");
      });
      expect(screen.getByText(/the old secret will be invalidated immediately/i)).toBeInTheDocument();
    });

    it("submits regenerate secret after confirming", async () => {
      vi.mocked(webhookApi.regenerateSecret).mockResolvedValue({
        data: { ...mockWebhook, secret: "whsec_new" },
      });

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const regenerateButton = screen.getByRole("button", { name: /regenerate secret/i });
      await user.click(regenerateButton);

      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Regenerate Secret");
      });

      await user.click(screen.getByTestId("confirm-dialog-action"));

      await waitFor(() => {
        expect(webhookApi.regenerateSecret).toHaveBeenCalledWith("tenant-1", "webhook-1", undefined);
      });
    });

    it("does not submit regenerate secret when cancel is clicked", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const regenerateButton = screen.getByRole("button", { name: /regenerate secret/i });
      await user.click(regenerateButton);

      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Regenerate Secret");
      });

      await user.click(screen.getByTestId("confirm-dialog-cancel"));

      await waitFor(() => {
        expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
      });

      expect(webhookApi.regenerateSecret).not.toHaveBeenCalled();
    });
  });

  describe("test webhook button", () => {
    it("submits test form when test button is clicked", async () => {
      vi.mocked(webhookApi.test).mockResolvedValue({
        data: { success: true, status_code: 200, response_time_ms: 100 },
      });

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const testButton = screen.getByRole("button", { name: /^test$/i });
      await user.click(testButton);

      await waitFor(() => {
        expect(webhookApi.test).toHaveBeenCalledWith("tenant-1", "webhook-1", undefined);
      });
    });

    it("disables test button for disabled webhooks", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhookDisabled], tenantId: "tenant-1" }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Audit Webhook")).toBeInTheDocument();
      });

      const testButton = screen.getByRole("button", { name: /^test$/i });
      expect(testButton).toBeDisabled();
    });
  });

  describe("action data messages", () => {
    it("renders success message from action data", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action: () => ({ success: true, message: "Webhook created" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      // Submit the test form to trigger the action
      const testButton = screen.getByRole("button", { name: /^test$/i });
      await user.click(testButton);

      await waitFor(() => {
        expect(screen.getByText("Webhook created")).toBeInTheDocument();
      });
    });

    it("renders error message from action data", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [mockWebhook], tenantId: "tenant-1" }),
          action: () => ({ error: "Something went wrong" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Login Notifications")).toBeInTheDocument();
      });

      const testButton = screen.getByRole("button", { name: /^test$/i });
      await user.click(testButton);

      await waitFor(() => {
        expect(screen.getByText("Something went wrong")).toBeInTheDocument();
      });
    });

    it("renders load error message", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], error: "Tenant ID is required", tenantId: undefined }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByText("Tenant ID is required")).toBeInTheDocument();
      });
    });
  });

  describe("enabled switch toggle", () => {
    it("toggles enabled switch in create dialog", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/webhooks",
          Component: WrappedPage,
          loader: () => ({ webhooks: [], tenantId: "tenant-1" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: /add your first webhook/i })).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /add your first webhook/i }));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const enabledSwitch = screen.getByRole("switch");
      // Default is enabled (checked)
      expect(enabledSwitch).toHaveAttribute("data-state", "checked");

      // Toggle off
      await user.click(enabledSwitch);
      expect(enabledSwitch).toHaveAttribute("data-state", "unchecked");

      // Toggle back on
      await user.click(enabledSwitch);
      expect(enabledSwitch).toHaveAttribute("data-state", "checked");
    });
  });

  // ============================================================================
  // Action Tests
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
      return new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });
    }

    // ------------------------------------------------------------------
    // Create intent
    // ------------------------------------------------------------------

    it("creates webhook with all fields", async () => {
      vi.mocked(webhookApi.create).mockResolvedValue({ data: mockWebhook });

      const formData = new FormData();
      formData.append("intent", "create");
      formData.append("name", "New Webhook");
      formData.append("url", "https://new.example.com/hook");
      formData.append("secret", "secret123");
      formData.append("events", JSON.stringify(["login.success", "user.created"]));
      formData.append("enabled", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.create).toHaveBeenCalledWith(
        "tenant-1",
        {
          name: "New Webhook",
          url: "https://new.example.com/hook",
          secret: "secret123",
          events: ["login.success", "user.created"],
          enabled: true,
        },
        "test-token",
      );
      expect(response).toEqual({ success: true, message: "Webhook created" });
    });

    it("creates webhook without secret", async () => {
      vi.mocked(webhookApi.create).mockResolvedValue({ data: mockWebhook });

      const formData = new FormData();
      formData.append("intent", "create");
      formData.append("name", "No Secret Hook");
      formData.append("url", "https://nosecret.example.com/hook");
      formData.append("events", JSON.stringify(["user.deleted"]));
      formData.append("enabled", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.create).toHaveBeenCalledWith(
        "tenant-1",
        expect.objectContaining({ secret: undefined }),
        "test-token",
      );
      expect(response).toEqual({ success: true, message: "Webhook created" });
    });

    it("creates webhook with enabled=false", async () => {
      vi.mocked(webhookApi.create).mockResolvedValue({ data: mockWebhookDisabled });

      const formData = new FormData();
      formData.append("intent", "create");
      formData.append("name", "Disabled Hook");
      formData.append("url", "https://disabled.example.com/hook");
      formData.append("events", JSON.stringify(["security.alert"]));
      formData.append("enabled", "false");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.create).toHaveBeenCalledWith(
        "tenant-1",
        expect.objectContaining({ enabled: false }),
        "test-token",
      );
      expect(response).toEqual({ success: true, message: "Webhook created" });
    });

    it("returns error when create API call fails", async () => {
      vi.mocked(webhookApi.create).mockRejectedValue(new Error("Network error"));

      const request = createFormRequest({
        intent: "create",
        name: "Fail Hook",
        url: "https://fail.example.com",
        events: JSON.stringify(["login.success"]),
        enabled: "true",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Network error" });
    });

    // ------------------------------------------------------------------
    // Update intent
    // ------------------------------------------------------------------

    it("updates webhook with all fields", async () => {
      vi.mocked(webhookApi.update).mockResolvedValue({ data: mockWebhook });

      const formData = new FormData();
      formData.append("intent", "update");
      formData.append("id", "webhook-1");
      formData.append("name", "Updated Webhook");
      formData.append("url", "https://updated.example.com/hook");
      formData.append("secret", "new-secret");
      formData.append("events", JSON.stringify(["login.failed", "user.updated"]));
      formData.append("enabled", "false");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.update).toHaveBeenCalledWith(
        "tenant-1",
        "webhook-1",
        {
          name: "Updated Webhook",
          url: "https://updated.example.com/hook",
          secret: "new-secret",
          events: ["login.failed", "user.updated"],
          enabled: false,
        },
        "test-token",
      );
      expect(response).toEqual({ success: true, message: "Webhook updated" });
    });

    it("updates webhook without secret", async () => {
      vi.mocked(webhookApi.update).mockResolvedValue({ data: mockWebhook });

      const formData = new FormData();
      formData.append("intent", "update");
      formData.append("id", "webhook-1");
      formData.append("name", "Updated No Secret");
      formData.append("url", "https://updated.example.com/hook");
      formData.append("events", JSON.stringify(["login.success"]));
      formData.append("enabled", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.update).toHaveBeenCalledWith(
        "tenant-1",
        "webhook-1",
        expect.objectContaining({ secret: undefined }),
        "test-token",
      );
      expect(response).toEqual({ success: true, message: "Webhook updated" });
    });

    it("returns error when update API call fails", async () => {
      vi.mocked(webhookApi.update).mockRejectedValue(new Error("Webhook not found"));

      const request = createFormRequest({
        intent: "update",
        id: "webhook-999",
        name: "Updated",
        url: "https://updated.example.com",
        events: JSON.stringify(["login.success"]),
        enabled: "true",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Webhook not found" });
    });

    // ------------------------------------------------------------------
    // Delete intent
    // ------------------------------------------------------------------

    it("deletes webhook", async () => {
      vi.mocked(webhookApi.delete).mockResolvedValue(undefined);

      const request = createFormRequest({
        intent: "delete",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.delete).toHaveBeenCalledWith("tenant-1", "webhook-1", "test-token");
      expect(response).toEqual({ success: true, message: "Webhook deleted" });
    });

    it("returns error when delete API call fails", async () => {
      vi.mocked(webhookApi.delete).mockRejectedValue(new Error("Forbidden"));

      const request = createFormRequest({
        intent: "delete",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Forbidden" });
    });

    // ------------------------------------------------------------------
    // Regenerate secret intent
    // ------------------------------------------------------------------

    it("regenerates webhook secret", async () => {
      vi.mocked(webhookApi.regenerateSecret).mockResolvedValue({
        data: { ...mockWebhook, secret: "whsec_new_secret" },
      });

      const request = createFormRequest({
        intent: "regenerate_secret",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.regenerateSecret).toHaveBeenCalledWith("tenant-1", "webhook-1", "test-token");
      expect(response).toEqual({ success: true, message: "Secret regenerated", newSecret: "whsec_new_secret" });
    });

    it("returns error when regenerate secret API call fails", async () => {
      vi.mocked(webhookApi.regenerateSecret).mockRejectedValue(new Error("Internal server error"));

      const request = createFormRequest({
        intent: "regenerate_secret",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Internal server error" });
    });

    // ------------------------------------------------------------------
    // Test intent
    // ------------------------------------------------------------------

    it("tests webhook successfully", async () => {
      vi.mocked(webhookApi.test).mockResolvedValue({
        data: { success: true, status_code: 200, response_time_ms: 150 },
      });

      const request = createFormRequest({
        intent: "test",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.test).toHaveBeenCalledWith("tenant-1", "webhook-1", "test-token");
      expect(response).toEqual({ success: true, message: "Test successful (200, 150ms)" });
    });

    it("returns error on test failure response", async () => {
      vi.mocked(webhookApi.test).mockResolvedValue({
        data: { success: false, error: "Connection refused" },
      });

      const request = createFormRequest({
        intent: "test",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Test failed: Connection refused" });
    });

    it("returns error when test API call throws", async () => {
      vi.mocked(webhookApi.test).mockRejectedValue(new Error("Timeout"));

      const request = createFormRequest({
        intent: "test",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Timeout" });
    });

    // ------------------------------------------------------------------
    // Validation / edge cases
    // ------------------------------------------------------------------

    it("returns error for missing tenantId", async () => {
      const formData = new FormData();
      formData.append("intent", "create");

      const request = new Request("http://localhost/dashboard/tenants//webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });

      expect(response).toEqual({ error: "Tenant ID is required" });
    });

    it("returns error for invalid intent", async () => {
      const request = createFormRequest({ intent: "invalid" });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Invalid action" });
    });

    it("returns error for missing intent", async () => {
      const formData = new FormData();
      const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Invalid action" });
    });

    it("passes undefined token when getAccessToken returns null", async () => {
      vi.mocked(getAccessToken).mockResolvedValueOnce(null);
      vi.mocked(webhookApi.delete).mockResolvedValue(undefined);

      const request = createFormRequest({
        intent: "delete",
        id: "webhook-1",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.delete).toHaveBeenCalledWith("tenant-1", "webhook-1", undefined);
      expect(response).toEqual({ success: true, message: "Webhook deleted" });
    });

    it("handles non-Error exception in catch block", async () => {
      vi.mocked(webhookApi.create).mockRejectedValue("string error");

      const request = createFormRequest({
        intent: "create",
        name: "Hook",
        url: "https://example.com",
        events: JSON.stringify(["login.success"]),
        enabled: "true",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(response).toEqual({ error: "Operation failed" });
    });

    it("creates webhook with empty events JSON array", async () => {
      vi.mocked(webhookApi.create).mockResolvedValue({ data: mockWebhook });

      const request = createFormRequest({
        intent: "create",
        name: "Empty Events Hook",
        url: "https://example.com/hook",
        events: JSON.stringify([]),
        enabled: "true",
      });

      const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

      expect(webhookApi.create).toHaveBeenCalledWith(
        "tenant-1",
        expect.objectContaining({ events: [] }),
        "test-token",
      );
      expect(response).toEqual({ success: true, message: "Webhook created" });
    });
  });
});
