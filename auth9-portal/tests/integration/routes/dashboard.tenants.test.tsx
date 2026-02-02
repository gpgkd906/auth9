import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import TenantsIndexPage from "~/routes/dashboard.tenants._index";
import { tenantApi } from "~/services/api";

// Mock the tenant API
vi.mock("~/services/api", () => ({
    tenantApi: {
        list: vi.fn(),
        create: vi.fn(),
        update: vi.fn(),
        delete: vi.fn(),
    },
}));

describe("Tenants Page", () => {
    const mockTenants = {
        data: [
            {
                id: "1",
                name: "Acme Corp",
                slug: "acme",
                settings: {},
                status: "active" as const,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
            {
                id: "2",
                name: "Globex",
                slug: "globex",
                settings: {},
                status: "inactive" as const,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        ],
        pagination: {
            total: 2,
            page: 1,
            per_page: 20,
            total_pages: 1,
        },
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

    it("renders tenant list from loader", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: TenantsIndexPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
            expect(screen.getByText("Globex")).toBeInTheDocument();
        });
    });

    it("displays create tenant dialog when button clicked", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: TenantsIndexPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for the button to be rendered
        const createButton = await screen.findByText("Create Tenant");
        await user.click(createButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add a new tenant to the system. Slug must be unique.")).toBeInTheDocument();
    });

    it("renders empty state when no tenants found", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: TenantsIndexPage,
                loader: () => ({
                    data: [],
                    pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByText("No tenants found")).toBeInTheDocument();
        });
    });
});
