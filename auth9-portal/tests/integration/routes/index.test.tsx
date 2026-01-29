import { createRemixStub } from "@remix-run/testing";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Index from "~/routes/_index";

describe("Landing Page", () => {
    it("renders hero section", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/",
                Component: Index,
            },
        ]);

        render(<RemixStub initialEntries={["/"]} />);

        expect(screen.getByText("Auth9")).toBeInTheDocument();
        expect(screen.getByText(/Identity Management/)).toBeInTheDocument();
        expect(screen.getByText(/Made Simple/)).toBeInTheDocument();
    });

    it("renders navigation links", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/",
                Component: Index,
            },
        ]);

        render(<RemixStub initialEntries={["/"]} />);

        expect(screen.getByText("Sign In")).toBeInTheDocument();
        expect(screen.getByText("Get Started")).toBeInTheDocument();
    });

    it("renders feature cards", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/",
                Component: Index,
            },
        ]);

        render(<RemixStub initialEntries={["/"]} />);

        expect(screen.getByText("Single Sign-On")).toBeInTheDocument();
        expect(screen.getByText("Multi-Tenant")).toBeInTheDocument();
        expect(screen.getByText("Dynamic RBAC")).toBeInTheDocument();
    });

    it("renders call-to-action buttons", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/",
                Component: Index,
            },
        ]);

        render(<RemixStub initialEntries={["/"]} />);

        expect(screen.getByText("Start Free Trial")).toBeInTheDocument();
        expect(screen.getByText("Read Documentation")).toBeInTheDocument();
    });

    it("renders footer with links", async () => {
        const RemixStub = createRemixStub([
            {
                path: "/",
                Component: Index,
            },
        ]);

        render(<RemixStub initialEntries={["/"]} />);

        expect(screen.getByText(/© 2024 Auth9/)).toBeInTheDocument();
        expect(screen.getByText("Privacy")).toBeInTheDocument();
        expect(screen.getByText("Terms")).toBeInTheDocument();
    });
});
