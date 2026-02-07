import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import TenantServicesPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId.services";
import { tenantApi, tenantServiceApi } from "~/services/api";

// Mock the APIs
vi.mock("~/services/api", () => ({
  tenantApi: {
    get: vi.fn(),
  },
  tenantServiceApi: {
    listServices: vi.fn(),
    toggleService: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue(null),
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
  status: "active" as const,
  created_at: "2024-01-10T08:00:00Z",
  updated_at: "2024-01-10T08:00:00Z",
};

const mockServices = [
  {
    id: "svc-1",
    name: "Auth Service",
    base_url: "https://auth.example.com",
    status: "active",
    enabled: true,
  },
  {
    id: "svc-2",
    name: "API Gateway",
    base_url: "https://api.example.com",
    status: "active",
    enabled: false,
  },
  {
    id: "svc-3",
    name: "Analytics",
    base_url: null,
    status: "inactive",
    enabled: true,
  },
];

describe("Tenant Services Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  describe("loader", () => {
    it("fetches tenant and services", async () => {
      vi.mocked(tenantApi.get).mockResolvedValue({ data: mockTenant });
      vi.mocked(tenantServiceApi.listServices).mockResolvedValue({ data: mockServices });

      const response = await loader({
        request: new Request("http://localhost/dashboard/tenants/tenant-1/services"),
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(response).toEqual({
        tenant: mockTenant,
        services: mockServices,
      });
      expect(tenantApi.get).toHaveBeenCalledWith("tenant-1", undefined);
      expect(tenantServiceApi.listServices).toHaveBeenCalledWith("tenant-1", undefined);
    });

    it("throws error when tenantId is missing", async () => {
      await expect(
        loader({
          request: new Request("http://localhost/dashboard/tenants//services"),
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
      services: mockServices,
    };

    it("renders page title with tenant name", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("Services for Acme Corporation")).toBeInTheDocument();
      });
    });

    it("renders page description", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(
          screen.getByText("Enable or disable global services for this tenant")
        ).toBeInTheDocument();
      });
    });

    it("renders back button to tenant detail", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
        {
          path: "/dashboard/tenants/:tenantId",
          Component: () => <div>Tenant Detail</div>,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        const backLink = screen.getByRole("link", { name: "" }); // Icon button
        expect(backLink).toHaveAttribute("href", "/dashboard/tenants/tenant-1");
      });
    });

    it("renders tenant logo when present", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        const logo = document.querySelector("img[src='https://example.com/logo.png']");
        expect(logo).toBeInTheDocument();
      });
    });

    it("does not render logo when absent", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => ({
            tenant: mockTenantNoLogo,
            services: mockServices,
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-2/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("Services for Globex Inc")).toBeInTheDocument();
      });
      expect(document.querySelector("img")).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Rendering Tests - Stats Cards
  // ============================================================================

  describe("stats cards rendering", () => {
    const loaderData = {
      tenant: mockTenant,
      services: mockServices,
    };

    it("renders total services count", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("Total Services")).toBeInTheDocument();
        expect(screen.getByText("3")).toBeInTheDocument();
      });
    });

    it("renders enabled services count", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        // "Enabled" appears in stats card and as service status labels
        const enabledTexts = screen.getAllByText("Enabled");
        expect(enabledTexts.length).toBeGreaterThanOrEqual(1);
        expect(screen.getByText("2")).toBeInTheDocument();
      });
    });

    it("renders disabled services count", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        // "Disabled" appears in stats card and as service status labels
        const disabledTexts = screen.getAllByText("Disabled");
        expect(disabledTexts.length).toBeGreaterThanOrEqual(1);
        expect(screen.getByText("1")).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Rendering Tests - Services List
  // ============================================================================

  describe("services list rendering", () => {
    const loaderData = {
      tenant: mockTenant,
      services: mockServices,
    };

    it("renders services list card", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("Global Services")).toBeInTheDocument();
        expect(
          screen.getByText(
            "Toggle services on or off for this tenant. Enabled services can be accessed by users in this tenant."
          )
        ).toBeInTheDocument();
      });
    });

    it("renders all service names", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("Auth Service")).toBeInTheDocument();
        expect(screen.getByText("API Gateway")).toBeInTheDocument();
        expect(screen.getByText("Analytics")).toBeInTheDocument();
      });
    });

    it("renders service base URLs when present", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("https://auth.example.com")).toBeInTheDocument();
        expect(screen.getByText("https://api.example.com")).toBeInTheDocument();
      });
    });

    it("renders service status badges", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        const activeBadges = screen.getAllByText("active");
        expect(activeBadges.length).toBe(2);
        expect(screen.getByText("inactive")).toBeInTheDocument();
      });
    });

    it("renders enabled/disabled text for each service", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        // Stats card "Enabled" + 2 service rows with "Enabled" = 3 total
        const enabledTexts = screen.getAllByText("Enabled");
        expect(enabledTexts.length).toBe(3);
        // Stats card "Disabled" + 1 service row with "Disabled" = 2 total
        const disabledTexts = screen.getAllByText("Disabled");
        expect(disabledTexts.length).toBe(2);
      });
    });

    it("renders toggle switches for each service", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        const switches = screen.getAllByRole("switch");
        expect(switches.length).toBe(3);
      });
    });

    it("renders switches with correct checked state", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => loaderData,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        const switches = screen.getAllByRole("switch");
        // Auth Service and Analytics are enabled
        expect(switches[0]).toHaveAttribute("data-state", "checked");
        // API Gateway is disabled
        expect(switches[1]).toHaveAttribute("data-state", "unchecked");
        // Analytics is enabled
        expect(switches[2]).toHaveAttribute("data-state", "checked");
      });
    });
  });

  // ============================================================================
  // Rendering Tests - Empty State
  // ============================================================================

  describe("empty state rendering", () => {
    it("renders empty state when no services", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => ({
            tenant: mockTenant,
            services: [],
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("No global services available")).toBeInTheDocument();
        expect(
          screen.getByText("Create services without a tenant_id to make them available here.")
        ).toBeInTheDocument();
      });
    });

    it("renders zero counts when no services", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => ({
            tenant: mockTenant,
            services: [],
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        const zeros = screen.getAllByText("0");
        expect(zeros.length).toBe(3); // Total, Enabled, Disabled all 0
      });
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  describe("action", () => {
    it("toggles service enabled state", async () => {
      vi.mocked(tenantServiceApi.toggleService).mockResolvedValue({
        data: mockServices.map((s) =>
          s.id === "svc-2" ? { ...s, enabled: true } : s
        ),
      });

      const formData = new FormData();
      formData.append("serviceId", "svc-2");
      formData.append("enabled", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/services", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(tenantServiceApi.toggleService).toHaveBeenCalledWith("tenant-1", "svc-2", true, undefined);
      expect(response).toEqual({
        success: true,
        services: expect.any(Array),
      });
    });

    it("toggles service disabled state", async () => {
      vi.mocked(tenantServiceApi.toggleService).mockResolvedValue({
        data: mockServices.map((s) =>
          s.id === "svc-1" ? { ...s, enabled: false } : s
        ),
      });

      const formData = new FormData();
      formData.append("serviceId", "svc-1");
      formData.append("enabled", "false");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/services", {
        method: "POST",
        body: formData,
      });

      const response = await action({
        request,
        params: { tenantId: "tenant-1" },
        context: {},
      });

      expect(tenantServiceApi.toggleService).toHaveBeenCalledWith("tenant-1", "svc-1", false, undefined);
      expect(response).toEqual({
        success: true,
        services: expect.any(Array),
      });
    });

    it("returns error when tenantId is missing", async () => {
      const formData = new FormData();
      formData.append("serviceId", "svc-1");
      formData.append("enabled", "true");

      const request = new Request("http://localhost/dashboard/tenants//services", {
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
      vi.mocked(tenantServiceApi.toggleService).mockRejectedValue(
        new Error("Service not found")
      );

      const formData = new FormData();
      formData.append("serviceId", "invalid-svc");
      formData.append("enabled", "true");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/services", {
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
      expect(json).toEqual({ error: "Service not found" });
    });
  });

  // ============================================================================
  // Interaction Tests
  // ============================================================================

  describe("interactions", () => {
    it("toggle switch triggers form submission", async () => {
      const mockToggle = vi.fn().mockResolvedValue({
        data: mockServices.map((s) =>
          s.id === "svc-2" ? { ...s, enabled: true } : s
        ),
      });

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/services",
          Component: TenantServicesPage,
          loader: () => ({
            tenant: mockTenant,
            services: mockServices,
          }),
          action: async ({ request }) => {
            const formData = await request.formData();
            const serviceId = formData.get("serviceId");
            const enabled = formData.get("enabled") === "true";
            const result = await mockToggle(serviceId, enabled);
            return { success: true, services: result.data };
          },
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/services"]} />);

      await waitFor(() => {
        expect(screen.getByText("API Gateway")).toBeInTheDocument();
      });

      // Find the disabled switch (API Gateway)
      const switches = screen.getAllByRole("switch");
      const apiGatewaySwitch = switches[1]; // Second service is API Gateway
      expect(apiGatewaySwitch).toHaveAttribute("data-state", "unchecked");

      // Click to enable
      await user.click(apiGatewaySwitch);

      await waitFor(() => {
        expect(mockToggle).toHaveBeenCalledWith("svc-2", true);
      });
    });
  });
});
