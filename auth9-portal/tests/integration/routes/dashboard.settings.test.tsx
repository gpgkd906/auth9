import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SettingsLayout, { meta } from "~/routes/dashboard.settings";
import OrganizationSettingsPage, { loader, action } from "~/routes/dashboard.settings._index";
import { tenantApi } from "~/services/api";
import type { Tenant } from "~/services/api";

// Mock tenant API
vi.mock("~/services/api", () => ({
    tenantApi: {
        list: vi.fn(),
        update: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("mock-token"),
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

describe("Settings Page", () => {
    it("meta returns correct title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Settings - Auth9" }]);
    });

    const mockTenants = {
        data: [
            {
                id: "1",
                name: "Acme Corp",
                slug: "acme",
                status: "active" as const,
                settings: {
                    branding: {
                        logo_url: "https://example.com/logo.png",
                        primary_color: "#ff5500",
                    },
                },
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
            {
                id: "2",
                name: "Globex",
                slug: "globex",
                status: "inactive" as const,
                settings: {},
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

    it("renders settings page with tenant list", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("Settings")).toBeInTheDocument();
            expect(screen.getByText("Organization Settings")).toBeInTheDocument();
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
            expect(screen.getByText("Globex")).toBeInTheDocument();
        });
    });

    it("displays branding info for tenants with settings", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            // Tenant with branding shows logo URL
            expect(screen.getByText("https://example.com/logo.png")).toBeInTheDocument();
            // Tenant without branding shows "No branding"
            expect(screen.getByText("No branding")).toBeInTheDocument();
        });
    });

    it("shows edit dialog when edit button clicked", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                    },
                ],
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

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
        vi.mocked(tenantApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("No tenant settings found")).toBeInTheDocument();
        });
    });

    it("displays pagination info", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText(/2 tenants/)).toBeInTheDocument();
            expect(screen.getByText(/Page 1 of/)).toBeInTheDocument();
        });
    });

    it("closes edit dialog when cancel button is clicked", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                    },
                ],
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
        });

        // Click edit button
        const editButtons = screen.getAllByRole("button");
        const svgButton = editButtons.find(btn => btn.querySelector("svg"));
        if (svgButton) {
            await user.click(svgButton);
        }

        await waitFor(() => {
            expect(screen.getByRole("dialog")).toBeInTheDocument();
        });

        // Click cancel
        const cancelButton = screen.getByRole("button", { name: "Cancel" });
        await user.click(cancelButton);

        await waitFor(() => {
            expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        });
    });

    it("closes edit dialog on successful form submission", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenants.data[0] });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                        action: () => ({ success: true }),
                    },
                ],
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
        });

        // Click edit button to open dialog
        const editButtons = screen.getAllByRole("button");
        const svgButton = editButtons.find(btn => btn.querySelector("svg"));
        if (svgButton) {
            await user.click(svgButton);
        }

        await waitFor(() => {
            expect(screen.getByRole("dialog")).toBeInTheDocument();
        });

        // Submit the form
        const saveBtn = screen.getByRole("button", { name: /Save Changes/i });
        await user.click(saveBtn);

        // Dialog should close after successful action
        await waitFor(() => {
            expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        });
    });

    it("shows error message in dialog when action returns error", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);
        vi.mocked(tenantApi.update).mockRejectedValue(new Error("Update failed"));

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/settings",
                Component: SettingsLayout,
                children: [
                    {
                        index: true,
                        Component: OrganizationSettingsPage,
                        loader,
                        action: () => ({ error: "Update failed" }),
                    },
                ],
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/settings"]} />);

        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
        });

        // Click edit button
        const editButtons = screen.getAllByRole("button");
        const svgButton = editButtons.find(btn => btn.querySelector("svg"));
        if (svgButton) {
            await user.click(svgButton);
        }

        await waitFor(() => {
            expect(screen.getByRole("dialog")).toBeInTheDocument();
        });

        // Submit the form
        const saveBtn = screen.getByRole("button", { name: /Save Changes/i });
        await user.click(saveBtn);

        // Error should appear
        await waitFor(() => {
            expect(screen.getByText("Update failed")).toBeInTheDocument();
        });
    });
});

describe("Settings action", () => {
    const mockTenant: Tenant = {
        id: "tenant-1",
        name: "Acme Corp",
        slug: "acme",
        status: "active",
        settings: {},
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

    function createFormRequest(data: Record<string, string>) {
        const formData = new FormData();
        for (const [key, value] of Object.entries(data)) {
            formData.append(key, value);
        }
        return new Request("http://localhost/dashboard/settings", { method: "POST", body: formData });
    }

    it("updates tenant settings successfully", async () => {
        vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenant });

        const request = createFormRequest({
            intent: "update_settings",
            id: "tenant-1",
            branding_logo_url: "https://example.com/logo.png",
            branding_primary_color: "#ff0000",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ success: true });
        expect(tenantApi.update).toHaveBeenCalledWith(
            "tenant-1",
            {
                settings: {
                    branding: {
                        logo_url: "https://example.com/logo.png",
                        primary_color: "#ff0000",
                    },
                },
            },
            "mock-token",
        );
    });

    it("omits empty branding values", async () => {
        vi.mocked(tenantApi.update).mockResolvedValue({ data: mockTenant });

        const request = createFormRequest({
            intent: "update_settings",
            id: "tenant-1",
            branding_logo_url: "",
            branding_primary_color: "",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ success: true });
        expect(tenantApi.update).toHaveBeenCalledWith(
            "tenant-1",
            {
                settings: {
                    branding: {
                        logo_url: undefined,
                        primary_color: undefined,
                    },
                },
            },
            "mock-token",
        );
    });

    it("returns error on API failure", async () => {
        vi.mocked(tenantApi.update).mockRejectedValue(new Error("Settings update failed"));

        const request = createFormRequest({
            intent: "update_settings",
            id: "tenant-1",
            branding_logo_url: "",
            branding_primary_color: "",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Settings update failed");
    });

    it("returns error for invalid intent", async () => {
        const request = createFormRequest({
            intent: "invalid",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Invalid intent");
    });

    it("returns error for non-Error thrown objects", async () => {
        vi.mocked(tenantApi.update).mockRejectedValue("some string error");

        const request = createFormRequest({
            intent: "update_settings",
            id: "tenant-1",
            branding_logo_url: "",
            branding_primary_color: "",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Unknown error");
    });
});
