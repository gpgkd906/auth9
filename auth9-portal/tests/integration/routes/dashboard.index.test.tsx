import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import DashboardIndex, { loader } from "~/routes/dashboard._index";
import { tenantApi, userApi, serviceApi, auditApi } from "~/services/api";

// Mock APIs
vi.mock("~/services/api", () => ({
    tenantApi: { list: vi.fn() },
    userApi: { list: vi.fn() },
    serviceApi: { list: vi.fn() },
    auditApi: { list: vi.fn() },
}));

describe("Dashboard Index Page", () => {
    const mockApiResponses = () => {
        vi.mocked(tenantApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 5, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(userApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 12, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 3, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(auditApi.list).mockResolvedValue({
            data: [
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
            pagination: { total: 2, page: 1, per_page: 50, total_pages: 1 },
        });
    };

    it("renders dashboard with stats cards", async () => {
        mockApiResponses();

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("Dashboard")).toBeInTheDocument();
            expect(screen.getByText("Total Tenants")).toBeInTheDocument();
            expect(screen.getByText("Active Users")).toBeInTheDocument();
            expect(screen.getByText("Services")).toBeInTheDocument();
        });
    });

    it("displays stats values from loader data", async () => {
        mockApiResponses();

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("5")).toBeInTheDocument(); // tenants
            expect(screen.getByText("12")).toBeInTheDocument(); // users
            expect(screen.getByText("3")).toBeInTheDocument(); // services
        });
    });

    it("renders recent activity list", async () => {
        mockApiResponses();

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("Recent Activity")).toBeInTheDocument();
            expect(screen.getByText(/CREATE • tenant/)).toBeInTheDocument();
            expect(screen.getByText(/UPDATE • user/)).toBeInTheDocument();
        });
    });

    it("shows empty state when no audit logs", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(userApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(auditApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 50, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("No recent activity")).toBeInTheDocument();
        });
    });
});
