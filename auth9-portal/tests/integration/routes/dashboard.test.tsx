import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import Dashboard from "~/routes/dashboard";

describe("Dashboard Layout", () => {
    it("renders dashboard sidebar and navigation", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard",
                Component: Dashboard,
                loader: () => ({
                    currentUser: {
                        id: "user-1",
                        display_name: "John Doe",
                        email: "john@example.com",
                        avatar_url: "",
                    },
                }),
                children: [
                    {
                        path: "/dashboard",
                        Component: () => <div>Dashboard Home</div>,
                    },
                ],
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard"]} />);

        await waitFor(() => {
            expect(screen.getAllByText("Auth9").length).toBeGreaterThanOrEqual(1);
        });
        expect(screen.getByText("Overview")).toBeInTheDocument();
        expect(screen.getByText("Tenants")).toBeInTheDocument();

        // Check user info
        expect(screen.getByText("John Doe")).toBeInTheDocument();
    });
});
