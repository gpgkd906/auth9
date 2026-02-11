import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import ServicesPage, { loader, action } from "~/routes/dashboard.services._index";
import { serviceApi } from "~/services/api";
import type { Service } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";

// Mock the APIs
vi.mock("~/services/api", () => ({
    serviceApi: {
        list: vi.fn(),
        create: vi.fn(),
        delete: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue(null),
    requireAuthWithUpdate: vi.fn().mockResolvedValue({
        session: {
            accessToken: "test-token",
            refreshToken: "test-refresh-token",
            idToken: "test-id-token",
            expiresAt: Date.now() + 3600000,
        },
        headers: undefined,
    }),
}));

function WrappedPage() {
    return (
        <ConfirmProvider>
            <ServicesPage />
        </ConfirmProvider>
    );
}

describe("Services Page", () => {
    const mockServices = {
        data: [
            { id: "s1", name: "My App", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
        ],
        pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
    };

    it("renders service registry list", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText("My App")).toBeInTheDocument();
            expect(screen.getByText("active")).toBeInTheDocument();
        });
    });

    it("displays register service dialog", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        const registerButton = await screen.findByRole("button", { name: /Register Service/i });
        await user.click(registerButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getAllByText("Register Service")[0]).toBeInTheDocument();
    });

    it("displays empty state when no services", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText("No services found")).toBeInTheDocument();
        });
    });

    it("displays pagination information", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [{ id: "s1", name: "Service 1", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() }],
            pagination: { total: 25, page: 2, per_page: 20, total_pages: 3 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText(/25 services/)).toBeInTheDocument();
            expect(screen.getByText(/Page 2 of/)).toBeInTheDocument();
        });
    });

    it("displays page header and description", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText("Services")).toBeInTheDocument();
            expect(screen.getByText("Register and manage OIDC clients")).toBeInTheDocument();
        });
    });

    it("displays table headers", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue(mockServices);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText("Name")).toBeInTheDocument();
            expect(screen.getByText("Status")).toBeInTheDocument();
            expect(screen.getByText("Updated")).toBeInTheDocument();
        });
    });

    it("displays service row with correct data", async () => {
        const testDate = new Date("2026-01-15T10:30:00Z");
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [{ id: "s1", name: "Test App", redirect_uris: [], logout_uris: [], status: "inactive" as const, created_at: testDate.toISOString(), updated_at: testDate.toISOString() }],
            pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        await waitFor(() => {
            expect(screen.getByText("Test App")).toBeInTheDocument();
            expect(screen.getByText("inactive")).toBeInTheDocument();
        });
    });

    it("displays error message from action in create dialog", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });
        vi.mocked(serviceApi.create).mockRejectedValue(new Error("Service creation failed"));

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        // Open create dialog
        const registerButton = await screen.findByRole("button", { name: /Register Service/i });
        await user.click(registerButton);

        // Fill and submit the form
        const nameInput = screen.getByLabelText("Service Name");
        await user.type(nameInput, "Test");
        const submitBtn = screen.getByRole("button", { name: /Register$/i });
        await user.click(submitBtn);

        // Error message should appear in dialog
        await waitFor(() => {
            expect(screen.getByText(/Service creation failed/i)).toBeInTheDocument();
        });
    });

    it("shows secret dialog after successful service creation with secret", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
                action: () => {
                    return { success: true, intent: "create", secret: "my-secret-value-123" };
                },
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        // Open create dialog
        const registerButton = await screen.findByRole("button", { name: /Register Service/i });
        await user.click(registerButton);

        // Fill and submit form
        const nameInput = screen.getByLabelText("Service Name");
        await user.type(nameInput, "New App");
        const submitBtn = screen.getByRole("button", { name: /Register$/i });
        await user.click(submitBtn);

        // Secret dialog should appear
        await waitFor(() => {
            expect(screen.getByText("Initial Client Secret Generated")).toBeInTheDocument();
            expect(screen.getByText("my-secret-value-123")).toBeInTheDocument();
        });
    });

    it("closes secret dialog when close button is clicked", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
                action: () => ({ success: true, intent: "create", secret: "secret-xyz" }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        // Open create dialog, submit
        const registerButton = await screen.findByRole("button", { name: /Register Service/i });
        await user.click(registerButton);
        const nameInput = screen.getByLabelText("Service Name");
        await user.type(nameInput, "App");
        const submitBtn = screen.getByRole("button", { name: /Register$/i });
        await user.click(submitBtn);

        // Wait for secret dialog
        await waitFor(() => {
            expect(screen.getByText("secret-xyz")).toBeInTheDocument();
        });

        // Close the secret dialog (use getAllByRole to handle multiple Close buttons/spans)
        const closeButtons = screen.getAllByRole("button", { name: "Close" });
        await user.click(closeButtons[0]);

        await waitFor(() => {
            expect(screen.queryByText("secret-xyz")).not.toBeInTheDocument();
        });
    });

    it("opens delete confirmation dialog and confirms deletion", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [
                { id: "s1", name: "Test Service", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            ],
            pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
                action: () => ({ success: true, intent: "delete" }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        // Wait for the service row
        await waitFor(() => {
            expect(screen.getByText("Test Service")).toBeInTheDocument();
        });

        // Open the dropdown menu
        const menuButton = screen.getByRole("button", { name: /open menu/i });
        await user.click(menuButton);

        // Click delete item
        await waitFor(() => {
            expect(screen.getByText("Delete")).toBeInTheDocument();
        });
        const deleteItem = screen.getByText("Delete");
        await user.click(deleteItem);

        // Confirm dialog should appear
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Service");
        });

        // Confirm the deletion
        const confirmButton = screen.getByTestId("confirm-dialog-action");
        await user.click(confirmButton);
    });

    it("cancels delete via confirmation dialog", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [
                { id: "s1", name: "Keep Service", redirect_uris: [], logout_uris: [], status: "active" as const, created_at: new Date().toISOString(), updated_at: new Date().toISOString() },
            ],
            pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
                action: () => ({ success: true, intent: "delete" }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        // Wait for the service row
        await waitFor(() => {
            expect(screen.getByText("Keep Service")).toBeInTheDocument();
        });

        // Open the dropdown menu
        const menuButton = screen.getByRole("button", { name: /open menu/i });
        await user.click(menuButton);

        // Click delete item
        await waitFor(() => {
            expect(screen.getByText("Delete")).toBeInTheDocument();
        });
        await user.click(screen.getByText("Delete"));

        // Confirm dialog should appear
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Service");
        });

        // Cancel the deletion
        const cancelButton = screen.getByTestId("confirm-dialog-cancel");
        await user.click(cancelButton);

        // Service should still be there
        expect(screen.getByText("Keep Service")).toBeInTheDocument();
    });

    it("closes create dialog via cancel button", async () => {
        vi.mocked(serviceApi.list).mockResolvedValue({
            data: [],
            pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services"]} />);

        // Open create dialog
        const registerButton = await screen.findByRole("button", { name: /Register Service/i });
        await user.click(registerButton);

        await waitFor(() => {
            expect(screen.getByRole("dialog")).toBeInTheDocument();
        });

        // Click cancel
        const cancelButton = screen.getByRole("button", { name: "Cancel" });
        await user.click(cancelButton);

        await waitFor(() => {
            expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        });
    });
});

