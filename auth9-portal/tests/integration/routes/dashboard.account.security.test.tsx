import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import AccountSecurityPage, { action } from "~/routes/dashboard.account.security";
import { passwordApi } from "~/services/api";

vi.mock("~/services/api", () => ({
    passwordApi: {
        changePassword: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
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

function createFormRequest(url: string, data: Record<string, string>): Request {
    const formData = new FormData();
    for (const [key, value] of Object.entries(data)) {
        formData.append(key, value);
    }
    return new Request(url, { method: "POST", body: formData });
}

describe("Account Security Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action returns error when fields are missing", async () => {
        const request = createFormRequest("http://localhost/dashboard/account/security", {
            currentPassword: "",
            newPassword: "",
            confirmPassword: "",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "All password fields are required" });
    });

    it("action returns error when new password is too short", async () => {
        const request = createFormRequest("http://localhost/dashboard/account/security", {
            currentPassword: "oldpass123",
            newPassword: "short",
            confirmPassword: "short",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "New password must be at least 8 characters" });
    });

    it("action returns error when passwords don't match", async () => {
        const request = createFormRequest("http://localhost/dashboard/account/security", {
            currentPassword: "oldpass123",
            newPassword: "newpass1234",
            confirmPassword: "different123",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "New passwords do not match" });
    });

    it("action changes password successfully", async () => {
        vi.mocked(passwordApi.changePassword).mockResolvedValue(undefined);

        const request = createFormRequest("http://localhost/dashboard/account/security", {
            currentPassword: "oldpass123",
            newPassword: "newpass1234",
            confirmPassword: "newpass1234",
        });

        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ success: true, message: "Password changed successfully" });
        expect(passwordApi.changePassword).toHaveBeenCalledWith("oldpass123", "newpass1234", "test-token");
    });

    it("action returns error on API failure", async () => {
        vi.mocked(passwordApi.changePassword).mockRejectedValue(new Error("Current password is incorrect"));

        const request = createFormRequest("http://localhost/dashboard/account/security", {
            currentPassword: "wrongpass",
            newPassword: "newpass1234",
            confirmPassword: "newpass1234",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "Current password is incorrect" });
    });

    it("action returns generic error for non-Error throws", async () => {
        vi.mocked(passwordApi.changePassword).mockRejectedValue("unexpected");

        const request = createFormRequest("http://localhost/dashboard/account/security", {
            currentPassword: "oldpass123",
            newPassword: "newpass1234",
            confirmPassword: "newpass1234",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ error: "Failed to change password" });
    });

    // ============================================================================
    // Rendering Tests
    // ============================================================================

    it("renders change password form", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/security",
                Component: AccountSecurityPage,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/security"]} />);

        expect(await screen.findByText("Change Password")).toBeInTheDocument();
        expect(screen.getByText("Update your account password. You will need to enter your current password.")).toBeInTheDocument();
        expect(screen.getByLabelText("Current password")).toBeInTheDocument();
        expect(screen.getByLabelText("New password")).toBeInTheDocument();
        expect(screen.getByLabelText("Confirm new password")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /Change password/i })).toBeInTheDocument();
    });

    it("displays password minimum length hint", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/security",
                Component: AccountSecurityPage,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/security"]} />);

        expect(await screen.findByText("Must be at least 8 characters")).toBeInTheDocument();
    });
});
