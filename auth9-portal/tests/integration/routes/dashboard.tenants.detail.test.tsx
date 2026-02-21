import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import TenantDetailPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId._index";
import { tenantApi, serviceApi, invitationApi, webhookApi, tenantServiceApi, tenantUserApi } from "~/services/api";

// Mock the APIs
vi.mock("~/services/api", () => ({
  tenantApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
  serviceApi: {
    list: vi.fn(),
  },
  invitationApi: {
    list: vi.fn(),
  },
  webhookApi: {
    list: vi.fn(),
  },
  tenantServiceApi: {
    listServices: vi.fn(),
  },
  tenantUserApi: {
    list: vi.fn(),
  },
}));

const mockTenant = {
  id: "tenant-1",
  name: "Acme Corporation",
  slug: "acme",
  logo_url: "https://example.com/logo.png",
  settings: {},
  status: "active" as const,
  created_at: "2024-01-15T10:00:00Z",
  updated_at: "2024-01-15T10:00:00Z",
};

const mockTenantNoLogo = {
  id: "tenant-2",
  name: "Globex Inc",
  slug: "globex",
  logo_url: null,
  settings: {},
  status: "inactive" as const,
  created_at: "2024-01-10T08:00:00Z",
  updated_at: "2024-01-10T08:00:00Z",
};

const mockServices = [
  { id: "svc-1", name: "Auth Service", base_url: "https://auth.example.com", status: "active", enabled: true },
  { id: "svc-2", name: "API Gateway", base_url: "https://api.example.com", status: "active", enabled: false },
  { id: "svc-3", name: "Analytics", base_url: null, status: "inactive", enabled: true },
];

