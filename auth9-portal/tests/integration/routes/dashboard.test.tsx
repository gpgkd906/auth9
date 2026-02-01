import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Dashboard from "~/routes/dashboard";

describe("Dashboard Layout", () => {
    it("renders dashboard sidebar and navigation", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: Dashboard,
                children: [
                    {
                        path: "/dashboard",
                        Component: () => <div>Dashboard Home</div>,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        expect(screen.getByText("Auth9")).toBeInTheDocument();
        expect(screen.getByText("Overview")).toBeInTheDocument();
        expect(screen.getByText("Tenants")).toBeInTheDocument();

        // Check user info
        expect(screen.getByText("John Doe")).toBeInTheDocument();
    });
});
