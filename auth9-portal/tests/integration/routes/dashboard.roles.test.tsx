import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import RolesPage, { loader } from "~/routes/dashboard.roles";
import { serviceApi, rbacApi } from "~/services/api";

// Mock the APIs
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
    },
}));

describe("Roles Page", () => {
    const mockServices = {
        data: [
            { id: "s1", name: "Service A", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ],
        pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
    };

    const mockRoles = {
        data: [
            { id: "r1", service_id: "s1", name: "Admin", description: "Full access", created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ],
    };

    const mockPermissions = {
        data: [],
    };

    it("renders roles grouped by service", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(rbacApi.listRoles).mockResolvedValue(mockRoles);
        vi.mocked(rbacApi.listPermissions).mockResolvedValue(mockPermissions);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/roles",
                Component: RolesPage,
                loader,
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
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);
        vi.mocked(rbacApi.listRoles).mockResolvedValue(mockRoles);
        vi.mocked(rbacApi.listPermissions).mockResolvedValue(mockPermissions);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/roles",
                Component: RolesPage,
                loader,
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
