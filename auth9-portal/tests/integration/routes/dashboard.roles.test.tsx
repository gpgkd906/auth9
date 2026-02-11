import { createRoutesStub } from "react-router";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import RolesPage, { action } from "~/routes/dashboard.roles";
import { rbacApi } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";
import { getAccessToken } from "~/services/session.server";

// Mock the APIs (used by the component internally when actions are triggered)
vi.mock("~/services/api", () => ({
    serviceApi: {
        list: vi.fn(),
    },
    rbacApi: {
        listRoles: vi.fn(),
        listPermissions: vi.fn(),
        createRole: vi.fn(),
        updateRole: vi.fn(),
        deleteRole: vi.fn(),
        getRole: vi.fn(),
        createPermission: vi.fn(),
        deletePermission: vi.fn(),
        assignPermissionToRole: vi.fn(),
        removePermissionFromRole: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
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

function WrappedPage() {
    return (
        <ConfirmProvider>
            <RolesPage />
        </ConfirmProvider>
    );
}

describe("Roles Page", () => {
    const mockServices = {
        data: [
            { id: "s1", name: "Service A", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            { id: "s2", name: "Service B", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ],
        pagination: { total: 2, page: 1, per_page: 20, total_pages: 1 },
    };

    const mockRoles = [
        { id: "r1", service_id: "s1", name: "Admin", description: "Full access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        { id: "r2", service_id: "s1", name: "Viewer", description: "Read only", parent_role_id: "r1", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    ];

    const mockPermissions = [
        { id: "p1", service_id: "s1", code: "users:read", name: "Read Users", description: "Can read user data", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        { id: "p2", service_id: "s1", code: "users:write", name: "Write Users", description: "Can modify user data", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    ];

    const mockLoaderData = {
        entries: [
            {
                service: mockServices.data[0],
                roles: mockRoles,
                permissions: mockPermissions,
            },
            {
                service: mockServices.data[1],
                roles: [],
                permissions: [],
            },
        ],
        pagination: mockServices.pagination,
    };

    const emptyLoaderData = {
        entries: [],
        pagination: { total: 0, page: 1, per_page: 20, total_pages: 0 },
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

    // ============================================================================
    // Page Header Tests
    // ============================================================================

    describe("page header", () => {
        it("renders page title and description", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Roles & Permissions")).toBeInTheDocument();
                expect(screen.getByText("Manage roles, permissions, and role hierarchy per service")).toBeInTheDocument();
            });
        });

        it("renders tab navigation", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByRole("tab", { name: "Roles" })).toBeInTheDocument();
                expect(screen.getByRole("tab", { name: "Permissions" })).toBeInTheDocument();
                expect(screen.getByRole("tab", { name: "Hierarchy" })).toBeInTheDocument();
            });
        });

        it("renders Roles tab as default selected", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                const rolesTab = screen.getByRole("tab", { name: "Roles" });
                expect(rolesTab).toHaveAttribute("data-state", "active");
            });
        });
    });

    // ============================================================================
    // Roles Tab Tests
    // ============================================================================

    describe("roles tab", () => {
        it("renders roles grouped by service", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
                expect(screen.getByText("Admin")).toBeInTheDocument();
                expect(screen.getByText("- Full access")).toBeInTheDocument();
            });
        });

        it("renders role with parent inheritance indicator", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Viewer")).toBeInTheDocument();
                expect(screen.getByText("(inherits from Admin)")).toBeInTheDocument();
            });
        });

        it("renders Add Role button for each service", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
                expect(addButtons.length).toBe(2); // One for each service
            });
        });

        it("renders Permissions button for each role", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
                expect(permButtons.length).toBe(2); // One for Admin, one for Viewer
            });
        });

        it("renders role management card title", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Role Management")).toBeInTheDocument();
                expect(screen.getByText(/2 services • Create and manage roles/)).toBeInTheDocument();
            });
        });

        it("renders empty state for service without roles", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("No roles created yet for this service")).toBeInTheDocument();
            });
        });

        it("renders empty state when no services exist", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => emptyLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("No services found")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Create Role Dialog Tests
    // ============================================================================

    describe("create role dialog", () => {
        it("opens create role dialog when clicking Add Role button", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            // Wait for page to load
            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            // Find add role buttons and click the first one
            const addButtons = screen.getAllByText(/Add Role/i);
            expect(addButtons.length).toBeGreaterThan(0);
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText("Add a new role to this service.")).toBeInTheDocument();
            });
        });

        it("renders multiple Add Role buttons (one per service)", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
                expect(screen.getByText("Service B")).toBeInTheDocument();
            });

            // Verify Add Role buttons exist for each service
            const addButtons = screen.getAllByText(/Add Role/i);
            expect(addButtons.length).toBe(2);
        });

        it("renders Create button in dialog", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByText(/Add Role/i);
            await user.click(addButtons[0]);

            await waitFor(() => {
                const dialog = screen.getByRole("dialog");
                expect(within(dialog).getByRole("button", { name: "Create" })).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Permissions Tab Tests
    // ============================================================================

    describe("permissions tab", () => {
        it("switches to permissions tab", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByRole("tab", { name: "Permissions" })).toBeInTheDocument();
            });

            await user.click(screen.getByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("Permission Management")).toBeInTheDocument();
                expect(screen.getByText("Create and manage permissions for each service")).toBeInTheDocument();
            });
        });

        it("renders permissions table with data after switching tab", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            // Wait for page to load
            await waitFor(() => {
                expect(screen.getByRole("tab", { name: "Permissions" })).toBeInTheDocument();
            });

            // Click permissions tab
            await user.click(screen.getByRole("tab", { name: "Permissions" }));

            // Wait for permission data to be visible
            await waitFor(() => {
                expect(screen.getByText("Permission Management")).toBeInTheDocument();
            });

            // Verify permissions are displayed
            expect(screen.getByText("users:read")).toBeInTheDocument();
            expect(screen.getByText("Read Users")).toBeInTheDocument();
        });

        it("renders permission data", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("users:read")).toBeInTheDocument();
                expect(screen.getByText("Read Users")).toBeInTheDocument();
                expect(screen.getByText("users:write")).toBeInTheDocument();
                expect(screen.getByText("Write Users")).toBeInTheDocument();
            });
        });

        it("renders Add Permission button for each service", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                const addButtons = screen.getAllByRole("button", { name: /Add Permission/i });
                expect(addButtons.length).toBe(2); // One for each service
            });
        });

        it("renders empty state for service without permissions", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("No permissions created yet for this service")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Hierarchy Tab Tests
    // ============================================================================

    describe("hierarchy tab", () => {
        it("switches to hierarchy tab", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Hierarchy" }));

            await waitFor(() => {
                expect(screen.getByText("Role Hierarchy")).toBeInTheDocument();
                expect(screen.getByText("View role inheritance structure for each service")).toBeInTheDocument();
            });
        });

        it("renders hierarchy tree with role names", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Hierarchy" }));

            await waitFor(() => {
                // Role Hierarchy card should be visible
                expect(screen.getByText("Role Hierarchy")).toBeInTheDocument();
                // Service names should be visible
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });
        });

        it("renders empty state for service without roles in hierarchy tab", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Hierarchy" }));

            await waitFor(() => {
                expect(screen.getByText("No roles defined for this service")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Create Role Dialog Interaction Tests
    // ============================================================================

    describe("create role dialog interactions", () => {
        it("shows form fields including parent role select with existing roles as options", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            // Click the first Add Role button (for Service A which has existing roles)
            const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                const dialog = screen.getByRole("dialog");
                expect(within(dialog).getByLabelText("Role Name")).toBeInTheDocument();
                expect(within(dialog).getByLabelText("Description")).toBeInTheDocument();
                expect(within(dialog).getByLabelText("Parent Role (Optional)")).toBeInTheDocument();
            });

            // The parent role select should have existing roles as options
            const parentSelect = screen.getByLabelText("Parent Role (Optional)");
            expect(parentSelect).toBeInTheDocument();
            // Should have "No parent" option plus both existing roles
            const options = within(parentSelect as HTMLElement).getAllByRole("option");
            expect(options.length).toBe(3); // "No parent" + Admin + Viewer
        });

        it("fills in the create role form and submits", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        expect(formData.get("intent")).toBe("create_role");
                        expect(formData.get("name")).toBe("Editor");
                        expect(formData.get("description")).toBe("Can edit content");
                        expect(formData.get("service_id")).toBe("s1");
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            await user.type(within(dialog).getByLabelText("Role Name"), "Editor");
            await user.type(within(dialog).getByLabelText("Description"), "Can edit content");

            const submitButton = within(dialog).getByRole("button", { name: "Create" });
            await user.click(submitButton);

            // Dialog should close after successful action
            await waitFor(() => {
                expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
            });
        });

        it("closes the create role dialog when Cancel is clicked", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            const cancelButton = within(dialog).getByRole("button", { name: "Cancel" });
            await user.click(cancelButton);

            await waitFor(() => {
                expect(screen.queryByText("Add a new role to this service.")).not.toBeInTheDocument();
            });
        });

        it("closes the create role dialog via the X close button", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const closeButton = screen.getByRole("button", { name: "Close" });
            await user.click(closeButton);

            await waitFor(() => {
                expect(screen.queryByText("Add a new role to this service.")).not.toBeInTheDocument();
            });
        });

        it("opens create role dialog for Service B (no parent role options)", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service B")).toBeInTheDocument();
            });

            // Click the second Add Role button (Service B has no roles)
            const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
            await user.click(addButtons[1]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            // Parent role select should only have "No parent" option since Service B has no roles
            const parentSelect = screen.getByLabelText("Parent Role (Optional)");
            const options = within(parentSelect as HTMLElement).getAllByRole("option");
            expect(options.length).toBe(1); // Only "No parent"
        });
    });

    // ============================================================================
    // Edit Role Dialog Tests
    // ============================================================================

    describe("edit role dialog", () => {
        it("opens edit role dialog from dropdown menu", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            // Open the dropdown menu for the first role
            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            // Click Edit in the dropdown
            await waitFor(() => {
                expect(screen.getByText("Edit")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Edit"));

            // The edit dialog should open
            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText("Edit Role")).toBeInTheDocument();
                expect(screen.getByText("Update role details.")).toBeInTheDocument();
            });

            // Check that form is pre-filled with role data
            const dialog = screen.getByRole("dialog");
            const nameInput = within(dialog).getByLabelText("Role Name");
            expect(nameInput).toHaveValue("Admin");
        });

        it("shows parent role options excluding the role being edited", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            // Open dropdown for Admin role and click Edit
            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Edit")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Edit"));

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            // Parent select should exclude the current role (Admin) but include Viewer
            const parentSelect = screen.getByLabelText("Parent Role (Optional)");
            const options = within(parentSelect as HTMLElement).getAllByRole("option");
            // Should have "No parent" + Viewer (excluding Admin itself)
            expect(options.length).toBe(2);
            expect(options[0]).toHaveTextContent("No parent (root role)");
            expect(options[1]).toHaveTextContent("Viewer");
        });

        it("submits edit role form", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        expect(formData.get("intent")).toBe("update_role");
                        expect(formData.get("role_id")).toBe("r1");
                        expect(formData.get("service_id")).toBe("s1");
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            // Open dropdown and click Edit
            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Edit")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Edit"));

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            const submitButton = within(dialog).getByRole("button", { name: "Save Changes" });
            await user.click(submitButton);

            // Dialog should close after success
            await waitFor(() => {
                expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
            });
        });

        it("closes edit role dialog when Cancel is clicked", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Edit")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Edit"));

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            const cancelButton = within(dialog).getByRole("button", { name: "Cancel" });
            await user.click(cancelButton);

            await waitFor(() => {
                expect(screen.queryByText("Update role details.")).not.toBeInTheDocument();
            });
        });

        it("pre-fills the description field in edit dialog", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Edit")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Edit"));

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            const descInput = within(dialog).getByLabelText("Description");
            expect(descInput).toHaveValue("Full access");
        });
    });

    // ============================================================================
    // Delete Role Tests
    // ============================================================================

    describe("delete role", () => {
        it("opens delete confirmation dialog from dropdown menu", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            // Open the dropdown menu
            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Delete")).toBeInTheDocument();
            });

            // Click Delete
            await user.click(screen.getByText("Delete"));

            // Confirm dialog should appear
            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Role");
                expect(screen.getByText("Are you sure you want to delete this role?")).toBeInTheDocument();
            });
        });

        it("submits delete role when confirmation is accepted", async () => {
            let submittedFormData: Record<string, string> = {};
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        submittedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Delete")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Delete"));

            // Wait for confirm dialog
            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-action")).toBeInTheDocument();
            });

            // Click Delete (confirm action)
            await user.click(screen.getByTestId("confirm-dialog-action"));

            await waitFor(() => {
                expect(submittedFormData.intent).toBe("delete_role");
                expect(submittedFormData.service_id).toBe("s1");
                expect(submittedFormData.role_id).toBe("r1");
            });
        });

        it("cancels delete role when confirmation is rejected", async () => {
            let actionCalled = false;
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async () => {
                        actionCalled = true;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Delete")).toBeInTheDocument();
            });
            await user.click(screen.getByText("Delete"));

            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-cancel")).toBeInTheDocument();
            });

            // Cancel
            await user.click(screen.getByTestId("confirm-dialog-cancel"));

            // Confirm dialog should close and no action should be triggered
            await waitFor(() => {
                expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
            });
            expect(actionCalled).toBe(false);
        });
    });

    // ============================================================================
    // Create Permission Dialog Interaction Tests
    // ============================================================================

    describe("create permission dialog interactions", () => {
        it("opens create permission dialog when clicking Add Permission button", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            // Switch to permissions tab
            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("Permission Management")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Permission/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText("Create Permission")).toBeInTheDocument();
                expect(screen.getByText("Add a new permission to this service.")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            expect(within(dialog).getByLabelText("Permission Code")).toBeInTheDocument();
            expect(within(dialog).getByLabelText("Display Name")).toBeInTheDocument();
            expect(within(dialog).getByLabelText("Description")).toBeInTheDocument();
        });

        it("fills and submits create permission form", async () => {
            let submittedFormData: Record<string, string> = {};
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        submittedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            // Switch to permissions tab
            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("Permission Management")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Permission/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            await user.type(within(dialog).getByLabelText("Permission Code"), "posts:create");
            await user.type(within(dialog).getByLabelText("Display Name"), "Create Posts");
            await user.type(within(dialog).getByLabelText("Description"), "Allows creating posts");

            await user.click(within(dialog).getByRole("button", { name: "Create" }));

            await waitFor(() => {
                expect(submittedFormData.intent).toBe("create_permission");
                expect(submittedFormData.code).toBe("posts:create");
                expect(submittedFormData.name).toBe("Create Posts");
                expect(submittedFormData.description).toBe("Allows creating posts");
                expect(submittedFormData.service_id).toBe("s1");
            });
        });

        it("closes create permission dialog when Cancel is clicked", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("Permission Management")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Permission/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            await user.click(within(dialog).getByRole("button", { name: "Cancel" }));

            await waitFor(() => {
                expect(screen.queryByText("Add a new permission to this service.")).not.toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Delete Permission Tests
    // ============================================================================

    describe("delete permission", () => {
        it("opens delete permission confirmation when clicking trash button", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            // Switch to permissions tab
            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("users:read")).toBeInTheDocument();
            });

            // Find the delete buttons (trash icons) in the first permissions table
            // There are two tables (one per service)
            const permTables = screen.getAllByRole("table");
            const deleteButtons = within(permTables[0]).getAllByRole("button");
            // Click the first delete button
            await user.click(deleteButtons[0]);

            // Confirm dialog should appear
            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Permission");
                expect(screen.getByText("Are you sure you want to delete this permission?")).toBeInTheDocument();
            });
        });

        it("submits delete permission when confirmation is accepted", async () => {
            let submittedFormData: Record<string, string> = {};
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        submittedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("users:read")).toBeInTheDocument();
            });

            const permTables = screen.getAllByRole("table");
            const deleteButtons = within(permTables[0]).getAllByRole("button");
            await user.click(deleteButtons[0]);

            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-action")).toBeInTheDocument();
            });

            await user.click(screen.getByTestId("confirm-dialog-action"));

            await waitFor(() => {
                expect(submittedFormData.intent).toBe("delete_permission");
                expect(submittedFormData.permission_id).toBe("p1");
            });
        });

        it("cancels delete permission when confirmation is rejected", async () => {
            let actionCalled = false;
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async () => {
                        actionCalled = true;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("users:read")).toBeInTheDocument();
            });

            const permTables = screen.getAllByRole("table");
            const deleteButtons = within(permTables[0]).getAllByRole("button");
            await user.click(deleteButtons[0]);

            await waitFor(() => {
                expect(screen.getByTestId("confirm-dialog-cancel")).toBeInTheDocument();
            });

            await user.click(screen.getByTestId("confirm-dialog-cancel"));

            await waitFor(() => {
                expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
            });
            expect(actionCalled).toBe(false);
        });
    });

    // ============================================================================
    // Manage Permissions Dialog Tests
    // ============================================================================

    describe("manage permissions dialog", () => {
        it("opens manage permissions dialog when clicking Permissions button", async () => {
            // Mock the fetch call that openManagePermissions makes
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: true,
                    role: { id: "r1", name: "Admin", permissions: [mockPermissions[0]] },
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            // Click the Permissions button for the Admin role
            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText("Manage Permissions")).toBeInTheDocument();
            });

            vi.restoreAllMocks();
        });

        it("shows permission checkboxes in the manage permissions dialog", async () => {
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: true,
                    role: { id: "r1", name: "Admin", permissions: [mockPermissions[0]] },
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            // Should show permission codes and names
            expect(within(dialog).getByText("users:read")).toBeInTheDocument();
            expect(within(dialog).getByText("Read Users")).toBeInTheDocument();
            expect(within(dialog).getByText("users:write")).toBeInTheDocument();
            expect(within(dialog).getByText("Write Users")).toBeInTheDocument();

            vi.restoreAllMocks();
        });

        it("closes manage permissions dialog when Done is clicked", async () => {
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: true,
                    role: { id: "r1", name: "Admin", permissions: [] },
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            await user.click(within(dialog).getByRole("button", { name: "Done" }));

            await waitFor(() => {
                expect(screen.queryByText("Manage Permissions")).not.toBeInTheDocument();
            });

            vi.restoreAllMocks();
        });

        it("falls back to empty permissions when fetch fails", async () => {
            vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("Network error"));

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            // Dialog should still open with empty permissions (fallback)
            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText("Manage Permissions")).toBeInTheDocument();
            });

            vi.restoreAllMocks();
        });

        it("falls back when fetch returns non-success result", async () => {
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: false,
                    error: "Not found",
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            // Dialog should still open with empty role permissions (fallback)
            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText("Manage Permissions")).toBeInTheDocument();
            });

            vi.restoreAllMocks();
        });

        it("shows empty permissions message when service has no permissions", async () => {
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: true,
                    role: { id: "r1", name: "Admin", permissions: [] },
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            // Create data where service has roles but no permissions
            const dataWithNoPermissions = {
                entries: [
                    {
                        service: mockServices.data[0],
                        roles: mockRoles,
                        permissions: [],
                    },
                ],
                pagination: mockServices.pagination,
            };

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => dataWithNoPermissions,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            // Find only the role action Permissions buttons (not the tab trigger)
            // The role Permissions buttons contain a gear icon and "Permissions" text
            const allPermButtons = screen.getAllByRole("button", { name: /Permissions/i });
            // Filter out the tab trigger (which has role="tab")
            const rolePermButtons = allPermButtons.filter(
                btn => !btn.hasAttribute("data-radix-collection-item") && btn.getAttribute("role") !== "tab"
            );
            await user.click(rolePermButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
                expect(screen.getByText(/No permissions defined for this service/)).toBeInTheDocument();
                expect(screen.getByText(/Create permissions in the Permissions tab first/)).toBeInTheDocument();
            });

            vi.restoreAllMocks();
        });

        it("toggles permission assignment via checkbox", async () => {
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: true,
                    role: { id: "r1", name: "Admin", permissions: [mockPermissions[0]] },
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            let submittedFormData: Record<string, string> = {};
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        submittedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            // Find checkboxes in the dialog
            const dialog = screen.getByRole("dialog");
            const checkboxes = within(dialog).getAllByRole("checkbox");
            expect(checkboxes.length).toBe(2); // Two permissions

            // Toggle the second permission (users:write, not assigned)
            await user.click(checkboxes[1]);

            await waitFor(() => {
                expect(submittedFormData.intent).toBe("assign_permission");
                expect(submittedFormData.role_id).toBe("r1");
                expect(submittedFormData.permission_id).toBe("p2");
            });

            vi.restoreAllMocks();
        });

        it("unassigns a permission via checkbox toggle", async () => {
            const mockFetchResponse = {
                ok: true,
                json: async () => ({
                    success: true,
                    role: { id: "r1", name: "Admin", permissions: [mockPermissions[0]] },
                }),
            };
            vi.spyOn(globalThis, "fetch").mockResolvedValue(mockFetchResponse as Response);

            let submittedFormData: Record<string, string> = {};
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async ({ request }) => {
                        const formData = await request.formData();
                        submittedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
                        return { success: true };
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const permButtons = screen.getAllByRole("button", { name: /Permissions/i });
            await user.click(permButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            const checkboxes = within(dialog).getAllByRole("checkbox");

            // Toggle the first permission (users:read, currently assigned)
            await user.click(checkboxes[0]);

            await waitFor(() => {
                expect(submittedFormData.intent).toBe("remove_permission");
                expect(submittedFormData.role_id).toBe("r1");
                expect(submittedFormData.permission_id).toBe("p1");
            });

            vi.restoreAllMocks();
        });
    });

    // ============================================================================
    // Hierarchy Tab - Orphaned Roles Tests
    // ============================================================================

    describe("hierarchy tab with orphaned roles", () => {
        it("renders orphaned roles section when roles have invalid parent references", async () => {
            const orphanedLoaderData = {
                entries: [
                    {
                        service: mockServices.data[0],
                        roles: [
                            { id: "r1", service_id: "s1", name: "Admin", description: "Full access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
                            { id: "r3", service_id: "s1", name: "Orphan Role", description: "Has invalid parent", parent_role_id: "nonexistent-id", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
                        ],
                        permissions: [],
                    },
                ],
                pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
            };

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => orphanedLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Hierarchy" }));

            await waitFor(() => {
                expect(screen.getByText("Orphaned Roles (invalid parent):")).toBeInTheDocument();
                expect(screen.getByText("Orphan Role")).toBeInTheDocument();
            });
        });

        it("renders role hierarchy with child roles indented under parents", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Hierarchy" }));

            await waitFor(() => {
                // Both Admin (root) and Viewer (child of Admin) should appear
                expect(screen.getByText("Admin")).toBeInTheDocument();
                expect(screen.getByText("Viewer")).toBeInTheDocument();
                // Descriptions should be shown in hierarchy
                expect(screen.getByText("(Full access)")).toBeInTheDocument();
                expect(screen.getByText("(Read only)")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Error Display Tests
    // ============================================================================

    describe("error display in dialogs", () => {
        it("shows error message in create role dialog when action returns error", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                    action: async () => {
                        return Response.json({ error: "Role name already exists" }, { status: 400 });
                    },
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Service A")).toBeInTheDocument();
            });

            const addButtons = screen.getAllByRole("button", { name: /Add Role/i });
            await user.click(addButtons[0]);

            await waitFor(() => {
                expect(screen.getByRole("dialog")).toBeInTheDocument();
            });

            const dialog = screen.getByRole("dialog");
            await user.type(within(dialog).getByLabelText("Role Name"), "Admin");
            await user.click(within(dialog).getByRole("button", { name: "Create" }));

            await waitFor(() => {
                expect(screen.getByText("Role name already exists")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Dropdown Menu Actions Label Tests
    // ============================================================================

    describe("dropdown menu", () => {
        it("shows Actions label in dropdown", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Admin")).toBeInTheDocument();
            });

            const menuButtons = screen.getAllByRole("button", { name: "Open menu" });
            await user.click(menuButtons[0]);

            await waitFor(() => {
                expect(screen.getByText("Actions")).toBeInTheDocument();
                expect(screen.getByText("Edit")).toBeInTheDocument();
                expect(screen.getByText("Delete")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Permission Description in Table Tests
    // ============================================================================

    describe("permissions table details", () => {
        it("renders permission description or dash for missing description", async () => {
            const dataWithMixedDescriptions = {
                entries: [
                    {
                        service: mockServices.data[0],
                        roles: [],
                        permissions: [
                            { id: "p1", service_id: "s1", code: "users:read", name: "Read Users", description: "Can read user data", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
                            { id: "p3", service_id: "s1", code: "users:delete", name: "Delete Users", description: undefined, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
                        ],
                    },
                ],
                pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
            };

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => dataWithMixedDescriptions,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                expect(screen.getByText("Can read user data")).toBeInTheDocument();
                expect(screen.getByText("-")).toBeInTheDocument();
            });
        });

        it("renders table column headers", async () => {
            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => mockLoaderData,
                },
            ]);

            const user = userEvent.setup();
            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await user.click(await screen.findByRole("tab", { name: "Permissions" }));

            await waitFor(() => {
                // There are two tables (one per service), so column headers appear twice
                const permTables = screen.getAllByRole("table");
                expect(permTables.length).toBe(2);
                const firstTable = permTables[0];
                expect(within(firstTable).getByText("Code")).toBeInTheDocument();
                expect(within(firstTable).getByText("Name")).toBeInTheDocument();
                expect(within(firstTable).getByText("Description")).toBeInTheDocument();
                expect(within(firstTable).getByText("Actions")).toBeInTheDocument();
            });
        });
    });

    // ============================================================================
    // Role Inheritance Display Tests
    // ============================================================================

    describe("role inheritance display", () => {
        it("shows 'parent' as fallback when parent role name is not found", async () => {
            const dataWithUnknownParent = {
                entries: [
                    {
                        service: mockServices.data[0],
                        roles: [
                            { id: "r5", service_id: "s1", name: "Child Role", parent_role_id: "unknown-parent-id", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
                        ],
                        permissions: [],
                    },
                ],
                pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
            };

            const RoutesStub = createRoutesStub([
                {
                    path: "/dashboard/roles",
                    Component: WrappedPage,
                    loader: () => dataWithUnknownParent,
                },
            ]);

            render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

            await waitFor(() => {
                expect(screen.getByText("Child Role")).toBeInTheDocument();
                expect(screen.getByText("(inherits from parent)")).toBeInTheDocument();
            });
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
            return new Request("http://localhost/dashboard/roles", {
                method: "POST",
                body: formData,
            });
        }

        it("create_role calls rbacApi.createRole", async () => {
            vi.mocked(rbacApi.createRole).mockResolvedValue({ data: { id: "r1", service_id: "s1", name: "Admin", created_at: "", updated_at: "" } });

            const request = createFormRequest({
                intent: "create_role",
                service_id: "s1",
                name: "Admin",
                description: "Full access",
                parent_role_id: "",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.createRole).toHaveBeenCalledWith(
                "s1",
                { name: "Admin", description: "Full access", parent_role_id: undefined },
                "test-token"
            );
        });

        it("update_role calls rbacApi.updateRole", async () => {
            vi.mocked(rbacApi.updateRole).mockResolvedValue({ data: { id: "r1", service_id: "s1", name: "Super Admin", created_at: "", updated_at: "" } });

            const request = createFormRequest({
                intent: "update_role",
                service_id: "s1",
                role_id: "r1",
                name: "Super Admin",
                description: "",
                parent_role_id: "r2",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.updateRole).toHaveBeenCalledWith(
                "s1",
                "r1",
                { name: "Super Admin", description: undefined, parent_role_id: "r2" },
                "test-token"
            );
        });

        it("delete_role calls rbacApi.deleteRole", async () => {
            vi.mocked(rbacApi.deleteRole).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "delete_role",
                service_id: "s1",
                role_id: "r1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.deleteRole).toHaveBeenCalledWith("s1", "r1", "test-token");
        });

        it("create_permission calls rbacApi.createPermission", async () => {
            vi.mocked(rbacApi.createPermission).mockResolvedValue({ data: { id: "p1", service_id: "s1", code: "users:read", name: "Read Users", created_at: "", updated_at: "" } });

            const request = createFormRequest({
                intent: "create_permission",
                service_id: "s1",
                code: "users:read",
                name: "Read Users",
                description: "Can read users",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.createPermission).toHaveBeenCalledWith(
                { service_id: "s1", code: "users:read", name: "Read Users", description: "Can read users" },
                "test-token"
            );
        });

        it("delete_permission calls rbacApi.deletePermission", async () => {
            vi.mocked(rbacApi.deletePermission).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "delete_permission",
                permission_id: "p1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.deletePermission).toHaveBeenCalledWith("p1", "test-token");
        });

        it("assign_permission calls rbacApi.assignPermissionToRole", async () => {
            vi.mocked(rbacApi.assignPermissionToRole).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "assign_permission",
                role_id: "r1",
                permission_id: "p1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.assignPermissionToRole).toHaveBeenCalledWith("r1", "p1", "test-token");
        });

        it("remove_permission calls rbacApi.removePermissionFromRole", async () => {
            vi.mocked(rbacApi.removePermissionFromRole).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "remove_permission",
                role_id: "r1",
                permission_id: "p1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(rbacApi.removePermissionFromRole).toHaveBeenCalledWith("r1", "p1", "test-token");
        });

        it("get_role_permissions calls rbacApi.getRole", async () => {
            vi.mocked(rbacApi.getRole).mockResolvedValue({
                data: { id: "r1", service_id: "s1", name: "Admin", permissions: [], created_at: "", updated_at: "" },
            });

            const request = createFormRequest({
                intent: "get_role_permissions",
                role_id: "r1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({
                success: true,
                role: { id: "r1", service_id: "s1", name: "Admin", permissions: [], created_at: "", updated_at: "" },
            });
        });

        it("returns error on API failure", async () => {
            vi.mocked(rbacApi.createRole).mockRejectedValue(new Error("Role already exists"));

            const request = createFormRequest({
                intent: "create_role",
                service_id: "s1",
                name: "Admin",
                description: "",
                parent_role_id: "",
            });

            const response = await action({ request, params: {}, context: {} });
            expect(response).toBeInstanceOf(Response);
            const data = await (response as Response).json();
            expect(data.error).toBe("Role already exists");
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
