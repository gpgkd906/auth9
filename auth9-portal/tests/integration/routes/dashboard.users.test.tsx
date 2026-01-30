import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import UsersPage, { loader } from "~/routes/dashboard.users";
import { userApi, tenantApi, serviceApi, rbacApi } from "~/services/api";

// Mock the APIs
vi.mock("~/services/api", () => ({
    userApi: {
        list: vi.fn(),
        create: vi.fn(),
        update: vi.fn(),
        addToTenant: vi.fn(),
        removeFromTenant: vi.fn(),
        getTenants: vi.fn(),
    },
    tenantApi: {
        list: vi.fn(),
    },
    serviceApi: {
        list: vi.fn(),
    },
    rbacApi: {
        listRoles: vi.fn(),
        getUserAssignedRoles: vi.fn(),
        assignRoles: vi.fn(),
        unassignRole: vi.fn(),
    },
}));

describe("Users Page", () => {
    const mockUsers = {
        data: [
            {
                id: "u1",
                email: "alice@example.com",
                display_name: "Alice",
                mfa_enabled: false,
                updated_at: new Date().toISOString(),
            },
            {
                id: "u2",
                email: "bob@example.com",
                display_name: "Bob",
                mfa_enabled: true,
                updated_at: new Date().toISOString(),
            },
        ],
        pagination: { total: 2, page: 1, total_pages: 1 },
    };

    const mockTenants = {
        data: [
            { id: "t1", name: "Tenant 1", slug: "t1" },
        ],
        pagination: { total: 1, page: 1, total_pages: 1 },
    };

    const mockServices = {
        data: [
            { id: "s1", name: "Service 1" },
        ],
        pagination: { total: 1, page: 1, total_pages: 1 },
    };

    it("renders user list from loader", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("Alice")).toBeInTheDocument();
            expect(screen.getByText("alice@example.com")).toBeInTheDocument();
        });
    });

    it("displays create user dialog", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        // Wait for the button
        const createButton = await screen.findByRole("button", { name: /\+ Create User/i });
        await user.click(createButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Create a new user account.")).toBeInTheDocument();
    });

    it("shows MFA status for each user", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            // Alice has MFA disabled, Bob has MFA enabled
            expect(screen.getByText("Disabled")).toBeInTheDocument();
            expect(screen.getByText("Enabled")).toBeInTheDocument();
        });
    });

    it("displays empty state when no users", async () => {
        (userApi.list as any).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, total_pages: 1 },
        });
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("No users found")).toBeInTheDocument();
        });
    });

    it("displays user directory card title", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("User Directory")).toBeInTheDocument();
            expect(screen.getByText(/2 users/)).toBeInTheDocument();
        });
    });

    it("displays user table headers", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("Email")).toBeInTheDocument();
            expect(screen.getByText("Display Name")).toBeInTheDocument();
            expect(screen.getByText("MFA")).toBeInTheDocument();
            expect(screen.getByText("Updated")).toBeInTheDocument();
        });
    });

    it("opens edit user dialog when clicking edit", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        // Wait for user list to load
        await waitFor(() => {
            expect(screen.getByText("Alice")).toBeInTheDocument();
        });

        // Click the dropdown menu (first one for Alice)
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);

        // Click Edit User option
        const editOption = await screen.findByText("Edit User");
        await user.click(editOption);

        // Verify edit dialog opens
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Edit User")).toBeInTheDocument();
    });

    it("opens manage tenants dialog when clicking manage tenants", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (userApi.getTenants as any).mockResolvedValue({ data: [] });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        // Wait for user list
        await waitFor(() => {
            expect(screen.getByText("Alice")).toBeInTheDocument();
        });

        // Click dropdown
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);

        // Click Manage Tenants option
        const manageTenants = await screen.findByText("Manage Tenants");
        await user.click(manageTenants);

        // Verify manage tenants dialog opens
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText(/Manage Tenants for/i)).toBeInTheDocument();
    });

    it("shows joined tenants list in manage tenants dialog", async () => {
        const userTenantsData = [
            {
                tenant_id: "t1",
                role_in_tenant: "admin",
                tenant: { id: "t1", name: "Tenant 1", logo_url: null },
            },
        ];
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (userApi.getTenants as any).mockResolvedValue({ data: userTenantsData });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        await user.click(await screen.findByText("Manage Tenants"));

        await waitFor(() => {
            expect(screen.getByText("Tenant 1")).toBeInTheDocument();
            expect(screen.getByText("(admin)")).toBeInTheDocument();
        });
    });

    it("shows empty state when user has no tenants", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (userApi.getTenants as any).mockResolvedValue({ data: [] });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        await user.click(await screen.findByText("Manage Tenants"));

        await waitFor(() => {
            expect(screen.getByText("Not a member of any tenant.")).toBeInTheDocument();
        });
    });

    it("displays page header and description", async () => {
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("Users")).toBeInTheDocument();
            expect(screen.getByText("Manage users and tenant assignments")).toBeInTheDocument();
        });
    });

    it("opens manage roles dialog and displays service selector", async () => {
        const userTenantsData = [
            {
                tenant_id: "t1",
                role_in_tenant: "admin",
                tenant: { id: "t1", name: "Tenant 1", logo_url: null },
            },
        ];
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (userApi.getTenants as any).mockResolvedValue({ data: userTenantsData });
        (rbacApi.getUserAssignedRoles as any).mockResolvedValue({ data: [] });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

        // Open dropdown menu
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        await user.click(await screen.findByText("Manage Tenants"));

        // Click Roles button for the tenant
        await waitFor(() => expect(screen.getByText("Tenant 1")).toBeInTheDocument());
        await user.click(screen.getByRole("button", { name: /Roles/i }));

        // Verify role assignment dialog opens
        await waitFor(() => {
            expect(screen.getByText("Assign Roles")).toBeInTheDocument();
            expect(screen.getByText(/Assign roles in/i)).toBeInTheDocument();
        });
    });

    it("disables save roles button when no service is selected", async () => {
        const userTenantsData = [
            {
                tenant_id: "t1",
                role_in_tenant: "admin",
                tenant: { id: "t1", name: "Tenant 1", logo_url: null },
            },
        ];
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (userApi.getTenants as any).mockResolvedValue({ data: userTenantsData });
        (rbacApi.getUserAssignedRoles as any).mockResolvedValue({ data: [] });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

        // Navigate to roles dialog
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        await user.click(await screen.findByText("Manage Tenants"));
        await waitFor(() => expect(screen.getByText("Tenant 1")).toBeInTheDocument());
        await user.click(screen.getByRole("button", { name: /Roles/i }));

        // Verify Save Roles button is disabled without service selected
        await waitFor(() => {
            const saveButton = screen.getByRole("button", { name: /Save Roles/i });
            expect(saveButton).toBeDisabled();
        });
    });

    it("fetches user assigned roles when opening roles dialog", async () => {
        const userTenantsData = [
            {
                tenant_id: "t1",
                role_in_tenant: "admin",
                tenant: { id: "t1", name: "Tenant 1", logo_url: null },
            },
        ];
        const assignedRoles = [
            { id: "r1", name: "Admin", description: "Admin role" },
        ];
        (userApi.list as any).mockResolvedValue(mockUsers);
        (tenantApi.list as any).mockResolvedValue(mockTenants);
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (userApi.getTenants as any).mockResolvedValue({ data: userTenantsData });
        (rbacApi.getUserAssignedRoles as any).mockResolvedValue({ data: assignedRoles });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/users",
                Component: UsersPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

        // Navigate to roles dialog
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        await user.click(await screen.findByText("Manage Tenants"));
        await waitFor(() => expect(screen.getByText("Tenant 1")).toBeInTheDocument());
        await user.click(screen.getByRole("button", { name: /Roles/i }));

        // Verify rbacApi.getUserAssignedRoles was called
        await waitFor(() => {
            expect(rbacApi.getUserAssignedRoles).toHaveBeenCalledWith("u1", "t1");
        });
    });
});

