import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
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
        ],
        pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
    };

    const mockRoles = [
        { id: "r1", service_id: "s1", name: "Admin", description: "Full access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
    ];

    const mockPermissions: unknown[] = [];

    const mockLoaderData = {
        entries: [
            {
                service: mockServices.data[0],
                roles: mockRoles,
                permissions: mockPermissions,
            },
        ],
        pagination: mockServices.pagination,
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

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

    it("opens create role dialog", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/roles",
                Component: RolesPage,
                loader: () => mockLoaderData,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/roles"]} />);

        // Add Role button
        const addButton = await screen.findByRole("button", { name: /Add Role/i });
        await user.click(addButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add a new role to this service.")).toBeInTheDocument();
    });
});
