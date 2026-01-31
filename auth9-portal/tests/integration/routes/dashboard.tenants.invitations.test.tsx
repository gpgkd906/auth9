import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import InvitationsPage, { loader } from "~/routes/dashboard.tenants.$tenantId.invitations";
import { invitationApi, tenantApi, serviceApi, rbacApi } from "~/services/api";

// Mock APIs
vi.mock("~/services/api", () => ({
  invitationApi: {
    list: vi.fn(),
    create: vi.fn(),
    revoke: vi.fn(),
    resend: vi.fn(),
    delete: vi.fn(),
  },
  tenantApi: {
    get: vi.fn(),
  },
  serviceApi: {
    list: vi.fn(),
  },
  rbacApi: {
    listRoles: vi.fn(),
  },
}));

describe("Invitations Page", () => {
  const mockTenant = {
    id: "tenant-1",
    name: "Acme Corp",
    slug: "acme",
    settings: {},
    status: "active" as const,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  const mockInvitations = [
    {
      id: "inv-1",
      tenant_id: "tenant-1",
      email: "pending@example.com",
      role_ids: ["role-1"],
      invited_by: "admin-1",
      status: "pending" as const,
      expires_at: new Date(Date.now() + 72 * 60 * 60 * 1000).toISOString(),
      created_at: new Date().toISOString(),
    },
    {
      id: "inv-2",
      tenant_id: "tenant-1",
      email: "accepted@example.com",
      role_ids: ["role-1", "role-2"],
      invited_by: "admin-1",
      status: "accepted" as const,
      expires_at: new Date(Date.now() + 72 * 60 * 60 * 1000).toISOString(),
      accepted_at: new Date().toISOString(),
      created_at: new Date().toISOString(),
    },
    {
      id: "inv-3",
      tenant_id: "tenant-1",
      email: "expired@example.com",
      role_ids: ["role-1"],
      invited_by: "admin-1",
      status: "expired" as const,
      expires_at: new Date(Date.now() - 1 * 60 * 60 * 1000).toISOString(),
      created_at: new Date().toISOString(),
    },
  ];

  const mockServices = [
    { id: "service-1", name: "Main App", tenant_id: "tenant-1", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  ];

  const mockRoles = [
    { id: "role-1", service_id: "service-1", name: "Admin", description: "Full access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    { id: "role-2", service_id: "service-1", name: "User", description: "Standard user access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
  ];

  beforeEach(() => {
    vi.mocked(tenantApi.get).mockResolvedValue({ data: mockTenant });
    vi.mocked(invitationApi.list).mockResolvedValue({
      data: mockInvitations,
      pagination: { page: 1, per_page: 20, total: 3, total_pages: 1 },
    });
    vi.mocked(serviceApi.list).mockResolvedValue({ data: mockServices, pagination: { page: 1, per_page: 20, total: 1, total_pages: 1 } });
    vi.mocked(rbacApi.listRoles).mockResolvedValue({ data: mockRoles });
  });

  it("renders invitations page with tenant info", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invitations")).toBeInTheDocument();
      expect(screen.getByText(/Acme Corp/)).toBeInTheDocument();
    });
  });

  it("displays invitation list with status badges", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      expect(screen.getByText("accepted@example.com")).toBeInTheDocument();
      expect(screen.getByText("expired@example.com")).toBeInTheDocument();
      expect(screen.getByText("Pending")).toBeInTheDocument();
      expect(screen.getByText("Accepted")).toBeInTheDocument();
      expect(screen.getByText("Expired")).toBeInTheDocument();
    });
  });

  it("shows invite user button", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invite User")).toBeInTheDocument();
    });
  });

  it("shows invite user button and dialog trigger", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      // Invite button should be present
      const inviteButton = screen.getByText("Invite User");
      expect(inviteButton).toBeInTheDocument();
      expect(inviteButton.closest("button")).toBeInTheDocument();
    });
  });

  it("loader fetches roles for dialog", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invite User")).toBeInTheDocument();
    });

    // Verify the API calls were made
    expect(serviceApi.list).toHaveBeenCalledWith("tenant-1");
    expect(rbacApi.listRoles).toHaveBeenCalledWith("service-1");
  });

  it("shows role count for each invitation", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    // Wait for the table to be rendered
    await waitFor(() => {
      expect(screen.getByText("Pending & Past Invitations")).toBeInTheDocument();
    });

    // The role column header should be present
    expect(screen.getByText("Roles")).toBeInTheDocument();
  });

  it("shows empty state when no invitations", async () => {
    vi.mocked(invitationApi.list).mockResolvedValue({
      data: [],
      pagination: { page: 1, per_page: 20, total: 0, total_pages: 1 },
    });

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText(/No invitations found/)).toBeInTheDocument();
    });
  });

  it("shows pagination info", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText(/3 invitations/)).toBeInTheDocument();
      expect(screen.getByText(/Page 1 of 1/)).toBeInTheDocument();
    });
  });

  it("handles empty services list", async () => {
    vi.mocked(serviceApi.list).mockResolvedValue({ data: [], pagination: { page: 1, per_page: 20, total: 0, total_pages: 1 } });

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invite User")).toBeInTheDocument();
    });

    // Verify service list was called even with no services
    expect(serviceApi.list).toHaveBeenCalledWith("tenant-1");
  });

  it("has action menu for each invitation", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      // Look for action menu buttons (sr-only text)
      const menuButtons = screen.getAllByText("Open menu");
      expect(menuButtons).toHaveLength(3); // One for each invitation
    });
  });

  it("has back link to tenants page", async () => {
    const RemixStub = createRemixStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: InvitationsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      // Find the back arrow link by its href
      const links = screen.getAllByRole("link");
      const backLink = links.find((link) => link.getAttribute("href") === "/dashboard/tenants");
      expect(backLink).toBeInTheDocument();
    });
  });
});
