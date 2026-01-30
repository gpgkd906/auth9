import { createRemixStub } from "@remix-run/testing";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import Register, { action } from "~/routes/register";
import { userApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
    userApi: {
        create: vi.fn(),
    },
}));

describe("Register Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it("renders registration form", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByText("Create your account")).toBeInTheDocument();
        expect(screen.getByText("Start managing identity with Auth9")).toBeInTheDocument();
    });

    it("renders email input field", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByLabelText(/Email/i)).toBeInTheDocument();
        expect(screen.getByPlaceholderText("you@example.com")).toBeInTheDocument();
    });

    it("renders display name input field", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByLabelText(/Display Name/i)).toBeInTheDocument();
        expect(screen.getByPlaceholderText("Jane Doe")).toBeInTheDocument();
    });

    it("renders password input field", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByLabelText(/Password/i)).toBeInTheDocument();
    });

    it("renders submit button", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByRole("button", { name: /Create account/i })).toBeInTheDocument();
    });

    it("renders sign in link", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByText("Already have an account?")).toBeInTheDocument();
        expect(screen.getByText("Sign in")).toBeInTheDocument();
    });

    it("renders Auth9 branding", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        expect(screen.getByText("A")).toBeInTheDocument();
    });

    it("action returns error when email is missing", async () => {
        const body = new URLSearchParams();
        body.append("password", "test123");

        const request = new Request("http://localhost:3000/register", {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });

        const response = await action({ request, params: {}, context: {} });
        const data = await response.json();

        expect(response.status).toBe(400);
        expect(data.error).toBe("Email and password are required");
    });

    it("action returns error when password is missing", async () => {
        const body = new URLSearchParams();
        body.append("email", "test@example.com");

        const request = new Request("http://localhost:3000/register", {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });

        const response = await action({ request, params: {}, context: {} });
        const data = await response.json();

        expect(response.status).toBe(400);
        expect(data.error).toBe("Email and password are required");
    });

    it("action creates user and redirects on success", async () => {
        vi.mocked(userApi.create).mockResolvedValue({
            data: {
                id: "user-1",
                email: "test@example.com",
                display_name: "Test User",
                mfa_enabled: false,
                created_at: "2024-01-01T00:00:00Z",
                updated_at: "2024-01-01T00:00:00Z"
            }
        });

        const body = new URLSearchParams();
        body.append("email", "test@example.com");
        body.append("password", "securePassword123");
        body.append("display_name", "Test User");

        const request = new Request("http://localhost:3000/register", {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });

        const response = await action({ request, params: {}, context: {} });

        expect(userApi.create).toHaveBeenCalledWith({
            email: "test@example.com",
            display_name: "Test User",
            password: "securePassword123",
        });
        expect(response.status).toBe(302);
        expect(response.headers.get("Location")).toBe("/login");
    });

    it("action returns error when API call fails", async () => {
        vi.mocked(userApi.create).mockRejectedValue(new Error("User already exists"));

        const body = new URLSearchParams();
        body.append("email", "existing@example.com");
        body.append("password", "password123");

        const request = new Request("http://localhost:3000/register", {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: body.toString(),
        });

        const response = await action({ request, params: {}, context: {} });
        const data = await response.json();

        expect(response.status).toBe(400);
        expect(data.error).toBe("User already exists");
    });

    it("sign in link navigates to login page", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/register",
                Component: Register,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/register"]} />);

        const signInLink = screen.getByRole("link", { name: /sign in/i });
        expect(signInLink).toHaveAttribute("href", "/login");
    });
});

