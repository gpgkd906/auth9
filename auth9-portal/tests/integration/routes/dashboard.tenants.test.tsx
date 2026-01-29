import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import TenantsPage, { loader, action } from "~/routes/dashboard.tenants";
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
                status: "active",
                updated_at: new Date().toISOString(),
            },
            {
                id: "2",
                name: "Globex",
                slug: "globex",
                status: "inactive",
                updated_at: new Date().toISOString(),
            },
        ],
        pagination: {
            total: 2,
            page: 1,
            total_pages: 1,
        },
    };

    it("renders tenant list from loader", async () => {
        (tenantApi.list as any).mockResolvedValue(mockTenants);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/tenants",
                Component: TenantsPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
            expect(screen.getByText("Globex")).toBeInTheDocument();
        });
    });

    it("displays create tenant dialog when button clicked", async () => {
        (tenantApi.list as any).mockResolvedValue(mockTenants);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/tenants",
                Component: TenantsPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for the button to be rendered
        const createButton = await screen.findByText("Create Tenant");
        await user.click(createButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add a new tenant to the system. Slug must be unique.")).toBeInTheDocument();
    });

    it("renders empty state when no tenants found", async () => {
        (tenantApi.list as any).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, total_pages: 1 }
        });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/tenants",
                Component: TenantsPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByText("No tenants found")).toBeInTheDocument();
        });
    });
});
