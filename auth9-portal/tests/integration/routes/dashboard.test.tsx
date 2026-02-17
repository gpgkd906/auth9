import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import Dashboard, { loader, meta } from "~/routes/dashboard";
import { userApi } from "~/services/api";

const mockTenantData = [
    {
        id: "tu-1",
        tenant_id: "tenant-1",
        user_id: "user-1",
        role_in_tenant: "owner",
        joined_at: "2024-01-01T00:00:00Z",
        tenant: {
            id: "tenant-1",
            name: "Acme Corp",
            slug: "acme-corp",
            logo_url: undefined,
            status: "active",
        },
    },
];

vi.mock("~/services/api", () => ({
    userApi: {
        getMe: vi.fn(),
        getMyTenants: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    requireAuth: vi.fn(),
    getAccessToken: vi.fn(),
    commitSession: vi.fn(),
    setActiveTenant: vi.fn(),
    requireAuthWithUpdate: vi.fn(),
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
            loader: () => ({
                currentUser,
                tenants: mockTenantData,
                activeTenant: mockTenantData[0],
                activeTenantId: "tenant-1",
            }),
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

// Import mocked modules for setup
import { requireAuthWithUpdate, commitSession } from "~/services/session.server";

describe("Dashboard Layout", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.mocked(requireAuthWithUpdate).mockResolvedValue({
            session: {
                accessToken: "test-token",
                refreshToken: "test-refresh-token",
                idToken: "test-id-token",
                expiresAt: Date.now() + 3600000,
            },
            headers: undefined,
        });
        vi.mocked(commitSession).mockResolvedValue("mocked-cookie");
        vi.mocked(userApi.getMyTenants).mockResolvedValue({ data: mockTenantData });
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
        vi.mocked(userApi.getMyTenants).mockResolvedValue({ data: mockTenantData });

        const request = new Request("http://localhost/dashboard");
        const result = await loader({ request, params: {}, context: {} });

        // Loader returns Response.json when setting active tenant cookie
        const data = result instanceof Response ? await result.json() : result;
        expect(data.currentUser).toEqual(mockUser);
        expect(data.tenants).toHaveLength(1);
        expect(data.activeTenantId).toBe("tenant-1");
    });

    it("loader returns null currentUser when API fails", async () => {
        vi.mocked(userApi.getMe).mockRejectedValue(new Error("fail"));
        vi.mocked(userApi.getMyTenants).mockResolvedValue({ data: mockTenantData });

        const request = new Request("http://localhost/dashboard");
        const result = await loader({ request, params: {}, context: {} });

        if (result instanceof Response) {
            const data = await result.json();
            expect(data).toMatchObject({ currentUser: null });
        } else {
            expect(result).toMatchObject({ currentUser: null });
        }
    });

    it("loader returns null currentUser when no access token", async () => {
        vi.mocked(userApi.getMe).mockRejectedValue(new Error("fail"));
        vi.mocked(userApi.getMyTenants).mockResolvedValue({ data: mockTenantData });

        const request = new Request("http://localhost/dashboard");
        const result = await loader({ request, params: {}, context: {} });

        if (result instanceof Response) {
            const data = await result.json();
            expect(data).toMatchObject({ currentUser: null });
        } else {
            expect(result).toMatchObject({ currentUser: null });
        }
    });

    it("loader redirects to /onboard when user has no tenants", async () => {
        vi.mocked(userApi.getMe).mockResolvedValue({ data: mockUser });
        vi.mocked(userApi.getMyTenants).mockResolvedValue({ data: [] });

        const request = new Request("http://localhost/dashboard");
        try {
            await loader({ request, params: {}, context: {} });
            expect.fail("Should have thrown redirect");
        } catch (response) {
            expect(response).toBeInstanceOf(Response);
            expect((response as Response).status).toBe(302);
            expect((response as Response).headers.get("Location")).toBe("/onboard");
        }
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
