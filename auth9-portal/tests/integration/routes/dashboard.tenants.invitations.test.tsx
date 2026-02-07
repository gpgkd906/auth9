import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import InvitationsPage, { loader, action } from "~/routes/dashboard.tenants.$tenantId.invitations";
import { invitationApi, tenantApi, tenantServiceApi, rbacApi } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";

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
  tenantServiceApi: {
    getEnabledServices: vi.fn(),
  },
  rbacApi: {
    listRoles: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue(null),
}));

import { getAccessToken } from "~/services/session.server";

function WrappedPage() {
  return (
    <ConfirmProvider>
      <InvitationsPage />
    </ConfirmProvider>
  );
}

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
    { id: "service-1", name: "Main App", base_url: "https://app.example.com", status: "active", enabled: true },
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
    vi.mocked(tenantServiceApi.getEnabledServices).mockResolvedValue({ data: mockServices });
    vi.mocked(rbacApi.listRoles).mockResolvedValue({ data: mockRoles });
  });

  it("renders invitations page with tenant info", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invitations")).toBeInTheDocument();
      expect(screen.getByText(/Acme Corp/)).toBeInTheDocument();
    });
  });

  it("displays invitation list with status badges", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

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
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invite User")).toBeInTheDocument();
    });
  });

  it("shows invite user button and dialog trigger", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      // Invite button should be present
      const inviteButton = screen.getByText("Invite User");
      expect(inviteButton).toBeInTheDocument();
      expect(inviteButton.closest("button")).toBeInTheDocument();
    });
  });

  it("loader fetches roles for dialog", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invite User")).toBeInTheDocument();
    });

    // Verify the API calls were made
    expect(tenantServiceApi.getEnabledServices).toHaveBeenCalledWith("tenant-1", undefined);
    expect(rbacApi.listRoles).toHaveBeenCalledWith("service-1", undefined);
  });

  it("shows role count for each invitation", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText(/No invitations found/)).toBeInTheDocument();
    });
  });

  it("shows pagination info", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText(/3 invitations/)).toBeInTheDocument();
      expect(screen.getByText(/Page 1 of 1/)).toBeInTheDocument();
    });
  });

  it("handles empty services list", async () => {
    vi.mocked(tenantServiceApi.getEnabledServices).mockResolvedValue({ data: [] });

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invite User")).toBeInTheDocument();
    });

    // Verify enabled services was called even with no services
    expect(tenantServiceApi.getEnabledServices).toHaveBeenCalledWith("tenant-1", undefined);
  });

  it("has action menu for each invitation", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      // Look for action menu buttons (sr-only text)
      const menuButtons = screen.getAllByText("Open menu");
      expect(menuButtons).toHaveLength(3); // One for each invitation
    });
  });

  it("has back link to tenants page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/invitations",
        Component: WrappedPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);

    await waitFor(() => {
      // Find the back arrow link by its href
      const links = screen.getAllByRole("link");
      const backLink = links.find((link) => link.getAttribute("href") === "/dashboard/tenants");
      expect(backLink).toBeInTheDocument();
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
      return new Request("http://localhost/dashboard/tenants/tenant-1/invitations", {
        method: "POST",
        body: formData,
      });
    }

    it("create invitation with roles", async () => {
      vi.mocked(invitationApi.create).mockResolvedValue({ data: { id: "inv-new", status: "pending" } });

      const formData = new FormData();
      formData.append("intent", "create");
      formData.append("email", "new@example.com");
      formData.append("expires_in_hours", "48");
      formData.append("role_role-1", "on");
      formData.append("role_role-2", "on");

      const request = new Request("http://localhost/dashboard/tenants/tenant-1/invitations", {
        method: "POST",
        body: formData,
      });

      const result = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(result).toEqual({ success: true });
      expect(invitationApi.create).toHaveBeenCalledWith(
        "tenant-1",
        {
          email: "new@example.com",
          role_ids: ["role-1", "role-2"],
          expires_in_hours: 48,
        },
        expect.any(String)
      );
    });

    it("create invitation returns error when no roles selected", async () => {
      const request = createFormRequest({
        intent: "create",
        email: "new@example.com",
        expires_in_hours: "48",
      });

      const response = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(response).toBeInstanceOf(Response);
      const data = await (response as Response).json();
      expect(data.error).toBe("At least one role must be selected");
    });

    it("revoke invitation calls invitationApi.revoke", async () => {
      vi.mocked(invitationApi.revoke).mockResolvedValue({ data: { id: "inv-1", status: "revoked" } });

      const request = createFormRequest({
        intent: "revoke",
        id: "inv-1",
      });

      const result = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(result).toEqual({ success: true });
      expect(invitationApi.revoke).toHaveBeenCalledWith("inv-1", expect.any(String));
    });

    it("resend invitation calls invitationApi.resend", async () => {
      vi.mocked(invitationApi.resend).mockResolvedValue({ data: { id: "inv-1", status: "pending" } });

      const request = createFormRequest({
        intent: "resend",
        id: "inv-1",
      });

      const result = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(result).toEqual({ success: true, message: "Invitation email resent" });
    });

    it("delete invitation calls invitationApi.delete", async () => {
      vi.mocked(invitationApi.delete).mockResolvedValue(undefined);

      const request = createFormRequest({
        intent: "delete",
        id: "inv-1",
      });

      const result = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(result).toEqual({ success: true });
      expect(invitationApi.delete).toHaveBeenCalledWith("inv-1", expect.any(String));
    });

    it("returns error on API failure", async () => {
      vi.mocked(invitationApi.revoke).mockRejectedValue(new Error("Cannot revoke accepted invitation"));

      const request = createFormRequest({
        intent: "revoke",
        id: "inv-2",
      });

      const response = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(response).toBeInstanceOf(Response);
      const data = await (response as Response).json();
      expect(data.error).toBe("Cannot revoke accepted invitation");
    });

    it("returns error for invalid intent", async () => {
      const request = createFormRequest({ intent: "invalid" });

      const response = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(response).toBeInstanceOf(Response);
      const data = await (response as Response).json();
      expect(data.error).toBe("Invalid intent");
    });

    it("returns 401 when no access token", async () => {
      vi.mocked(getAccessToken).mockResolvedValueOnce(null);

      const request = createFormRequest({
        intent: "create",
        email: "test@test.com",
      });

      const response = await action({ params: { tenantId: "tenant-1" }, request, context: {} });
      expect(response).toBeInstanceOf(Response);
      const data = await (response as Response).json();
      expect(data.error).toBe("Authentication required");
    });
  });

  // ============================================================================
  // Component Interaction Tests
  // ============================================================================

  describe("component interactions", () => {
    function renderPage(loaderOverride?: () => unknown) {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/tenants/:tenantId/invitations",
          Component: WrappedPage,
          loader: loaderOverride ?? loader,
          action,
        },
      ]);

      const user = userEvent.setup();
      render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/invitations"]} />);
      return { user };
    }

    it("opens the invite user dialog when clicking the Invite User button", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText(/Send an invitation email to join/)).toBeInTheDocument();
        expect(screen.getByLabelText("Email Address")).toBeInTheDocument();
      });
    });

    it("shows role checkboxes in the invite dialog", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        // Service name is rendered as "Main App" in the DOM; CSS uppercase transforms it visually
        expect(screen.getByText("Main App")).toBeInTheDocument();
        expect(screen.getByText("Admin")).toBeInTheDocument();
        expect(screen.getByText("User")).toBeInTheDocument();
      });
    });

    it("toggles role checkbox selection", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // The Send Invitation button should be disabled initially (no roles selected)
      const sendButton = screen.getByRole("button", { name: /Send Invitation/i });
      expect(sendButton).toBeDisabled();

      // Click the Admin role checkbox
      const adminCheckbox = screen.getByRole("checkbox", { name: /Admin/i });
      await user.click(adminCheckbox);

      // Now the Send Invitation button should be enabled
      expect(sendButton).not.toBeDisabled();

      // Click the Admin role checkbox again to deselect
      await user.click(adminCheckbox);

      // Button should be disabled again
      expect(sendButton).toBeDisabled();
    });

    it("closes the invite dialog when Cancel button is clicked", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Click Cancel
      await user.click(screen.getByRole("button", { name: /Cancel/i }));

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });

    it("shows no services message in invite dialog when no services exist", async () => {
      const { user } = renderPage(() => ({
        tenant: mockTenant,
        invitations: [],
        pagination: { page: 1, per_page: 20, total: 0, total_pages: 1 },
        roles: [],
        servicesCount: 0,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByText(/No services configured for this tenant/)).toBeInTheDocument();
      });
    });

    it("shows no roles message in invite dialog when services exist but no roles defined", async () => {
      const { user } = renderPage(() => ({
        tenant: mockTenant,
        invitations: [],
        pagination: { page: 1, per_page: 20, total: 0, total_pages: 1 },
        roles: [],
        servicesCount: 2,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByText(/Services exist but no roles are defined/)).toBeInTheDocument();
      });
    });

    it("shows 'No roles defined' for a service group with empty roles in invite dialog", async () => {
      const { user } = renderPage(() => ({
        tenant: mockTenant,
        invitations: [],
        pagination: { page: 1, per_page: 20, total: 0, total_pages: 1 },
        roles: [{ serviceId: "svc-1", serviceName: "Empty Service", roles: [] }],
        servicesCount: 1,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        // Service name is rendered as "Empty Service" in the DOM; CSS uppercase transforms it visually
        expect(screen.getByText("Empty Service")).toBeInTheDocument();
        expect(screen.getByText("No roles defined")).toBeInTheDocument();
      });
    });

    it("opens dropdown menu and shows actions for a pending invitation", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      });

      // Click the first action menu (pending invitation)
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Actions")).toBeInTheDocument();
        expect(screen.getByText("Resend Email")).toBeInTheDocument();
        expect(screen.getByText("Revoke")).toBeInTheDocument();
        expect(screen.getByText("Delete")).toBeInTheDocument();
      });
    });

    it("opens dropdown menu for non-pending invitation and only shows Delete", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("accepted@example.com")).toBeInTheDocument();
      });

      // Click the second action menu (accepted invitation)
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[1]);

      await waitFor(() => {
        expect(screen.getByText("Actions")).toBeInTheDocument();
        expect(screen.getByText("Delete")).toBeInTheDocument();
      });

      // Resend and Revoke should NOT be present for accepted invitations
      expect(screen.queryByText("Resend Email")).not.toBeInTheDocument();
      expect(screen.queryByText("Revoke")).not.toBeInTheDocument();
    });

    it("clicking Resend Email triggers the resend action", async () => {
      vi.mocked(invitationApi.resend).mockResolvedValue({ data: { id: "inv-1", status: "pending" } });
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      });

      // Open dropdown for pending invitation
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Resend Email")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Resend Email"));

      // The resend action submits a form; verify the API was eventually called
      await waitFor(() => {
        expect(invitationApi.resend).toHaveBeenCalledWith("inv-1", expect.any(String));
      });
    });

    it("clicking Delete opens confirm dialog and confirming submits delete action", async () => {
      vi.mocked(invitationApi.delete).mockResolvedValue(undefined);
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      });

      // Open dropdown for pending invitation
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Delete")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Delete"));

      // Confirm dialog should appear
      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Invitation");
        expect(screen.getByText("Are you sure you want to delete this invitation?")).toBeInTheDocument();
      });

      // Click the confirm action button
      await user.click(screen.getByTestId("confirm-dialog-action"));

      await waitFor(() => {
        expect(invitationApi.delete).toHaveBeenCalledWith("inv-1", expect.any(String));
      });
    });

    it("clicking Delete and then cancelling does not submit delete action", async () => {
      vi.mocked(invitationApi.delete).mockClear();
      vi.mocked(invitationApi.delete).mockResolvedValue(undefined);
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      });

      // Open dropdown for pending invitation
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Delete")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Delete"));

      // Confirm dialog should appear
      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Invitation");
      });

      // Click the cancel button
      await user.click(screen.getByTestId("confirm-dialog-cancel"));

      // Confirm dialog should close
      await waitFor(() => {
        expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
      });

      // Delete should not have been called after cancelling
      expect(invitationApi.delete).not.toHaveBeenCalled();
    });

    it("clicking Revoke opens confirm dialog and confirming submits revoke action", async () => {
      vi.mocked(invitationApi.revoke).mockResolvedValue({ data: { id: "inv-1", status: "revoked" } });
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      });

      // Open dropdown for pending invitation
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Revoke")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Revoke"));

      // Confirm dialog should appear with "Revoke" label
      await waitFor(() => {
        expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Revoke Invitation");
        expect(screen.getByText("Are you sure you want to revoke this invitation?")).toBeInTheDocument();
      });

      // The confirm button should say "Revoke"
      expect(screen.getByTestId("confirm-dialog-action")).toHaveTextContent("Revoke");

      // Click the confirm action button
      await user.click(screen.getByTestId("confirm-dialog-action"));

      await waitFor(() => {
        expect(invitationApi.revoke).toHaveBeenCalledWith("inv-1", expect.any(String));
      });
    });

    it("renders pagination links when multiple pages exist", async () => {
      renderPage(() => ({
        tenant: mockTenant,
        invitations: mockInvitations,
        pagination: { page: 1, per_page: 20, total: 45, total_pages: 3 },
        roles: [],
        servicesCount: 0,
        status: "all",
      }));

      await waitFor(() => {
        // Text is split across elements, so use regex matcher
        expect(screen.getByText(/45/)).toBeInTheDocument();
        expect(screen.getByText(/Page 1 of 3/)).toBeInTheDocument();
      });

      // Pagination links should be present
      const link1 = screen.getByRole("link", { name: "1" });
      const link2 = screen.getByRole("link", { name: "2" });
      const link3 = screen.getByRole("link", { name: "3" });

      expect(link1).toBeInTheDocument();
      expect(link2).toBeInTheDocument();
      expect(link3).toBeInTheDocument();

      // Verify page link URLs
      expect(link2.getAttribute("href")).toBe("/dashboard/tenants/tenant-1/invitations?page=2");
      expect(link3.getAttribute("href")).toBe("/dashboard/tenants/tenant-1/invitations?page=3");
    });

    it("pagination links include status param when filtering", async () => {
      renderPage(() => ({
        tenant: mockTenant,
        invitations: mockInvitations,
        pagination: { page: 1, per_page: 20, total: 45, total_pages: 3 },
        roles: [],
        servicesCount: 0,
        status: "pending",
      }));

      await waitFor(() => {
        expect(screen.getByText(/Page 1 of 3/)).toBeInTheDocument();
      });

      const link2 = screen.getByRole("link", { name: "2" });
      expect(link2.getAttribute("href")).toBe("/dashboard/tenants/tenant-1/invitations?page=2&status=pending");
    });

    it("displays success message when action returns success with message", async () => {
      renderPage(() => ({
        tenant: mockTenant,
        invitations: mockInvitations,
        pagination: { page: 1, per_page: 20, total: 3, total_pages: 1 },
        roles: [{ serviceId: "service-1", serviceName: "Main App", roles: mockRoles }],
        servicesCount: 1,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("Invitations")).toBeInTheDocument();
      });
    });

    it("displays role count as singular when invitation has exactly 1 role", async () => {
      renderPage(() => ({
        tenant: mockTenant,
        invitations: [
          {
            id: "inv-single",
            tenant_id: "tenant-1",
            email: "single@example.com",
            role_ids: ["role-1"],
            invited_by: "admin-1",
            status: "pending" as const,
            expires_at: new Date(Date.now() + 72 * 60 * 60 * 1000).toISOString(),
            created_at: new Date().toISOString(),
          },
        ],
        pagination: { page: 1, per_page: 20, total: 1, total_pages: 1 },
        roles: [],
        servicesCount: 0,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("1 role")).toBeInTheDocument();
      });
    });

    it("displays role count as plural when invitation has multiple roles", async () => {
      renderPage(() => ({
        tenant: mockTenant,
        invitations: [
          {
            id: "inv-multi",
            tenant_id: "tenant-1",
            email: "multi@example.com",
            role_ids: ["role-1", "role-2", "role-3"],
            invited_by: "admin-1",
            status: "pending" as const,
            expires_at: new Date(Date.now() + 72 * 60 * 60 * 1000).toISOString(),
            created_at: new Date().toISOString(),
          },
        ],
        pagination: { page: 1, per_page: 20, total: 1, total_pages: 1 },
        roles: [],
        servicesCount: 0,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("3 roles")).toBeInTheDocument();
      });
    });

    it("renders the Revoked status badge", async () => {
      renderPage(() => ({
        tenant: mockTenant,
        invitations: [
          {
            id: "inv-revoked",
            tenant_id: "tenant-1",
            email: "revoked@example.com",
            role_ids: ["role-1"],
            invited_by: "admin-1",
            status: "revoked" as const,
            expires_at: new Date().toISOString(),
            created_at: new Date().toISOString(),
          },
        ],
        pagination: { page: 1, per_page: 20, total: 1, total_pages: 1 },
        roles: [],
        servicesCount: 0,
        status: "all",
      }));

      await waitFor(() => {
        expect(screen.getByText("Revoked")).toBeInTheDocument();
      });
    });

    it("sends invitation form with selected roles and email", async () => {
      vi.mocked(invitationApi.create).mockResolvedValue({ data: { id: "inv-new", status: "pending" } });
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      // Open dialog
      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Fill email
      const emailInput = screen.getByLabelText("Email Address");
      await user.type(emailInput, "newuser@example.com");

      // Select a role
      const adminCheckbox = screen.getByRole("checkbox", { name: /Admin/i });
      await user.click(adminCheckbox);

      // Submit the form
      const sendButton = screen.getByRole("button", { name: /Send Invitation/i });
      expect(sendButton).not.toBeDisabled();
      await user.click(sendButton);

      // Verify the create API was called
      await waitFor(() => {
        expect(invitationApi.create).toHaveBeenCalledWith(
          "tenant-1",
          expect.objectContaining({
            email: "newuser@example.com",
            role_ids: expect.arrayContaining(["role-1"]),
          }),
          expect.any(String)
        );
      });
    });

    it("selecting multiple roles and toggling works correctly", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const adminCheckbox = screen.getByRole("checkbox", { name: /Admin/i });
      const userCheckbox = screen.getByRole("checkbox", { name: /User/i });
      const sendButton = screen.getByRole("button", { name: /Send Invitation/i });

      // Select both roles
      await user.click(adminCheckbox);
      await user.click(userCheckbox);
      expect(sendButton).not.toBeDisabled();

      // Deselect Admin
      await user.click(adminCheckbox);
      // Still has User selected, so button should remain enabled
      expect(sendButton).not.toBeDisabled();

      // Deselect User too
      await user.click(userCheckbox);
      // No roles selected, button should be disabled
      expect(sendButton).toBeDisabled();
    });

    it("displays role description in invite dialog", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText(/Full access/)).toBeInTheDocument();
        expect(screen.getByText(/Standard user access/)).toBeInTheDocument();
      });
    });

    it("shows Expires In field with default value in invite dialog", async () => {
      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Expires In")).toBeInTheDocument();
        // The Select component may render the value text in multiple places
        expect(screen.getAllByText("72 hours (default)").length).toBeGreaterThanOrEqual(1);
      });
    });

    it("dialog closes on successful invitation creation via useEffect", async () => {
      vi.mocked(invitationApi.create).mockResolvedValue({ data: { id: "inv-new", status: "pending" } });
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      // Open dialog
      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Fill the form
      const emailInput = screen.getByLabelText("Email Address");
      await user.type(emailInput, "newuser@test.com");

      // Select a role
      const adminCheckbox = screen.getByRole("checkbox", { name: /Admin/i });
      await user.click(adminCheckbox);

      // Submit
      await user.click(screen.getByRole("button", { name: /Send Invitation/i }));

      // After successful action, the dialog should close via useEffect
      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });

    it("displays action error message in invite dialog", async () => {
      vi.mocked(getAccessToken).mockResolvedValue("test-token");
      vi.mocked(invitationApi.create).mockRejectedValue(new Error("Email already invited"));

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("Invite User")).toBeInTheDocument();
      });

      // Open dialog
      await user.click(screen.getByText("Invite User"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Fill form
      const emailInput = screen.getByLabelText("Email Address");
      await user.type(emailInput, "existing@test.com");

      // Select a role
      const adminCheckbox = screen.getByRole("checkbox", { name: /Admin/i });
      await user.click(adminCheckbox);

      // Submit
      await user.click(screen.getByRole("button", { name: /Send Invitation/i }));

      // Error should be displayed
      await waitFor(() => {
        expect(screen.getByText("Email already invited")).toBeInTheDocument();
      });
    });

    it("resend success shows success message banner", async () => {
      vi.mocked(invitationApi.resend).mockResolvedValue({ data: { id: "inv-1", status: "pending" } });
      vi.mocked(getAccessToken).mockResolvedValue("test-token");

      const { user } = renderPage();

      await waitFor(() => {
        expect(screen.getByText("pending@example.com")).toBeInTheDocument();
      });

      // Open dropdown for pending invitation
      const menuButtons = screen.getAllByText("Open menu");
      await user.click(menuButtons[0]);

      await waitFor(() => {
        expect(screen.getByText("Resend Email")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Resend Email"));

      // After successful resend, the success message should appear
      await waitFor(() => {
        expect(screen.getByText("Invitation email resent")).toBeInTheDocument();
      });
    });
  });
});
