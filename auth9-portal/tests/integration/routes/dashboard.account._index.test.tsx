import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import AccountProfilePage, { loader, action } from "~/routes/dashboard.account._index";
import { userApi } from "~/services/api";

vi.mock("~/services/api", () => ({
    userApi: {
        getMe: vi.fn(),
        updateMe: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

const mockUser = {
    id: "u1",
    email: "alice@example.com",
    display_name: "Alice Smith",
    avatar_url: "https://example.com/avatar.png",
    mfa_enabled: true,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-01-01T00:00:00Z",
};

function createFormRequest(url: string, data: Record<string, string>): Request {
    const formData = new FormData();
    for (const [key, value] of Object.entries(data)) {
        formData.append(key, value);
    }
    return new Request(url, { method: "POST", body: formData });
}

describe("Account Profile Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader returns user data", async () => {
        vi.mocked(userApi.getMe).mockResolvedValue({ data: mockUser });

        const request = new Request("http://localhost/dashboard/account");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ user: mockUser });
        expect(userApi.getMe).toHaveBeenCalledWith("test-token");
    });

    it("loader redirects to /login when no access token", async () => {
        const { getAccessToken } = await import("~/services/session.server");
        vi.mocked(getAccessToken).mockResolvedValueOnce(null);

        const request = new Request("http://localhost/dashboard/account");
        try {
            await loader({ request, params: {}, context: {} });
            expect.fail("Expected redirect");
        } catch (response) {
            expect((response as Response).status).toBe(302);
            expect((response as Response).headers.get("Location")).toBe("/login");
        }
    });

    it("loader redirects to /login on API error", async () => {
        vi.mocked(userApi.getMe).mockRejectedValue(new Error("Unauthorized"));

        const request = new Request("http://localhost/dashboard/account");
        try {
            await loader({ request, params: {}, context: {} });
            expect.fail("Expected redirect");
        } catch (response) {
            expect((response as Response).status).toBe(302);
            expect((response as Response).headers.get("Location")).toBe("/login");
        }
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action updates profile successfully", async () => {
        vi.mocked(userApi.updateMe).mockResolvedValue({ data: mockUser });

        const request = createFormRequest("http://localhost/dashboard/account", {
            display_name: "Alice Updated",
            avatar_url: "https://example.com/new-avatar.png",
        });

        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ success: true, message: "Profile updated successfully" });
        expect(userApi.updateMe).toHaveBeenCalledWith(
            { display_name: "Alice Updated", avatar_url: "https://example.com/new-avatar.png" },
            "test-token"
        );
    });

    it("action returns error when not authenticated", async () => {
        const { getAccessToken } = await import("~/services/session.server");
        vi.mocked(getAccessToken).mockResolvedValueOnce(null);

        const request = createFormRequest("http://localhost/dashboard/account", {
            display_name: "Test",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "Not authenticated" });
    });

    it("action returns error on API failure", async () => {
        vi.mocked(userApi.updateMe).mockRejectedValue(new Error("Server error"));

        const request = createFormRequest("http://localhost/dashboard/account", {
            display_name: "Test",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "Server error" });
    });

    it("action returns generic error for non-Error throws", async () => {
        vi.mocked(userApi.updateMe).mockRejectedValue("unexpected");

        const request = createFormRequest("http://localhost/dashboard/account", {
            display_name: "Test",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "Failed to update profile" });
    });

    it("action sends undefined for empty fields", async () => {
        vi.mocked(userApi.updateMe).mockResolvedValue({ data: mockUser });

        const request = createFormRequest("http://localhost/dashboard/account", {
            display_name: "",
            avatar_url: "",
        });

        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ success: true, message: "Profile updated successfully" });
        expect(userApi.updateMe).toHaveBeenCalledWith(
            { display_name: undefined, avatar_url: undefined },
            "test-token"
        );
    });

    // ============================================================================
    // Rendering Tests
    // ============================================================================

    it("renders profile form with user data", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountProfilePage,
                loader: () => ({ user: mockUser }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);

        expect(await screen.findByText("Profile")).toBeInTheDocument();
        expect(screen.getByLabelText("Display name")).toBeInTheDocument();
        expect(screen.getByLabelText("Avatar URL")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /Save changes/i })).toBeInTheDocument();
    });

    it("displays user initials in avatar", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountProfilePage,
                loader: () => ({ user: mockUser }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);
        expect(await screen.findByText("AS")).toBeInTheDocument();
    });

    it("displays MFA Enabled", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountProfilePage,
                loader: () => ({ user: mockUser }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);
        expect(await screen.findByText("Enabled")).toBeInTheDocument();
    });

    it("displays MFA Disabled", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountProfilePage,
                loader: () => ({ user: { ...mockUser, mfa_enabled: false } }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);
        expect(await screen.findByText("Disabled")).toBeInTheDocument();
    });

    it("uses email for initials when display_name is empty", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountProfilePage,
                loader: () => ({ user: { ...mockUser, display_name: "" } }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);
        expect(await screen.findByText("A")).toBeInTheDocument();
    });

    it("displays user email", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountProfilePage,
                loader: () => ({ user: mockUser }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);
        expect(await screen.findByText("alice@example.com")).toBeInTheDocument();
    });
});
