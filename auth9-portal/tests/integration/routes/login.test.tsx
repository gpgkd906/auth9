import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Login, { action } from "~/routes/login";

describe("Login Page", () => {
    it("renders login form", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
                action,
            },
        ]);

        render(<RoutesStub initialEntries={["/login"]} />);

        expect(screen.getByText("Welcome back")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /sign in/i })).toBeInTheDocument();
    });

    it("displays Auth9 branding", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
            },
        ]);

        render(<RoutesStub initialEntries={["/login"]} />);

        expect(screen.getByText("Sign in to your Auth9 account")).toBeInTheDocument();
    });

    it("has link to registration page", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
            },
        ]);

        render(<RoutesStub initialEntries={["/login"]} />);

        expect(screen.getByText("Sign up")).toBeInTheDocument();
        expect(screen.getByRole("link", { name: /sign up/i })).toHaveAttribute("href", "/register");
    });

    it("renders sign in button with correct text", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
            },
        ]);

        render(<RoutesStub initialEntries={["/login"]} />);

        const submitButton = screen.getByRole("button", { name: /sign in with sso/i });
        expect(submitButton).toBeInTheDocument();
        expect(submitButton).not.toBeDisabled();
    });

    it("displays Auth9 logo with letter A", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
            },
        ]);

        render(<RoutesStub initialEntries={["/login"]} />);

        // Logo container has "A9"
        expect(screen.getByText("A9")).toBeInTheDocument();
    });

    it("displays 'Don't have an account?' text", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/login",
                Component: Login,
            },
        ]);

        render(<RoutesStub initialEntries={["/login"]} />);

        expect(screen.getByText(/Don't have an account\?/i)).toBeInTheDocument();
    });

    it("action redirects to authorize endpoint", async () => {
        // Create a mock request
        const request = new Request("http://localhost:3000/login", {
            method: "POST",
        });

        const response = await action({ request, params: {}, context: {} });

        // Verify it returns a redirect
        expect(response.status).toBe(302);

        // Get the Location header
        const location = response.headers.get("Location");
        expect(location).toContain("/api/v1/auth/authorize");
        expect(location).toContain("response_type=code");
        expect(location).toContain("scope=openid+email+profile");
    });
});

