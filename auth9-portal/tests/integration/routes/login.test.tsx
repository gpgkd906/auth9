import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Login, { loader, action, meta } from "~/routes/login";

describe("Login Page", () => {
    it("meta returns correct title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Sign In - Auth9" }]);
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader redirects to authorize endpoint when no error", async () => {
        const request = new Request("http://localhost:3000/login");
        const response = await loader({ request, params: {}, context: {} });

        expect(response.status).toBe(302);
        const location = response.headers.get("Location");
        expect(location).toContain("/api/v1/auth/authorize");
        expect(location).toContain("response_type=code");
        expect(location).toContain("scope=openid+email+profile");
    });

    it("loader returns error data when error param present", async () => {
        const request = new Request("http://localhost:3000/login?error=access_denied");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "access_denied" });
    });

    it("loader returns error data for generic errors", async () => {
        const request = new Request("http://localhost:3000/login?error=server_error");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "server_error" });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action redirects to authorize endpoint", async () => {
        const request = new Request("http://localhost:3000/login", {
            method: "POST",
        });

        const response = await action({ request, params: {}, context: {} });

        expect(response.status).toBe(302);
        const location = response.headers.get("Location");
        expect(location).toContain("/api/v1/auth/authorize");
        expect(location).toContain("response_type=code");
        expect(location).toContain("scope=openid+email+profile");
    });

    // ============================================================================
    // Component Tests (error state only)
    // ============================================================================

    it("renders error page with Try Again button on access_denied", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
                loader() {
                    return { error: "access_denied" };
                },
            },
        ]);

        render(<RoutesStub initialEntries={["/login?error=access_denied"]} />);

        expect(await screen.findByText("Sign In Failed")).toBeInTheDocument();
        expect(screen.getByText(/Access was denied/)).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /try again/i })).toBeInTheDocument();
    });

    it("renders error page with error message for generic errors", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
                loader() {
                    return { error: "server_error" };
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
                    return { error: "test_error" };
                },
            },
        ]);

        render(<RoutesStub initialEntries={["/login?error=test_error"]} />);

        expect(await screen.findByText("A9")).toBeInTheDocument();
    });
});
