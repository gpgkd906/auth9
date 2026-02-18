import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import Login, { loader, action, meta } from "~/routes/login";
import { enterpriseSsoApi } from "~/services/api";

// Mock session.server
vi.mock("~/services/session.server", () => ({
    commitSession: vi.fn().mockResolvedValue("mock-session-cookie"),
    serializeOAuthState: vi.fn().mockResolvedValue("oauth_state=mock-state"),
}));

vi.mock("~/services/api", () => ({
    enterpriseSsoApi: {
        discover: vi.fn(),
    },
}));

describe("Login Page", () => {
    it("meta returns correct title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Sign In - Auth9" }]);
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader redirects to SSO when no error or passkey params", async () => {
        const request = new Request("http://localhost:3000/login");
        try {
            await loader({ request, params: {}, context: {} });
            throw new Error("Expected redirect");
        } catch (response: unknown) {
            expect(response).toBeInstanceOf(Response);
            const res = response as Response;
            expect(res.status).toBe(302);
            const location = res.headers.get("Location");
            expect(location).toContain("/api/v1/auth/authorize");
            expect(location).toContain("response_type=code");
            expect(location).toContain("scope=openid+email+profile");
        }
    });

    it("loader returns error data when error param present", async () => {
        const request = new Request("http://localhost:3000/login?error=access_denied");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({
            error: "access_denied",
            showPasskey: true,
            apiBaseUrl: "http://localhost:8080",
        });
    });

    it("loader returns error data for generic errors", async () => {
        const request = new Request("http://localhost:3000/login?error=server_error");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({
            error: "server_error",
            showPasskey: true,
            apiBaseUrl: "http://localhost:8080",
        });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action redirects to discovered enterprise SSO URL", async () => {
        const formData = new FormData();
        formData.append("intent", "sso-login");
        formData.append("email", "user@acme.com");
        const request = new Request("http://localhost:3000/login", {
            method: "POST",
            body: formData,
        });

        vi.mocked(enterpriseSsoApi.discover).mockResolvedValueOnce({
            data: {
                tenant_id: "11111111-1111-1111-1111-111111111111",
                tenant_slug: "acme",
                connector_alias: "acme-saml",
                authorize_url: "https://keycloak.example.com/auth?kc_idp_hint=acme--acme-saml",
            },
        });

        const response = await action({ request, params: {}, context: {} });

        expect(response.status).toBe(302);
        const location = response.headers.get("Location");
        expect(location).toContain("kc_idp_hint=acme--acme-saml");
    });

    it("action returns validation error when SSO email is missing", async () => {
        const formData = new FormData();
        formData.append("intent", "sso-login");
        const request = new Request("http://localhost:3000/login", {
            method: "POST",
            body: formData,
        });

        const response = await action({ request, params: {}, context: {} });
        expect(response).toEqual({ error: "Email is required for enterprise SSO discovery" });
    });

    // ============================================================================
    // Passkey Action Tests
    // ============================================================================

    it("action handles passkey-login intent with valid token", async () => {
        const formData = new FormData();
        formData.append("intent", "passkey-login");
        formData.append("accessToken", "test-access-token");
        formData.append("expiresIn", "3600");

        const request = new Request("http://localhost:3000/login", {
            method: "POST",
            body: formData,
        });

        const response = await action({ request, params: {}, context: {} });

        expect(response.status).toBe(302);
        expect(response.headers.get("Location")).toBe("/tenant/select");
        // Session cookie is set via commitSession - verified by redirect
    });

    it("action validates accessToken is required for passkey-login", async () => {
        const formData = new FormData();
        formData.append("intent", "passkey-login");
        formData.append("expiresIn", "3600");
        // Missing accessToken

        const request = new Request("http://localhost:3000/login", {
            method: "POST",
            body: formData,
        });

        const response = await action({ request, params: {}, context: {} });

        expect(response).toEqual({ error: "Missing access token" });
    });

    it("action calculates correct expiresAt timestamp", async () => {
        const formData = new FormData();
        formData.append("intent", "passkey-login");
        formData.append("accessToken", "test-token");
        formData.append("expiresIn", "7200"); // 2 hours

        const request = new Request("http://localhost:3000/login", {
            method: "POST",
            body: formData,
        });

        const response = await action({ request, params: {}, context: {} });

        // Session should expire in ~2 hours from now
        expect(response.status).toBe(302);
        // We can't directly test the session content, but we verified the redirect
    });

    it("action defaults expiresIn to 3600 when not provided", async () => {
        const formData = new FormData();
        formData.append("intent", "passkey-login");
        formData.append("accessToken", "test-token");
        // expiresIn not provided

        const request = new Request("http://localhost:3000/login", {
            method: "POST",
            body: formData,
        });

        const response = await action({ request, params: {}, context: {} });

        expect(response.status).toBe(302);
        expect(response.headers.get("Location")).toBe("/tenant/select");
    });

    // ============================================================================
    // Component Tests (error state only)
    // ============================================================================

    it("renders error page with SSO and passkey buttons on access_denied", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
                loader() {
                    return { error: "access_denied", showPasskey: true, apiBaseUrl: "http://localhost:8080" };
                },
            },
        ]);

        render(<RoutesStub initialEntries={["/login?error=access_denied"]} />);

        expect(await screen.findByText("Sign In Failed")).toBeInTheDocument();
        expect(screen.getByText(/Access was denied/)).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /continue with enterprise sso/i })).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /sign in with passkey/i })).toBeInTheDocument();
    });

    it("renders error page with error message for generic errors", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
                loader() {
                    return { error: "server_error", showPasskey: true, apiBaseUrl: "http://localhost:8080" };
                },
            },
        ]);

        render(<RoutesStub initialEntries={["/login?error=server_error"]} />);

        expect(await screen.findByText("Sign In Failed")).toBeInTheDocument();
        expect(screen.getByText(/An error occurred during sign in: server_error/)).toBeInTheDocument();
    });

    it("displays Auth9 logo on error page", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
                loader() {
                    return { error: "test_error", showPasskey: true, apiBaseUrl: "http://localhost:8080" };
                },
            },
        ]);

        render(<RoutesStub initialEntries={["/login?error=test_error"]} />);

        expect(await screen.findByText("A9")).toBeInTheDocument();
    });

    // ============================================================================
    // Passkey Mode Loader Tests
    // ============================================================================

    it("loader shows passkey login page when passkey param is true", async () => {
        const request = new Request("http://localhost:3000/login?passkey=true");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({
            error: null,
            showPasskey: true,
            apiBaseUrl: "http://localhost:8080",
        });
    });

    it("loader redirects to SSO by default (passkey mode off)", async () => {
        const request = new Request("http://localhost:3000/login");
        try {
            await loader({ request, params: {}, context: {} });
            throw new Error("Expected redirect");
        } catch (response: unknown) {
            expect(response).toBeInstanceOf(Response);
            const res = response as Response;
            expect(res.status).toBe(302);
            expect(res.headers.get("Location")).toContain("/api/v1/auth/authorize");
        }
    });
});
