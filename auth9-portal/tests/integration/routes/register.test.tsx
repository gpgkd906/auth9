import { createRemixStub } from "@remix-run/testing";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Register, { action } from "~/routes/register";

describe("Register Page", () => {
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
});
