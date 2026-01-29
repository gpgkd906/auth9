import { createRemixStub } from "@remix-run/testing";
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
            { id: "s1", name: "Service A" },
        ],
        pagination: { total: 1, page: 1, total_pages: 1 },
    };

    const mockRoles = {
        data: [
            { id: "r1", name: "Admin", description: "Full access" },
        ],
    };

    const mockPermissions = {
        data: [],
    };

    it("renders roles grouped by service", async () => {
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (rbacApi.listRoles as any).mockResolvedValue(mockRoles);
        (rbacApi.listPermissions as any).mockResolvedValue(mockPermissions);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/roles",
                Component: RolesPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/roles"]} />);

        await waitFor(() => {
            expect(screen.getByText("Service A")).toBeInTheDocument();
            expect(screen.getByText("Admin")).toBeInTheDocument();
            expect(screen.getByText("- Full access")).toBeInTheDocument();
        });
    });

    it("opens create role dialog", async () => {
        (serviceApi.list as any).mockResolvedValue(mockServices);
        (rbacApi.listRoles as any).mockResolvedValue(mockRoles);
        (rbacApi.listPermissions as any).mockResolvedValue(mockPermissions);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/roles",
                Component: RolesPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/roles"]} />);

        // Add Role button
        const addButton = await screen.findByRole("button", { name: /Add Role/i });
        await user.click(addButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add a new role to this service.")).toBeInTheDocument();
    });
});
