import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import UsersPage, { loader, action } from "~/routes/dashboard.users";
import { userApi, tenantApi, serviceApi, rbacApi, sessionApi } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";

// Mock the APIs
vi.mock("~/services/api", () => ({
    userApi: {
        list: vi.fn(),
        create: vi.fn(),
        update: vi.fn(),
        delete: vi.fn(),
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
    sessionApi: {
        forceLogoutUser: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

function WrappedPage() {
    return (
        <ConfirmProvider>
            <UsersPage />
        </ConfirmProvider>
    );
}

describe("Users Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    const mockUsers = {
        data: [
            {
                id: "u1",
                email: "alice@example.com",
                display_name: "Alice",
                mfa_enabled: false,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
            {
                id: "u2",
                email: "bob@example.com",
                display_name: "Bob",
                mfa_enabled: true,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        ],
        pagination: { total: 2, page: 1, per_page: 20, total_pages: 1 },
    };

    const mockTenants = {
        data: [
            { id: "t1", name: "Tenant 1", slug: "t1", settings: {}, status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ],
        pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
    };

    const mockServices = {
        data: [
            { id: "s1", name: "Service 1", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ],
        pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
    };

    it("renders user list from loader", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("Alice")).toBeInTheDocument();
            expect(screen.getByText("alice@example.com")).toBeInTheDocument();
        });
    });

    it("displays create user dialog", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        // Wait for the button
        const createButton = await screen.findByRole("button", { name: /\+ Create User/i });
        await user.click(createButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Create a new user account.")).toBeInTheDocument();
    });

    it("shows MFA status for each user", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            // Alice has MFA disabled, Bob has MFA enabled
            expect(screen.getByText("Disabled")).toBeInTheDocument();
            expect(screen.getByText("Enabled")).toBeInTheDocument();
        });
    });

    it("displays empty state when no users", async () => {
        vi.mocked(userApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("No users found")).toBeInTheDocument();
        });
    });

    it("displays user directory card title", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("User Directory")).toBeInTheDocument();
            expect(screen.getByText(/2 users/)).toBeInTheDocument();
        });
    });

    it("displays user table headers", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("Email")).toBeInTheDocument();
            expect(screen.getByText("Display Name")).toBeInTheDocument();
            expect(screen.getByText("MFA")).toBeInTheDocument();
            expect(screen.getByText("Updated")).toBeInTheDocument();
        });
    });

    it("opens edit user dialog when clicking edit", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

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
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(userApi.getTenants).mockResolvedValue({ data: [] });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

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
                id: "ut1",
                tenant_id: "t1",
                role_in_tenant: "admin",
                joined_at: new Date().toISOString(),
                tenant: { id: "t1", name: "Tenant 1", slug: "t1", logo_url: undefined, settings: {}, status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            },
        ];
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

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
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(userApi.getTenants).mockResolvedValue({ data: [] });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        await user.click(await screen.findByText("Manage Tenants"));

        await waitFor(() => {
            expect(screen.getByText("Not a member of any tenant.")).toBeInTheDocument();
        });
    });

    it("displays page header and description", async () => {
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

        await waitFor(() => {
            expect(screen.getByText("Users")).toBeInTheDocument();
            expect(screen.getByText("Manage users and tenant assignments")).toBeInTheDocument();
        });
    });

    it("opens manage roles dialog and displays service selector", async () => {
        const userTenantsData = [
            {
                id: "ut1",
                tenant_id: "t1",
                role_in_tenant: "admin",
                joined_at: new Date().toISOString(),
                tenant: { id: "t1", name: "Tenant 1", slug: "t1", logo_url: undefined, settings: {}, status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            },
        ];
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
        vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: [] });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

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
                id: "ut1",
                tenant_id: "t1",
                role_in_tenant: "admin",
                joined_at: new Date().toISOString(),
                tenant: { id: "t1", name: "Tenant 1", slug: "t1", logo_url: undefined, settings: {}, status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            },
        ];
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
        vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: [] });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

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
                id: "ut1",
                tenant_id: "t1",
                role_in_tenant: "admin",
                joined_at: new Date().toISOString(),
                tenant: { id: "t1", name: "Tenant 1", slug: "t1", logo_url: undefined, settings: {}, status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            },
        ];
        const assignedRoles = [
            { id: "r1", service_id: "s1", name: "Admin", description: "Admin role", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ];
        vi.mocked(userApi.list).mockResolvedValue(mockUsers);
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
        vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: assignedRoles });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/users",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/users"]} />);

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

    // ============================================================================
    // Component Interaction Tests
    // ============================================================================

    describe("Edit User Dialog", () => {
        it("populates the edit form with user data", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Edit User"));

            await waitFor(() => {
                const dialog = screen.getByRole("dialog");
                expect(dialog).toBeInTheDocument();
                const input = screen.getByLabelText("Display Name") as HTMLInputElement;
                expect(input.defaultValue).toBe("Alice");
            });
        });

        it("closes edit dialog when Cancel is clicked", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Edit User"));

            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

            // Click Cancel
            await user.click(screen.getByRole("button", { name: /Cancel/i }));

            await waitFor(() => {
                expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
            });
        });

        it("submits edit user form with updated display name", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.update).mockResolvedValue({ data: { ...mockUsers.data[0], display_name: "Alice Updated" } });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Edit User"));

            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

            const input = screen.getByLabelText("Display Name");
            await user.clear(input);
            await user.type(input, "Alice Updated");

            await user.click(screen.getByRole("button", { name: /Save Changes/i }));

            await waitFor(() => {
                expect(userApi.update).toHaveBeenCalledWith("u1", { display_name: "Alice Updated" });
            });
        });
    });

    describe("Create User Dialog", () => {
        it("opens and displays form fields", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));

            await waitFor(() => {
                expect(screen.getByLabelText("Email *")).toBeInTheDocument();
                expect(screen.getByLabelText("Display Name")).toBeInTheDocument();
                expect(screen.getByLabelText("Password *")).toBeInTheDocument();
                expect(screen.getByRole("button", { name: /Create User/i })).toBeInTheDocument();
            });
        });

        it("shows validation error for empty email on blur", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));

            await waitFor(() => expect(screen.getByLabelText("Email *")).toBeInTheDocument());

            // Focus then blur the email field while empty
            const emailInput = screen.getByLabelText("Email *");
            await user.click(emailInput);
            await user.tab(); // blur

            await waitFor(() => {
                expect(screen.getByText("Email is required")).toBeInTheDocument();
            });
        });

        it("shows validation error for invalid email on blur", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));

            await waitFor(() => expect(screen.getByLabelText("Email *")).toBeInTheDocument());

            const emailInput = screen.getByLabelText("Email *");
            await user.type(emailInput, "notanemail");
            await user.tab();

            await waitFor(() => {
                expect(screen.getByText("Please enter a valid email address")).toBeInTheDocument();
            });
        });

        it("clears validation error when valid email is typed after error", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));

            await waitFor(() => expect(screen.getByLabelText("Email *")).toBeInTheDocument());

            const emailInput = screen.getByLabelText("Email *");
            await user.type(emailInput, "bad");
            await user.tab();
            await waitFor(() => expect(screen.getByText("Please enter a valid email address")).toBeInTheDocument());

            // Clear and type valid email - error should clear as onChange fires validateEmail when error exists
            await user.clear(emailInput);
            await user.type(emailInput, "good@example.com");

            await waitFor(() => {
                expect(screen.queryByText("Please enter a valid email address")).not.toBeInTheDocument();
                expect(screen.queryByText("Email is required")).not.toBeInTheDocument();
            });
        });

        it("closes dialog and resets state when Cancel is clicked", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));
            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

            // Type some data then cancel
            const emailInput = screen.getByLabelText("Email *");
            await user.type(emailInput, "bad");
            await user.tab();
            await waitFor(() => expect(screen.getByText("Please enter a valid email address")).toBeInTheDocument());

            // Click Cancel button within create dialog
            const cancelButtons = screen.getAllByRole("button", { name: /Cancel/i });
            await user.click(cancelButtons[cancelButtons.length - 1]);

            await waitFor(() => {
                expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
            });

            // Reopen and check state was reset
            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));
            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());
            const newEmailInput = screen.getByLabelText("Email *") as HTMLInputElement;
            expect(newEmailInput.value).toBe("");
            expect(screen.queryByText("Please enter a valid email address")).not.toBeInTheDocument();
        });

        it("submits create user form with valid data", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.create).mockResolvedValue({ data: { id: "u3", email: "new@example.com", display_name: "New User", mfa_enabled: false, created_at: "", updated_at: "" } });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));
            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

            await user.type(screen.getByLabelText("Email *"), "new@example.com");
            await user.type(screen.getByLabelText("Display Name"), "New User");
            await user.type(screen.getByLabelText("Password *"), "SecurePass123!");

            await user.click(screen.getByRole("button", { name: /^Create User$/i }));

            await waitFor(() => {
                expect(userApi.create).toHaveBeenCalledWith(
                    { email: "new@example.com", display_name: "New User", password: "SecurePass123!" },
                    "test-token"
                );
            });
        });

        it("prevents form submission with invalid email", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await user.click(await screen.findByRole("button", { name: /\+ Create User/i }));
            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

            // Type invalid email
            await user.type(screen.getByLabelText("Email *"), "notvalid");
            await user.type(screen.getByLabelText("Password *"), "Password1!");

            // Submit - should be prevented by validateEmail
            await user.click(screen.getByRole("button", { name: /^Create User$/i }));

            // Validation error should appear
            await waitFor(() => {
                expect(screen.getByText("Please enter a valid email address")).toBeInTheDocument();
            });

            // API should NOT have been called since form submission was prevented
            expect(userApi.create).not.toHaveBeenCalled();
        });
    });

    describe("Delete User Flow", () => {
        it("shows delete confirmation dialog and submits on confirm", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.delete).mockResolvedValue(undefined);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Delete"));

            // Confirmation dialog should appear
            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete User");
                expect(screen.getByText(/Are you sure you want to delete this user/)).toBeInTheDocument();
            });

            // Click the confirm (Delete) button
            await user.click(screen.getByTestId("confirm-dialog-action"));

            await waitFor(() => {
                expect(userApi.delete).toHaveBeenCalledWith("u1");
            });
        });

        it("cancels delete when cancel is clicked in confirmation dialog", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Delete"));

            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete User");
            });

            // Cancel
            await user.click(screen.getByTestId("confirm-dialog-cancel"));

            await waitFor(() => {
                expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
            });
            expect(userApi.delete).not.toHaveBeenCalled();
        });
    });

    describe("Force Logout Flow", () => {
        it("shows force logout confirmation and submits on confirm", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(sessionApi.forceLogoutUser).mockResolvedValue({ message: "ok" });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Force Logout"));

            // Confirmation dialog should appear
            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Force Logout");
                expect(screen.getByText(/Force logout this user from all active sessions/)).toBeInTheDocument();
            });

            // Confirm
            await user.click(screen.getByTestId("confirm-dialog-action"));

            await waitFor(() => {
                expect(sessionApi.forceLogoutUser).toHaveBeenCalledWith("u1", "test-token");
            });
        });

        it("cancels force logout when cancel is clicked", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                    action,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Force Logout"));

            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Force Logout");
            });

            await user.click(screen.getByTestId("confirm-dialog-cancel"));

            await waitFor(() => {
                expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
            });
            expect(sessionApi.forceLogoutUser).not.toHaveBeenCalled();
        });
    });

    describe("Manage Tenants Dialog Interactions", () => {
        const userTenantsData = [
            {
                id: "ut1",
                tenant_id: "t1",
                user_id: "u1",
                role_in_tenant: "admin",
                joined_at: new Date().toISOString(),
                tenant: { id: "t1", name: "Tenant 1", slug: "t1", logo_url: undefined, status: "active" as const },
            },
        ];

        it("shows Remove button for joined tenants", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Manage Tenants"));

            await waitFor(() => {
                expect(screen.getByText("Tenant 1")).toBeInTheDocument();
                expect(screen.getByRole("button", { name: /Remove/i })).toBeInTheDocument();
            });
        });

        it("shows Add to Tenant form with tenant selector", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue({
                ...mockTenants,
                data: [
                    ...mockTenants.data,
                    { id: "t2", name: "Tenant 2", slug: "t2", settings: {}, status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
                ],
            });
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Manage Tenants"));

            await waitFor(() => {
                expect(screen.getByText("Add to Tenant")).toBeInTheDocument();
                expect(screen.getByRole("button", { name: /Add/i })).toBeInTheDocument();
            });
        });

        it("closes manage tenants dialog when dialog is dismissed", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: [] });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup({ pointerEventsCheck: 0 });
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Manage Tenants"));

            await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

            // Press Escape to close
            await user.keyboard("{Escape}");

            await waitFor(() => {
                expect(screen.queryByText(/Manage Tenants for/)).not.toBeInTheDocument();
            });
        });
    });

    describe("Manage Roles Dialog Interactions", () => {
        const userTenantsData = [
            {
                id: "ut1",
                tenant_id: "t1",
                user_id: "u1",
                role_in_tenant: "admin",
                joined_at: new Date().toISOString(),
                tenant: { id: "t1", name: "Tenant 1", slug: "t1", logo_url: undefined, status: "active" as const },
            },
        ];

        const mockRoles = [
            { id: "r1", service_id: "s1", name: "Admin Role", description: "Full access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            { id: "r2", service_id: "s1", name: "Viewer Role", description: "", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ];

        async function openRolesDialog(user: ReturnType<typeof userEvent.setup>) {
            await waitFor(() => expect(screen.getByText("Alice")).toBeInTheDocument());

            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[0]);
            await user.click(await screen.findByText("Manage Tenants"));

            await waitFor(() => expect(screen.getByText("Tenant 1")).toBeInTheDocument());
            await user.click(screen.getByRole("button", { name: /Roles/i }));

            await waitFor(() => expect(screen.getByText("Assign Roles")).toBeInTheDocument());
        }

        it("shows service selector placeholder in roles dialog", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
            vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: [] });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup({ pointerEventsCheck: 0 });
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await openRolesDialog(user);

            // Verify service selector is shown
            expect(screen.getByText("Select Service")).toBeInTheDocument();
            expect(screen.getByText("Service")).toBeInTheDocument();
        });

        it("shows Save Roles and Done buttons in roles dialog", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
            vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: [] });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup({ pointerEventsCheck: 0 });
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await openRolesDialog(user);

            expect(screen.getByRole("button", { name: /Done/i })).toBeInTheDocument();
            expect(screen.getByRole("button", { name: /Save Roles/i })).toBeInTheDocument();
            // Save Roles should be disabled since no service is selected
            expect(screen.getByRole("button", { name: /Save Roles/i })).toBeDisabled();
        });

        it("closes roles dialog when Done is clicked", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
            vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: [] });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup({ pointerEventsCheck: 0 });
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await openRolesDialog(user);

            await user.click(screen.getByRole("button", { name: /Done/i }));

            await waitFor(() => {
                expect(screen.queryByText("Assign Roles")).not.toBeInTheDocument();
            });
        });

        it("displays tenant name in roles dialog description", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
            vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: [] });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup({ pointerEventsCheck: 0 });
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await openRolesDialog(user);

            expect(screen.getByText(/Assign roles in Tenant 1/)).toBeInTheDocument();
        });

        it("fetches user assigned roles when roles dialog opens", async () => {
            const assignedRoles = [mockRoles[0]];
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
            vi.mocked(userApi.getTenants).mockResolvedValue({ data: userTenantsData });
            vi.mocked(rbacApi.getUserAssignedRoles).mockResolvedValue({ data: assignedRoles });

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup({ pointerEventsCheck: 0 });
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await openRolesDialog(user);

            await waitFor(() => {
                expect(rbacApi.getUserAssignedRoles).toHaveBeenCalledWith("u1", "t1");
            });
        });
    });

    describe("Pagination display", () => {
        it("displays correct pagination info for multiple pages", async () => {
            vi.mocked(userApi.list).mockResolvedValue({
                data: mockUsers.data,
                pagination: { total: 50, page: 2, per_page: 20, total_pages: 3 },
            });
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => {
                expect(screen.getByText(/50 users/)).toBeInTheDocument();
                expect(screen.getByText(/Page 2 of/)).toBeInTheDocument();
            });
        });
    });

    describe("Dropdown menu actions for second user", () => {
        it("opens dropdown menu for Bob and shows all actions", async () => {
            vi.mocked(userApi.list).mockResolvedValue(mockUsers);
            vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
            vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/users",
                    Component: WrappedPage,
                    loader,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/users"]} />);

            await waitFor(() => expect(screen.getByText("Bob")).toBeInTheDocument());

            // Open dropdown for Bob (second user)
            const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
            await user.click(menuButtons[1]);

            await waitFor(() => {
                expect(screen.getByText("Edit User")).toBeInTheDocument();
                expect(screen.getByText("Manage Tenants")).toBeInTheDocument();
                expect(screen.getByText("Force Logout")).toBeInTheDocument();
                expect(screen.getByText("Delete")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    describe("action", () => {
        function createFormRequest(data: Record<string, string>) {
            const formData = new FormData();
            for (const [key, value] of Object.entries(data)) {
                formData.append(key, value);
            }
            return new Request("http://localhost/dashboard/users", {
                method: "POST",
                body: formData,
            });
        }

        it("update_user calls userApi.update", async () => {
            vi.mocked(userApi.update).mockResolvedValue({ data: { id: "u1", email: "a@b.com", display_name: "New Name", mfa_enabled: false, created_at: "", updated_at: "" } });

            const request = createFormRequest({
                intent: "update_user",
                id: "u1",
                display_name: "New Name",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "update_user" });
            expect(userApi.update).toHaveBeenCalledWith("u1", { display_name: "New Name" });
        });

        it("create_user calls userApi.create", async () => {
            vi.mocked(userApi.create).mockResolvedValue({ data: { id: "u2", email: "new@test.com", display_name: "New", mfa_enabled: false, created_at: "", updated_at: "" } });

            const request = createFormRequest({
                intent: "create_user",
                email: "new@test.com",
                display_name: "New User",
                password: "Password123!",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "create_user" });
            expect(userApi.create).toHaveBeenCalledWith(
                { email: "new@test.com", display_name: "New User", password: "Password123!" },
                "test-token"
            );
        });

        it("add_to_tenant calls userApi.addToTenant", async () => {
            vi.mocked(userApi.addToTenant).mockResolvedValue({});

            const request = createFormRequest({
                intent: "add_to_tenant",
                user_id: "u1",
                tenant_id: "t1",
                role_in_tenant: "admin",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "add_to_tenant" });
            expect(userApi.addToTenant).toHaveBeenCalledWith("u1", "t1", "admin");
        });

        it("remove_from_tenant calls userApi.removeFromTenant", async () => {
            vi.mocked(userApi.removeFromTenant).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "remove_from_tenant",
                user_id: "u1",
                tenant_id: "t1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "remove_from_tenant" });
            expect(userApi.removeFromTenant).toHaveBeenCalledWith("u1", "t1");
        });

        it("assign_roles calls rbacApi.assignRoles", async () => {
            vi.mocked(rbacApi.assignRoles).mockResolvedValue({});

            const formData = new FormData();
            formData.append("intent", "assign_roles");
            formData.append("user_id", "u1");
            formData.append("tenant_id", "t1");
            formData.append("roles", JSON.stringify(["r1", "r2"]));

            const request = new Request("http://localhost/dashboard/users", {
                method: "POST",
                body: formData,
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "assign_roles" });
            expect(rbacApi.assignRoles).toHaveBeenCalledWith({
                user_id: "u1",
                tenant_id: "t1",
                roles: ["r1", "r2"],
            });
        });

        it("unassign_role calls rbacApi.unassignRole", async () => {
            vi.mocked(rbacApi.unassignRole).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "unassign_role",
                user_id: "u1",
                tenant_id: "t1",
                role_id: "r1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "unassign_role" });
            expect(rbacApi.unassignRole).toHaveBeenCalledWith("u1", "t1", "r1");
        });

        it("delete_user calls userApi.delete", async () => {
            vi.mocked(userApi.delete).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "delete_user",
                id: "u1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "delete_user" });
            expect(userApi.delete).toHaveBeenCalledWith("u1");
        });

        it("force_logout calls sessionApi.forceLogoutUser", async () => {
            vi.mocked(sessionApi.forceLogoutUser).mockResolvedValue({ message: "ok" });

            const request = createFormRequest({
                intent: "force_logout",
                id: "u1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true, intent: "force_logout" });
            expect(sessionApi.forceLogoutUser).toHaveBeenCalledWith("u1", "test-token");
        });

        it("returns error on API failure", async () => {
            vi.mocked(userApi.update).mockRejectedValue(new Error("User not found"));

            const request = createFormRequest({
                intent: "update_user",
                id: "u1",
                display_name: "Test",
            });

            const response = await action({ request, params: {}, context: {} });
            expect(response).toBeInstanceOf(Response);
            const data = await (response as Response).json();
            expect(data.error).toBe("User not found");
            expect(data.intent).toBe("update_user");
        });

        it("returns error for invalid intent", async () => {
            const request = createFormRequest({ intent: "invalid" });

            const response = await action({ request, params: {}, context: {} });
            expect(response).toBeInstanceOf(Response);
            const data = await (response as Response).json();
            expect(data.error).toBe("Invalid intent");
        });
    });
});

