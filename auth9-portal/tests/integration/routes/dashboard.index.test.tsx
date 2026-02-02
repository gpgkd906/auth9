import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import DashboardIndex from "~/routes/dashboard._index";

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
}));

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

    it("renders dashboard with stats cards", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader: () => mockLoaderData,
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
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader: () => mockLoaderData,
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
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader: () => mockLoaderData,
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
        const emptyLoaderData = {
            totals: {
                tenants: 0,
                users: 0,
                services: 0,
            },
            audits: [],
        };

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: DashboardIndex,
                loader: () => emptyLoaderData,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("No recent activity")).toBeInTheDocument();
        });
    });
});
