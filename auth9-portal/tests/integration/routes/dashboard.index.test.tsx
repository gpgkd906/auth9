import { createRoutesStub, Outlet } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import DashboardIndex, { loader } from "~/routes/dashboard._index";
import { tenantApi, userApi, serviceApi, auditApi } from "~/services/api";

// Mock APIs (used internally by loader)
vi.mock("~/services/api", () => ({
    tenantApi: { list: vi.fn() },
    userApi: { list: vi.fn() },
    serviceApi: { list: vi.fn() },
    auditApi: { list: vi.fn() },
}));

// Mock the session server
vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("mock-access-token"),
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

import { getAccessToken } from "~/services/session.server";

describe("Dashboard Index Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    const mockLoaderData = {
        totals: {
            tenants: 5,
            users: 12,
            services: 3,
        },
        audits: [
            {
                id: 1,
                action: "CREATE",
                resource_type: "tenant",
                created_at: new Date().toISOString(),
            },
            {
                id: 2,
                action: "UPDATE",
                resource_type: "user",
                created_at: new Date().toISOString(),
            },
        ],
    };
    const mockOutletContext = {
        activeTenant: {
            tenant: {
                id: "tenant-1",
                name: "Acme Corp",
            },
        },
        tenants: [],
        currentUser: null,
    };

    function renderDashboardWithContext(loaderData = mockLoaderData) {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: () => <Outlet context={mockOutletContext} />,
                children: [
                    {
                        index: true,
                        Component: DashboardIndex,
                        loader: () => loaderData,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);
    }

    it("renders dashboard with stats cards", async () => {
        renderDashboardWithContext();

        await waitFor(() => {
            expect(screen.getByText("Acme Corp")).toBeInTheDocument();
            expect(screen.getByText("Total Tenants")).toBeInTheDocument();
            expect(screen.getByText("Active Users")).toBeInTheDocument();
            expect(screen.getByText("Services")).toBeInTheDocument();
        });
    });

    it("displays stats values from loader data", async () => {
        renderDashboardWithContext();

        await waitFor(() => {
            expect(screen.getByText("5")).toBeInTheDocument(); // tenants
            expect(screen.getByText("12")).toBeInTheDocument(); // users
            expect(screen.getByText("3")).toBeInTheDocument(); // services
        });
    });

    it("renders recent activity list", async () => {
        renderDashboardWithContext();

        await waitFor(() => {
            expect(screen.getByText("Recent Activity")).toBeInTheDocument();
            expect(screen.getByText(/CREATE • tenant/)).toBeInTheDocument();
            expect(screen.getByText(/UPDATE • user/)).toBeInTheDocument();
        });
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    describe("loader", () => {
        beforeEach(() => {
            vi.mocked(tenantApi.list).mockResolvedValue({
                data: [],
                pagination: { page: 1, per_page: 1, total: 5, total_pages: 5 },
            });
            vi.mocked(userApi.list).mockResolvedValue({
                data: [],
                pagination: { page: 1, per_page: 1, total: 12, total_pages: 12 },
            });
            vi.mocked(serviceApi.list).mockResolvedValue({
                data: [],
                pagination: { page: 1, per_page: 1, total: 3, total_pages: 3 },
            });
            vi.mocked(auditApi.list).mockResolvedValue({
                data: [
                    { id: 1, action: "CREATE", resource_type: "tenant", created_at: new Date().toISOString() },
                ],
                pagination: { page: 1, per_page: 5, total: 1, total_pages: 1 },
            });
        });

        it("returns totals and audits from APIs", async () => {
            const request = new Request("http://localhost/dashboard");
            const result = await loader({ request, params: {}, context: {} });
            expect(result).toEqual({
                totals: { tenants: 5, users: 12, services: 3 },
                audits: expect.any(Array),
            });
        });

        it("passes page and perPage from search params", async () => {
            const request = new Request("http://localhost/dashboard?page=2&perPage=10");
            await loader({ request, params: {}, context: {} });
            expect(auditApi.list).toHaveBeenCalledWith(2, 10, "mock-access-token");
        });

        it("defaults page=1 and perPage=5", async () => {
            const request = new Request("http://localhost/dashboard");
            await loader({ request, params: {}, context: {} });
            expect(auditApi.list).toHaveBeenCalledWith(1, 5, "mock-access-token");
        });

        it("redirects to /login when no access token", async () => {
            vi.mocked(getAccessToken).mockResolvedValueOnce(null);
            const request = new Request("http://localhost/dashboard");
            await expect(loader({ request, params: {}, context: {} })).rejects.toEqual(
                expect.objectContaining({ status: 302 })
            );
        });
    });

    it("shows empty state when no audit logs", async () => {
        const emptyLoaderData = {
            totals: {
                tenants: 0,
                users: 0,
                services: 0,
            },
            audits: [],
        };

        renderDashboardWithContext(emptyLoaderData);

        await waitFor(() => {
            expect(screen.getByText("No recent activity")).toBeInTheDocument();
        });
    });
});
