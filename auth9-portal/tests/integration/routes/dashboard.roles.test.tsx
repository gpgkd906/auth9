import { createRoutesStub } from "react-router";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import RolesPage from "~/routes/dashboard.roles";

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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
                    Component: RolesPage,
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
});
