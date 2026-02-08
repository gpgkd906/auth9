import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import Dashboard, { loader, meta } from "~/routes/dashboard";
import { userApi } from "~/services/api";

vi.mock("~/services/api", () => ({
    userApi: {
        getMe: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    requireAuth: vi.fn().mockResolvedValue(undefined),
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

const mockUser = {
    id: "user-1",
    display_name: "John Doe",
    email: "john@example.com",
    avatar_url: "https://example.com/avatar.png",
};

function createDashboardStub(currentUser = mockUser) {
    return createRoutesStub([
        {
            path: "/dashboard",
            Component: Dashboard,
            loader: () => ({ currentUser }),
            children: [
                {
                    index: true,
                    Component: () => <div>Dashboard Home</div>,
                },
                {
                    path: "tenants",
                    Component: () => <div>Tenants Page</div>,
                },
                {
                    path: "users",
                    Component: () => <div>Users Page</div>,
                },
                {
                    path: "services",
                    Component: () => <div>Services Page</div>,
                },
                {
                    path: "roles",
                    Component: () => <div>Roles Page</div>,
                },
                {
                    path: "analytics",
                    Component: () => <div>Analytics Page</div>,
                },
                {
                    path: "security/alerts",
                    Component: () => <div>Security Page</div>,
                },
                {
                    path: "audit-logs",
                    Component: () => <div>Audit Page</div>,
                },
                {
                    path: "settings",
                    Component: () => <div>Settings Page</div>,
                },
                {
                    path: "account",
                    Component: () => <div>Account Page</div>,
                },
            ],
        },
    ]);
}

describe("Dashboard Layout", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    // ============================================================================
    // Meta Tests
    // ============================================================================

    it("meta returns correct title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Dashboard - Auth9" }]);
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader returns current user data", async () => {
        vi.mocked(userApi.getMe).mockResolvedValue({ data: mockUser });

        const request = new Request("http://localhost/dashboard");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ currentUser: mockUser });
    });

    it("loader returns null currentUser when API fails", async () => {
        vi.mocked(userApi.getMe).mockRejectedValue(new Error("fail"));

        const request = new Request("http://localhost/dashboard");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ currentUser: null });
    });

    it("loader returns null currentUser when no access token", async () => {
        const { getAccessToken } = await import("~/services/session.server");
        vi.mocked(getAccessToken).mockResolvedValueOnce(null);

        const request = new Request("http://localhost/dashboard");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ currentUser: null });
    });

    // ============================================================================
    // Rendering Tests
    // ============================================================================

    it("renders dashboard sidebar and navigation", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getAllByText("Auth9").length).toBeGreaterThanOrEqual(1);
        });
        expect(screen.getByText("Overview")).toBeInTheDocument();
        expect(screen.getByText("Tenants")).toBeInTheDocument();
        expect(screen.getByText("Users")).toBeInTheDocument();
        expect(screen.getByText("Services")).toBeInTheDocument();
        expect(screen.getByText("Roles")).toBeInTheDocument();
        expect(screen.getByText("Analytics")).toBeInTheDocument();
        expect(screen.getAllByText("Security").length).toBeGreaterThanOrEqual(1);
        expect(screen.getByText("Audit Logs")).toBeInTheDocument();
        expect(screen.getByText("Settings")).toBeInTheDocument();

        // Check user info
        expect(screen.getByText("John Doe")).toBeInTheDocument();
    });

    it("renders navigation section titles", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("Main")).toBeInTheDocument();
        });
        // "Security" appears both as a section title and nav item
        expect(screen.getAllByText("Security").length).toBeGreaterThanOrEqual(2);
        expect(screen.getByText("System")).toBeInTheDocument();
    });

    it("displays user email", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("john@example.com")).toBeInTheDocument();
        });
    });

    it("shows user initials in avatar", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("JD")).toBeInTheDocument();
        });
    });

    it("falls back to email when display_name is empty", async () => {
        const RoutesStub = createDashboardStub({
            ...mockUser,
            display_name: "",
        });
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        // Both displayName and email resolve to the same value, so use getAllByText
        await waitFor(() => {
            expect(screen.getAllByText("john@example.com").length).toBeGreaterThanOrEqual(1);
        });
    });

    it("falls back to 'User' when currentUser is null", async () => {
        const RoutesStub = createDashboardStub(null as unknown as typeof mockUser);
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            // displayName falls back to "User", initials will be "U"
            expect(screen.getAllByText("User").length).toBeGreaterThanOrEqual(1);
        });
    });

    it("renders skip to content link", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("Skip to main content")).toBeInTheDocument();
        });
    });

    it("renders sign out link", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            const signOut = screen.getByTitle("Sign out");
            expect(signOut).toBeInTheDocument();
            expect(signOut.closest("a")).toHaveAttribute("href", "/logout");
        });
    });

    it("toggles mobile sidebar on button click", async () => {
        const user = userEvent.setup();
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByLabelText("Open sidebar")).toBeInTheDocument();
        });

        // Open sidebar
        await user.click(screen.getByLabelText("Open sidebar"));
        expect(screen.getByLabelText("Close sidebar")).toBeInTheDocument();

        // Close sidebar
        await user.click(screen.getByLabelText("Close sidebar"));
        expect(screen.getByLabelText("Open sidebar")).toBeInTheDocument();
    });

    it("renders account link to user profile", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getByText("John Doe")).toBeInTheDocument();
        });

        const accountLink = screen.getByText("John Doe").closest("a");
        expect(accountLink).toHaveAttribute("href", "/dashboard/account");
    });

    it("highlights active navigation item", async () => {
        const RoutesStub = createDashboardStub();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            const tenantsLink = screen.getAllByText("Tenants").find(el => el.tagName === "A" || el.closest("a"));
            expect(tenantsLink?.closest("a")).toHaveAttribute("aria-current", "page");
        });
    });
});