describe("action", () => {
    function createFormRequest(data: Record<string, string>) {
        const formData = new FormData();
        for (const [key, value] of Object.entries(data)) {
            formData.append(key, value);
        }
        return new Request("http://localhost/dashboard/services", { method: "POST", body: formData });
    }

    it("creates a service and returns success with secret", async () => {
        vi.mocked(serviceApi.create).mockResolvedValue({
            data: {
                id: "s1",
                name: "New App",
                redirect_uris: ["https://app.com/callback"],
                logout_uris: ["https://app.com/logout"],
                status: "active",
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
                client: { client_id: "auto-id", client_secret: "secret-abc-123" },
            },
        });

        const request = createFormRequest({
            intent: "create",
            name: "New App",
            client_id: "my-client",
            base_url: "https://app.com",
            redirect_uris: "https://app.com/callback",
            logout_uris: "https://app.com/logout",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ success: true, intent: "create", secret: "secret-abc-123" });
        expect(serviceApi.create).toHaveBeenCalledWith(
            expect.objectContaining({
                name: "New App",
                client_id: "my-client",
                base_url: "https://app.com",
                redirect_uris: ["https://app.com/callback"],
                logout_uris: ["https://app.com/logout"],
            }),
            undefined,
        );
    });

    it("creates a service without client in response", async () => {
        vi.mocked(serviceApi.create).mockResolvedValue({
            data: {
                id: "s2",
                name: "No Client App",
                redirect_uris: [],
                logout_uris: [],
                status: "active",
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            } as Service,
        });

        const request = createFormRequest({
            intent: "create",
            name: "No Client App",
            client_id: "",
            base_url: "",
            redirect_uris: "",
            logout_uris: "",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ success: true, intent: "create" });
    });

    it("auto-generates client_id when not provided", async () => {
        vi.mocked(serviceApi.create).mockResolvedValue({
            data: {
                id: "s3",
                name: "Auto ID App",
                redirect_uris: [],
                logout_uris: [],
                status: "active",
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            } as Service,
        });

        const request = createFormRequest({
            intent: "create",
            name: "Auto ID App",
            client_id: "",
            base_url: "",
            redirect_uris: "",
            logout_uris: "",
        });

        await action({ request, params: {}, context: {} });
        expect(serviceApi.create).toHaveBeenCalledWith(
            expect.objectContaining({
                client_id: expect.any(String),
            }),
            undefined,
        );
        // The auto-generated client_id should be non-empty (UUID format)
        const callArgs = vi.mocked(serviceApi.create).mock.calls[0][0];
        expect(callArgs.client_id).toBeTruthy();
        expect(callArgs.client_id.length).toBeGreaterThan(0);
    });

    it("deletes a service and returns success", async () => {
        vi.mocked(serviceApi.delete).mockResolvedValue(undefined);

        const request = createFormRequest({
            intent: "delete",
            id: "s1",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toEqual({ success: true, intent: "delete" });
        expect(serviceApi.delete).toHaveBeenCalledWith("s1", undefined);
    });

    it("returns error on create API failure", async () => {
        vi.mocked(serviceApi.create).mockRejectedValue(new Error("Service name already exists"));

        const request = createFormRequest({
            intent: "create",
            name: "Duplicate App",
            client_id: "",
            base_url: "",
            redirect_uris: "",
            logout_uris: "",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Service name already exists");
        expect((result as Response).status).toBe(400);
    });

    it("returns error on delete API failure", async () => {
        vi.mocked(serviceApi.delete).mockRejectedValue(new Error("Service not found"));

        const request = createFormRequest({
            intent: "delete",
            id: "nonexistent",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Service not found");
        expect((result as Response).status).toBe(400);
    });

    it("returns error for invalid intent", async () => {
        const request = createFormRequest({
            intent: "invalid_action",
        });

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Invalid intent");
        expect((result as Response).status).toBe(400);
    });

    it("returns error when no intent is provided", async () => {
        const request = createFormRequest({});

        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Invalid intent");
        expect((result as Response).status).toBe(400);
    });
});
