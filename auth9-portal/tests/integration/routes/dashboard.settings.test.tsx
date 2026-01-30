import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import SettingsPage, { loader, action } from "~/routes/dashboard.settings";
import { tenantApi } from "~/services/api";

// Mock tenant API
vi.mock("~/services/api", () => ({
    tenantApi: {
        list: vi.fn(),
        update: vi.fn(),
    },
}));

describe("Settings Page", () => {
    const mockTenants = {
        data: [
            {
                id: "1",
                name: "Acme Corp",
                slug: "acme",
                status: "active",
                settings: {
                    branding: {
                        logo_url: "https://example.com/logo.png",
                        primary_color: "#ff5500",
                    },
                },
            },
            {
                id: "2",
                name: "Globex",
                slug: "globex",
                status: "inactive",
                settings: null,
            },
        ],
        pagination: {
            total: 2,
            page: 1,
            total_pages: 1,
        },
    };

    it("renders settings page with tenant list", async () => {
        (tenantApi.list as any).mockResolvedValue(mockTenants);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/settings",
                Component: SettingsPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("Settings")).toBeInTheDocument();
            expect(screen.getByText("Organization Settings")).toBeInTheDocument();
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
            expect(screen.getByText("Globex")).toBeInTheDocument();
        });
    });

    it("displays branding info for tenants with settings", async () => {
        (tenantApi.list as any).mockResolvedValue(mockTenants);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/settings",
                Component: SettingsPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            // Tenant with branding shows logo URL
            expect(screen.getByText("https://example.com/logo.png")).toBeInTheDocument();
            // Tenant without branding shows "No branding"
            expect(screen.getByText("No branding")).toBeInTheDocument();
        });
    });

    it("shows edit dialog when edit button clicked", async () => {
        (tenantApi.list as any).mockResolvedValue(mockTenants);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/settings",
                Component: SettingsPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/settings"]} />);

        // Wait for content and find all edit buttons
        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
        });

        const editButtons = screen.getAllByRole("button");
        // Click the first edit button (for Acme Corp)
        const svgButton = editButtons.find(btn => btn.querySelector("svg"));
        if (svgButton) {
            await user.click(svgButton);
        }

        await waitFor(() => {
            expect(screen.getByRole("dialog")).toBeInTheDocument();
            expect(screen.getByText("Edit Settings: Acme Corp")).toBeInTheDocument();
        });
    });

    it("shows empty state when no tenants", async () => {
        (tenantApi.list as any).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, total_pages: 1 },
        });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/settings",
                Component: SettingsPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("No tenant settings found")).toBeInTheDocument();
        });
    });

    it("displays pagination info", async () => {
        (tenantApi.list as any).mockResolvedValue(mockTenants);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/settings",
                Component: SettingsPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText(/2 tenants/)).toBeInTheDocument();
            expect(screen.getByText(/Page 1 of/)).toBeInTheDocument();
        });
    });
});
