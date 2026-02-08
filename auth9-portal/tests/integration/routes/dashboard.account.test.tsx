import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import AccountLayout, { meta } from "~/routes/dashboard.account";

describe("Account Layout", () => {
    it("meta returns correct title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Account - Auth9" }]);
    });

    it("renders navigation links and heading", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountLayout,
                children: [
                    {
                        index: true,
                        Component: () => <div>Account Content</div>,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);

        expect(await screen.findByText("Account")).toBeInTheDocument();
        expect(screen.getByText("Manage your personal account settings")).toBeInTheDocument();
        expect(screen.getByText("Profile")).toBeInTheDocument();
        expect(screen.getByText("Security")).toBeInTheDocument();
        expect(screen.getByText("Passkeys")).toBeInTheDocument();
        expect(screen.getByText("Sessions")).toBeInTheDocument();
        expect(screen.getByText("Linked Identities")).toBeInTheDocument();
    });

    it("highlights active navigation link", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountLayout,
                children: [
                    {
                        path: "security",
                        Component: () => <div>Security Content</div>,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/security"]} />);

        const securityLink = await screen.findByText("Security");
        expect(securityLink.closest("a")).toHaveAttribute("href", "/dashboard/account/security");
    });

    it("renders child content via Outlet", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account",
                Component: AccountLayout,
                children: [
                    {
                        index: true,
                        Component: () => <div>Child Outlet Content</div>,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account"]} />);

        expect(await screen.findByText("Child Outlet Content")).toBeInTheDocument();
    });
});
