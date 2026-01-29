import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import Login, { action } from "~/routes/login";

describe("Login Page", () => {
    it("renders login form", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/login",
                Component: Login,
                action,
            },
        ]);

        render(<RemixStub initialEntries={["/login"]} />);

        expect(screen.getByText("Welcome back")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /sign in/i })).toBeInTheDocument();
    });

    // Note: Full form submission testing with action requires more complex mocking of the action
    // or relying on the stub's internal handling. For now, we verify rendering and basic interaction.
});
