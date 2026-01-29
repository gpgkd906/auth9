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
        ],
        pagination: { total: 1, page: 1, total_pages: 1 },
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
});
