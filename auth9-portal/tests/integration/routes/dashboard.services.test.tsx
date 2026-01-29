import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import ServicesPage, { loader } from "~/routes/dashboard.services._index";
import { serviceApi } from "~/services/api";

// Mock the APIs
vi.mock("~/services/api", () => ({
    serviceApi: {
        list: vi.fn(),
        create: vi.fn(),
        delete: vi.fn(),
    },
}));

describe("Services Page", () => {
    const mockServices = {
        data: [
            { id: "s1", name: "My App", status: "active", updated_at: new Date().toISOString() },
        ],
        pagination: { total: 1, page: 1, total_pages: 1 },
    };

    it("renders service registry list", async () => {
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/services",
                Component: ServicesPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText("My App")).toBeInTheDocument();
            expect(screen.getByText("active")).toBeInTheDocument();
        });
    });

    it("displays register service dialog", async () => {
        (serviceApi.list as any).mockResolvedValue(mockServices);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/services",
                Component: ServicesPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/services"]} />);

        const registerButton = await screen.findByRole("button", { name: /Register Service/i });
        await user.click(registerButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getAllByText("Register Service")[0]).toBeInTheDocument();
    });
});
