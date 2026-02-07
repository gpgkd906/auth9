import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import TenantsLayout from "~/routes/dashboard.tenants";

describe("Tenants Layout", () => {
  it("renders Outlet for child routes", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants",
        Component: TenantsLayout,
        children: [
          {
            index: true,
            Component: () => <div>Child Route Content</div>,
          },
        ],
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

    await waitFor(() => {
      expect(screen.getByText("Child Route Content")).toBeInTheDocument();
    });
  });
});
