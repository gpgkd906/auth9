import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import ServiceDetailPage, { loader, action } from "~/routes/dashboard.services.$id";
import { serviceApi } from "~/services/api";

// Mock the APIs
vi.mock("~/services/api", () => ({
    serviceApi: {
        get: vi.fn(),
        listClients: vi.fn(),
        regenerateClientSecret: vi.fn(),
        createClient: vi.fn(),
        deleteClient: vi.fn(),
        update: vi.fn(),
    },
}));

// Mock global confirm
global.confirm = vi.fn(() => true);

describe("Service Detail Page", () => {
    const mockService = {
        id: "s1",
        name: "My App",
        base_url: "https://myapp.com",
        status: "active",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
    };

    const mockClients = {
        data: [
            { id: "c1", client_id: "client-id-1", name: "Web App", created_at: new Date().toISOString() },
        ],
    };

    it("renders service details and clients", async () => {
        (serviceApi.get as any).mockResolvedValue({ data: mockService });
        (serviceApi.listClients as any).mockResolvedValue(mockClients);

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/services/:id",
                Component: ServiceDetailPage,
                loader,
            },
        ]);

        render(<RemixStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("My App")).toBeInTheDocument();
            expect(screen.getByDisplayValue("https://myapp.com")).toBeInTheDocument();
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });
    });

    it("regenerates client secret", async () => {
        (serviceApi.get as any).mockResolvedValue({ data: mockService });
        (serviceApi.listClients as any).mockResolvedValue(mockClients);
        (serviceApi.regenerateClientSecret as any).mockResolvedValue({
            data: { client_id: "client-id-1", client_secret: "new-secret-123" },
        });

        const RemixStub = createRemixStub([
            {
                path: "/dashboard/services/:id",
                Component: ServiceDetailPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RemixStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        const regenerateButton = screen.getByRole("button", { name: /Regenerate/i });
        await user.click(regenerateButton);

        expect(global.confirm).toHaveBeenCalled();

        await waitFor(() => {
            expect(serviceApi.regenerateClientSecret).toHaveBeenCalledWith("s1", "client-id-1");
            expect(screen.getByText("Secret Regenerated")).toBeInTheDocument();
            expect(screen.getByText("new-secret-123")).toBeInTheDocument();
        });
    });
});
