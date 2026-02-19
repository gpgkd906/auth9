import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import AccountSessionsPage, { loader, action } from "~/routes/dashboard.account.sessions";
import { sessionApi } from "~/services/api";

vi.mock("~/services/api", () => ({
    sessionApi: {
        listMySessions: vi.fn(),
        revokeSession: vi.fn(),
        revokeOtherSessions: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    requireIdentityAuthWithUpdate: vi.fn().mockResolvedValue({
        session: {
            identityAccessToken: "test-token",
            refreshToken: "test-refresh-token",
            idToken: "test-id-token",
            identityExpiresAt: Date.now() + 3600000,
        },
        headers: undefined,
    }),
}));

const mockCurrentSession = {
    id: "s1",
    device_name: "Chrome on macOS",
    device_type: "desktop",
    ip_address: "192.168.1.1",
    location: "Tokyo, JP",
    is_current: true,
    last_active_at: new Date().toISOString(),
    created_at: "2024-01-01T00:00:00Z",
};

const mockOtherSession = {
    id: "s2",
    device_name: "Safari on iPhone",
    device_type: "mobile",
    ip_address: "10.0.0.1",
    location: "Osaka, JP",
    is_current: false,
    last_active_at: new Date(Date.now() - 3600000).toISOString(),
    created_at: "2024-01-02T00:00:00Z",
};

const mockTabletSession = {
    id: "s3",
    device_name: "Firefox on iPad",
    device_type: "tablet",
    ip_address: "10.0.0.2",
    location: null,
    is_current: false,
    last_active_at: new Date(Date.now() - 86400000 * 3).toISOString(),
    created_at: "2024-01-03T00:00:00Z",
};

const mockOldSession = {
    id: "s4",
    device_name: null,
    device_type: null,
    ip_address: null,
    location: null,
    is_current: false,
    last_active_at: new Date(Date.now() - 86400000 * 14).toISOString(),
    created_at: "2024-01-04T00:00:00Z",
};

function createFormRequest(data: Record<string, string>): Request {
    const formData = new FormData();
    for (const [key, value] of Object.entries(data)) {
        formData.append(key, value);
    }
    return new Request("http://localhost/dashboard/account/sessions", {
        method: "POST",
        body: formData,
    });
}

describe("Account Sessions Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader returns sessions from API", async () => {
        vi.mocked(sessionApi.listMySessions).mockResolvedValue({
            data: [mockCurrentSession, mockOtherSession],
        });

        const request = new Request("http://localhost/dashboard/account/sessions");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ sessions: [mockCurrentSession, mockOtherSession] });
    });

    it("loader redirects when no access token", async () => {
        const { requireIdentityAuthWithUpdate } = await import("~/services/session.server");
        const redirectResponse = new Response(null, { status: 302, headers: { Location: "/login" } });
        vi.mocked(requireIdentityAuthWithUpdate).mockRejectedValueOnce(redirectResponse);

        const request = new Request("http://localhost/dashboard/account/sessions");
        try {
            await loader({ request, params: {}, context: {} });
            expect.fail("Expected redirect");
        } catch (response) {
            expect((response as Response).status).toBe(302);
            expect((response as Response).headers.get("Location")).toBe("/login");
        }
    });

    it("loader returns empty sessions on error", async () => {
        vi.mocked(sessionApi.listMySessions).mockRejectedValue(new Error("fail"));

        const request = new Request("http://localhost/dashboard/account/sessions");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ sessions: [], error: "Failed to load sessions" });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action revokes a specific session", async () => {
        vi.mocked(sessionApi.revokeSession).mockResolvedValue(undefined);

        const request = createFormRequest({ intent: "revoke", sessionId: "s2" });
        const result = await action({ request, params: {}, context: {} });

        expect(result.status).toBe(302);
        expect(result.headers.get("Location")).toBe("/dashboard/account/sessions");
        expect(sessionApi.revokeSession).toHaveBeenCalledWith("s2", "test-token");
    });

    it("action revokes all other sessions", async () => {
        vi.mocked(sessionApi.revokeOtherSessions).mockResolvedValue(undefined);

        const request = createFormRequest({ intent: "revoke_all" });
        const result = await action({ request, params: {}, context: {} });

        expect(result.status).toBe(302);
        expect(result.headers.get("Location")).toBe("/dashboard/account/sessions");
        expect(sessionApi.revokeOtherSessions).toHaveBeenCalledWith("test-token");
    });

    it("action redirects when not authenticated", async () => {
        const { requireIdentityAuthWithUpdate } = await import("~/services/session.server");
        const redirectResponse = new Response(null, { status: 302, headers: { Location: "/login" } });
        vi.mocked(requireIdentityAuthWithUpdate).mockRejectedValueOnce(redirectResponse);

        const request = createFormRequest({ intent: "revoke", sessionId: "s2" });
        try {
            await action({ request, params: {}, context: {} });
            expect.fail("Expected redirect");
        } catch (response) {
            expect((response as Response).status).toBe(302);
            expect((response as Response).headers.get("Location")).toBe("/login");
        }
    });

    it("action returns error on revoke failure", async () => {
        vi.mocked(sessionApi.revokeSession).mockRejectedValue(new Error("Session not found"));

        const request = createFormRequest({ intent: "revoke", sessionId: "bad-id" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Session not found" });
    });

    it("action returns generic error for non-Error throw", async () => {
        vi.mocked(sessionApi.revokeSession).mockRejectedValue("unexpected");

        const request = createFormRequest({ intent: "revoke", sessionId: "s2" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Operation failed" });
    });

    it("action returns error for invalid intent", async () => {
        const request = createFormRequest({ intent: "invalid" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Invalid action" });
    });

    // ============================================================================
    // Rendering Tests
    // ============================================================================

    it("renders current session card", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockCurrentSession, mockOtherSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);

        expect(await screen.findByText("Current Session")).toBeInTheDocument();
        expect(screen.getByText("Chrome on macOS")).toBeInTheDocument();
        expect(screen.getByText("Current")).toBeInTheDocument();
    });

    it("renders other sessions with revoke button", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockCurrentSession, mockOtherSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);

        expect(await screen.findByText("Other Sessions")).toBeInTheDocument();
        expect(screen.getByText("Safari on iPhone")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /Revoke/i })).toBeInTheDocument();
    });

    it("renders Sign out all button when other sessions exist", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockCurrentSession, mockOtherSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);
        expect(await screen.findByRole("button", { name: /Sign out all/i })).toBeInTheDocument();
    });

    it("renders empty state when no other sessions", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockCurrentSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);

        expect(await screen.findByText("No other active sessions")).toBeInTheDocument();
        expect(screen.queryByRole("button", { name: /Sign out all/i })).not.toBeInTheDocument();
    });

    it("renders security tips", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockCurrentSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);
        expect(await screen.findByText("Security Tips")).toBeInTheDocument();
    });

    it("renders different device types", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({
                    sessions: [mockCurrentSession, mockOtherSession, mockTabletSession],
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);

        await screen.findByText("Current Session");
        expect(screen.getByText("Chrome on macOS")).toBeInTheDocument();
        expect(screen.getByText("Safari on iPhone")).toBeInTheDocument();
        expect(screen.getByText("Firefox on iPad")).toBeInTheDocument();
    });

    it("shows Unknown Device for sessions without device name", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({
                    sessions: [mockCurrentSession, mockOldSession],
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);

        await screen.findByText("Current Session");
        expect(screen.getByText("Unknown Device")).toBeInTheDocument();
    });

    it("shows unable to identify when no current session", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockOtherSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);
        expect(await screen.findByText("Unable to identify current session")).toBeInTheDocument();
    });

    it("displays location when available", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [mockCurrentSession] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);

        await screen.findByText("Current Session");
        expect(screen.getByText(/192\.168\.1\.1/)).toBeInTheDocument();
    });

    it("displays load error when present", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/sessions",
                Component: AccountSessionsPage,
                loader: () => ({ sessions: [], error: "Failed to load sessions" }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/sessions"]} />);
        expect(await screen.findByText("Failed to load sessions")).toBeInTheDocument();
    });
});