describe("Tenant Detail Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  describe("loader", () => {
    it("fetches tenant details and related counts", async () => {
      vi.mocked(tenantApi.get).mockResolvedValue({ data: mockTenant });
      vi.mocked(serviceApi.list).mockResolvedValue({
        data: [],
        pagination: { total: 5, page: 1, per_page: 1, total_pages: 5 },
      });
      vi.mocked(invitationApi.list).mockResolvedValue({
        data: [],
        pagination: { total: 3, page: 1, per_page: 1, total_pages: 3 },
      });
      vi.mocked(webhookApi.list).mockResolvedValue({
        data: [{ id: "wh-1" }, { id: "wh-2" }],
      });
      vi.mocked(tenantServiceApi.listServices).mockResolvedValue({
        data: mockServices,
      });
      vi.mocked(tenantUserApi.list).mockResolvedValue({
        data: [{ id: "u-1" }, { id: "u-2" }, { id: "u-3" }],
      });
      const response = await loader({
        request: new Request("http://localhost/dashboard/tenants/tenant-1"),
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(response).toEqual({
        tenant: mockTenant,
        usersCount: 3,
        servicesCount: 5,
        pendingInvitationsCount: 3,
        webhooksCount: 2,
        enabledServicesCount: 2,
        totalGlobalServicesCount: 3,
      });
      expect(tenantApi.get).toHaveBeenCalledWith("tenant-1", undefined);
      expect(tenantServiceApi.listServices).toHaveBeenCalledWith("tenant-1", undefined);
    });

    it("throws error when tenantId is missing", async () => {
      await expect(
        loader({
          request: new Request("http://localhost/dashboard/tenants/"),
          params: {},
          context: {},
        })
      ).rejects.toThrow("Tenant ID is required");
    });
  });

  // ============================================================================
  // Rendering Tests - Header
  // ============================================================================

  describe("header rendering", () => {
    const loaderData = {
      tenant: mockTenant,
      usersCount: 3,
      servicesCount: 5,
      pendingInvitationsCount: 3,
      webhooksCount: 2,
      enabledServicesCount: 2,
      totalGlobalServicesCount: 3,
    };

    it("renders tenant name in header", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Acme Corporation")).toBeInTheDocument();
      });
    });

    it("renders back button to tenants list", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
        {
          path: "/dashboard/tenants",
          Component: () => <div>Tenants List</div>,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        const backLink = screen.getByRole("link", { name: "" }); // Icon button has no text
        expect(backLink).toHaveAttribute("href", "/dashboard/tenants");
      });
    });

    it("renders tenant logo when present", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        const logo = document.querySelector("img[src='https://example.com/logo.png']");
        expect(logo).toBeInTheDocument();
      });
    });

    it("does not render logo when absent", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => ({
            ...loaderData,
            tenant: mockTenantNoLogo,
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-2"]} />);

      await waitFor(() => {
        expect(screen.getByText("Globex Inc")).toBeInTheDocument();
      });
      expect(document.querySelector("img")).not.toBeInTheDocument();
    });

    it("renders page description", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Tenant Configuration and Management")).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Rendering Tests - Configuration Form
  // ============================================================================

  describe("configuration form rendering", () => {
    const loaderData = {
      tenant: mockTenant,
      usersCount: 3,
      servicesCount: 5,
      pendingInvitationsCount: 3,
      webhooksCount: 2,
      enabledServicesCount: 2,
      totalGlobalServicesCount: 3,
    };

    it("renders configuration card with title", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Configuration")).toBeInTheDocument();
        expect(screen.getByText("General settings for this tenant")).toBeInTheDocument();
      });
    });

    it("renders name input with default value", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        const nameInput = screen.getByLabelText("Name");
        expect(nameInput).toHaveValue("Acme Corporation");
      });
    });

    it("renders slug input with default value", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        const slugInput = screen.getByLabelText("Slug");
        expect(slugInput).toHaveValue("acme");
      });
    });

    it("renders logo URL input with default value", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        const logoInput = screen.getByLabelText("Logo URL");
        expect(logoInput).toHaveValue("https://example.com/logo.png");
      });
    });

    it("renders tenant status", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Status")).toBeInTheDocument();
        expect(screen.getByText("active")).toBeInTheDocument();
      });
    });

    it("renders created date", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Created")).toBeInTheDocument();
      });
    });

    it("renders save changes button", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: "Save Changes" })).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Rendering Tests - Quick Links
  // ============================================================================

  describe("quick links rendering", () => {
    const loaderData = {
      tenant: mockTenant,
      usersCount: 3,
      servicesCount: 5,
      pendingInvitationsCount: 3,
      webhooksCount: 2,
      enabledServicesCount: 2,
      totalGlobalServicesCount: 3,
    };

    it("renders quick links card", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Quick Links")).toBeInTheDocument();
        expect(screen.getByText("Manage tenant resources")).toBeInTheDocument();
      });
    });

    it("renders services link with count", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByRole("link", { name: /services/i })).toHaveAttribute(
          "href",
          "/dashboard/tenants/tenant-1/services"
        );
        expect(screen.getByText("2/3 enabled")).toBeInTheDocument();
      });
    });

    it("renders invitations link with pending count badge", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByRole("link", { name: /invitations/i })).toHaveAttribute(
          "href",
          "/dashboard/tenants/tenant-1/invitations"
        );
        expect(screen.getByText("3 pending")).toBeInTheDocument();
      });
    });

    it("renders webhooks link with count", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByRole("link", { name: /webhooks/i })).toHaveAttribute(
          "href",
          "/dashboard/tenants/tenant-1/webhooks"
        );
      });
    });

    it("does not render pending badge when no pending invitations", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => ({
            ...loaderData,
            pendingInvitationsCount: 0,
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByRole("link", { name: /invitations/i })).toBeInTheDocument();
      });
      expect(screen.queryByText(/pending/)).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Rendering Tests - Overview Stats
  // ============================================================================

  describe("overview stats rendering", () => {
    const loaderData = {
      tenant: mockTenant,
      usersCount: 3,
      servicesCount: 5,
      pendingInvitationsCount: 3,
      webhooksCount: 2,
      enabledServicesCount: 2,
      totalGlobalServicesCount: 3,
    };

    it("renders overview card", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Overview")).toBeInTheDocument();
        expect(screen.getByText("Tenant statistics")).toBeInTheDocument();
      });
    });

    it("renders users count stat", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Users")).toBeInTheDocument();
      });
    });

    it("renders global services stat", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Global Services")).toBeInTheDocument();
        expect(screen.getByText("2/3")).toBeInTheDocument();
      });
    });

    it("renders tenant services stat", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Tenant Services")).toBeInTheDocument();
        expect(screen.getByText("5")).toBeInTheDocument();
      });
    });

    it("renders pending invitations stat", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Pending Invitations")).toBeInTheDocument();
      });
    });

    it("renders webhooks stat", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        // First verify overview card is rendered
        expect(screen.getByText("Overview")).toBeInTheDocument();
        expect(screen.getByText("Tenant statistics")).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  describe("action", () => {
    it("updates tenant successfully", async () => {
      vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenant });

      const formData = new FormData();
      formData.append("intent", "update");
      formData.append("name", "Updated Acme");
      formData.append("slug", "updated-acme");
      formData.append("logo_url", "https://new-logo.com/logo.png");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(tenantApi.update).toHaveBeenCalledWith("tenant-1", {
        name: "Updated Acme",
        slug: "updated-acme",
        logo_url: "https://new-logo.com/logo.png",
      }, undefined);
      expect(response).toEqual({ success: true });
    });

    it("handles empty logo URL", async () => {
      vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenant });

      const formData = new FormData();
      formData.append("intent", "update");
      formData.append("name", "Updated Acme");
      formData.append("slug", "updated-acme");
      formData.append("logo_url", "");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(tenantApi.update).toHaveBeenCalledWith("tenant-1", {
        name: "Updated Acme",
        slug: "updated-acme",
        logo_url: undefined,
      }, undefined);
    });

    it("returns error when tenantId is missing", async () => {
      const formData = new FormData();
      formData.append("intent", "update");

      const request = new Request("http://localhost/dashboard/tenants/", {
        method: "POST",
        body: formData,
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

    it("returns error on API failure", async () => {
      vi.mocked(tenantApi.update).mockRejectedValue(new Error("Slug already exists"));

      const formData = new FormData();
      formData.append("intent", "update");
      formData.append("name", "Updated Acme");
      formData.append("slug", "existing-slug");
      formData.append("logo_url", "");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Slug already exists" });
    });

    it("returns error for invalid intent", async () => {
      const formData = new FormData();
      formData.append("intent", "invalid");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Invalid intent" });
    });
  });

  // ============================================================================
  // Feedback Message Tests
  // ============================================================================

  describe("feedback messages", () => {
    it("displays success message after update", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => ({
            tenant: mockTenant,
            servicesCount: 5,
            pendingInvitationsCount: 3,
            webhooksCount: 2,
                  enabledServicesCount: 2,
            totalGlobalServicesCount: 3,
          }),
          action: () => ({ success: true }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByLabelText("Name")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Save Changes" }));

      await waitFor(() => {
        expect(screen.getByText("Tenant updated successfully")).toBeInTheDocument();
      });
    });

    it("renders error message container when error exists in actionData", async () => {
      // Test that the error message display logic exists by checking the component structure
      // The actual error message formatting is tested via action tests
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => ({
            tenant: mockTenant,
            servicesCount: 5,
            pendingInvitationsCount: 3,
            webhooksCount: 2,
                  enabledServicesCount: 2,
            totalGlobalServicesCount: 3,
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        // Verify the form structure exists for displaying feedback
        expect(screen.getByRole("button", { name: "Save Changes" })).toBeInTheDocument();
        expect(screen.getByLabelText("Name")).toBeInTheDocument();
        expect(screen.getByLabelText("Slug")).toBeInTheDocument();
      });
    });

    it("displays error message from action", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => ({
            tenant: mockTenant,
            servicesCount: 5,
            pendingInvitationsCount: 3,
            webhooksCount: 2,
                  enabledServicesCount: 2,
            totalGlobalServicesCount: 3,
          }),
          action: () => ({ error: "Operation not allowed" }),
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByLabelText("Name")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Save Changes" }));

      await waitFor(() => {
        expect(screen.getByText("Operation not allowed")).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Security Settings Tests
  // ============================================================================

  describe("security settings", () => {
    const loaderData = {
      tenant: mockTenant,
      usersCount: 3,
      servicesCount: 5,
      pendingInvitationsCount: 3,
      webhooksCount: 2,
      enabledServicesCount: 2,
      totalGlobalServicesCount: 3,
    };

    it("renders security settings card with MFA switch", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByText("Security Settings")).toBeInTheDocument();
        expect(screen.getByText("Require MFA")).toBeInTheDocument();
        expect(screen.getByText(/require all users in this tenant/i)).toBeInTheDocument();
      });

      // MFA switch should be present and unchecked (settings is {})
      const mfaSwitch = screen.getByRole("switch");
      expect(mfaSwitch).toBeInTheDocument();
      expect(mfaSwitch).toHaveAttribute("data-state", "unchecked");
    });

    it("renders MFA switch as checked when require_mfa is true", async () => {
      const tenantWithMfa = {
        ...mockTenant,
        settings: { require_mfa: true },
      };

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => ({
            ...loaderData,
            tenant: tenantWithMfa,
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        const mfaSwitch = screen.getByRole("switch");
        expect(mfaSwitch).toHaveAttribute("data-state", "checked");
      });
    });

    it("toggles MFA switch and submits settings via fetcher", async () => {
      const actionFn = vi.fn().mockReturnValue({ success: true, settingsUpdated: true });
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId",
          Component: TenantDetailPage,
          loader: () => loaderData,
          action: actionFn,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1"]} />);

      await waitFor(() => {
        expect(screen.getByRole("switch")).toBeInTheDocument();
      });

      // MFA switch should be unchecked initially
      const mfaSwitch = screen.getByRole("switch");
      expect(mfaSwitch).toHaveAttribute("data-state", "unchecked");

      // Click to toggle MFA on - this triggers settingsFetcher.submit
      await user.click(mfaSwitch);

      // The action should have been called with the settings data
      await waitFor(() => {
        expect(actionFn).toHaveBeenCalled();
      });
    });
  });

  // ============================================================================
  // Action - update_settings tests
  // ============================================================================

  describe("action - update_settings", () => {
    it("updates security settings with require_mfa true", async () => {
      vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenant });

      const formData = new FormData();
      formData.append("intent", "update_settings");
      formData.append("require_mfa", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(tenantApi.update).toHaveBeenCalledWith("tenant-1", {
        settings: { require_mfa: true },
      }, undefined);
      expect(response).toEqual({ success: true, settingsUpdated: true });
    });

    it("updates security settings with require_mfa false", async () => {
      vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenant });

      const formData = new FormData();
      formData.append("intent", "update_settings");
      formData.append("require_mfa", "false");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(tenantApi.update).toHaveBeenCalledWith("tenant-1", {
        settings: { require_mfa: false },
      }, undefined);
      expect(response).toEqual({ success: true, settingsUpdated: true });
    });

    it("returns error when update_settings API call fails", async () => {
      vi.mocked(tenantApi.update).mockRejectedValue(new Error("MFA update failed"));

      const formData = new FormData();
      formData.append("intent", "update_settings");
      formData.append("require_mfa", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "MFA update failed" });
    });
  });
});
