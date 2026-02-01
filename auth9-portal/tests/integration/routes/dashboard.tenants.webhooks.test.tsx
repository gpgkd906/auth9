import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import WebhooksPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId.webhooks";
import { webhookApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  webhookApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
    test: vi.fn(),
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

describe("Webhooks Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
    expect(webhookApi.list).toHaveBeenCalledWith("tenant-1");
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

  it("loader returns error on API failure", async () => {
    vi.mocked(webhookApi.list).mockRejectedValue(new Error("API Error"));

    const response = await loader({
      request: new Request("http://localhost/dashboard/tenants/tenant-1/webhooks"),
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(response).toEqual({
      webhooks: [],
      tenantId: "tenant-1",
      error: "Failed to load webhooks",
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders webhooks page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/webhooks",
        Component: WebhooksPage,
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
        Component: WebhooksPage,
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
        Component: WebhooksPage,
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
        Component: WebhooksPage,
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
        Component: WebhooksPage,
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
        Component: WebhooksPage,
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
        Component: WebhooksPage,
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
        Component: WebhooksPage,
        loader: () => ({ webhooks: [mockWebhookDisabled], tenantId: "tenant-1" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/webhooks"]} />);

    await waitFor(() => {
      expect(screen.getByText(/never triggered/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action creates webhook", async () => {
    vi.mocked(webhookApi.create).mockResolvedValue({ data: mockWebhook });

    const formData = new FormData();
    formData.append("intent", "create");
    formData.append("name", "New Webhook");
    formData.append("url", "https://new.example.com/hook");
    formData.append("secret", "secret123");
    formData.append("events", JSON.stringify(["login.success"]));
    formData.append("enabled", "true");

    const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

    expect(webhookApi.create).toHaveBeenCalledWith("tenant-1", {
      name: "New Webhook",
      url: "https://new.example.com/hook",
      secret: "secret123",
      events: ["login.success"],
      enabled: true,
    });
    expect(response).toEqual({ success: true, message: "Webhook created" });
  });

  it("action updates webhook", async () => {
    vi.mocked(webhookApi.update).mockResolvedValue({ data: mockWebhook });

    const formData = new FormData();
    formData.append("intent", "update");
    formData.append("id", "webhook-1");
    formData.append("name", "Updated Webhook");
    formData.append("url", "https://updated.example.com/hook");
    formData.append("events", JSON.stringify(["login.failed"]));
    formData.append("enabled", "false");

    const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

    expect(webhookApi.update).toHaveBeenCalledWith("tenant-1", "webhook-1", expect.any(Object));
    expect(response).toEqual({ success: true, message: "Webhook updated" });
  });

  it("action deletes webhook", async () => {
    vi.mocked(webhookApi.delete).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("intent", "delete");
    formData.append("id", "webhook-1");

    const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

    expect(webhookApi.delete).toHaveBeenCalledWith("tenant-1", "webhook-1");
    expect(response).toEqual({ success: true, message: "Webhook deleted" });
  });

  it("action tests webhook successfully", async () => {
    vi.mocked(webhookApi.test).mockResolvedValue({
      data: { success: true, status_code: 200, response_time_ms: 150 },
    });

    const formData = new FormData();
    formData.append("intent", "test");
    formData.append("id", "webhook-1");

    const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

    expect(webhookApi.test).toHaveBeenCalledWith("tenant-1", "webhook-1");
    expect(response).toEqual({ success: true, message: "Test successful (200, 150ms)" });
  });

  it("action returns error on test failure", async () => {
    vi.mocked(webhookApi.test).mockResolvedValue({
      data: { success: false, error: "Connection refused" },
    });

    const formData = new FormData();
    formData.append("intent", "test");
    formData.append("id", "webhook-1");

    const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

    expect(response).toEqual({ error: "Test failed: Connection refused" });
  });

  it("action returns error for missing tenantId", async () => {
    const formData = new FormData();
    formData.append("intent", "create");

    const request = new Request("http://localhost/dashboard/tenants//webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Tenant ID is required" });
  });

  it("action returns error for invalid intent", async () => {
    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request("http://localhost/dashboard/tenants/tenant-1/webhooks", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { tenantId: "tenant-1" }, context: {} });

    expect(response).toEqual({ error: "Invalid action" });
  });
});
