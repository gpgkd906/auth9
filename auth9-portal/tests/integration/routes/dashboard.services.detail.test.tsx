import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ServiceDetailPage, { loader, action, meta } from "~/routes/dashboard.services.$id";
import { serviceApi } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";

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

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue(null),
}));

// Ensure navigator.clipboard.writeText is available (happy-dom provides it natively)

describe("meta", () => {
    it("returns the correct page title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Service Details - Auth9" }]);
    });
});

describe("Service Detail Page", () => {
    const mockService = {
        id: "s1",
        name: "My App",
        base_url: "https://myapp.com",
        redirect_uris: [],
        logout_uris: [],
        status: "active" as const,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
    };

    const mockClients = {
        data: [
            { id: "c1", service_id: "s1", client_id: "client-id-1", name: "Web App", created_at: new Date().toISOString() },
        ],
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

    function WrappedPage() {
        return (
            <ConfirmProvider>
                <ServiceDetailPage />
            </ConfirmProvider>
        );
    }

    it("renders service details and clients", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("My App")).toBeInTheDocument();
            expect(screen.getByDisplayValue("https://myapp.com")).toBeInTheDocument();
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });
    });

    it("regenerates client secret", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);
        vi.mocked(serviceApi.regenerateClientSecret).mockResolvedValue({
            data: { client_id: "client-id-1", client_secret: "new-secret-123" },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        const regenerateButton = screen.getByRole("button", { name: /Regenerate/i });
        await user.click(regenerateButton);

        // AlertDialog should appear
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Regenerate Secret");
        });

        // Confirm the action
        await user.click(screen.getByTestId("confirm-dialog-action"));

        await waitFor(() => {
            expect(serviceApi.regenerateClientSecret).toHaveBeenCalledWith("s1", "client-id-1", undefined);
            expect(screen.getByText("Secret Regenerated")).toBeInTheDocument();
            expect(screen.getByText("new-secret-123")).toBeInTheDocument();
        });
    });

    it("deletes a client through the UI with confirmation", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);
        vi.mocked(serviceApi.deleteClient).mockResolvedValue(undefined);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        const deleteButton = screen.getByRole("button", { name: /Delete/i });
        await user.click(deleteButton);

        // Confirm dialog should appear with destructive variant
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Client");
        });

        // Confirm the deletion
        await user.click(screen.getByTestId("confirm-dialog-action"));

        await waitFor(() => {
            expect(serviceApi.deleteClient).toHaveBeenCalledWith("s1", "client-id-1", undefined);
        });
    });

    it("cancels delete client confirmation", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        const deleteButton = screen.getByRole("button", { name: /Delete/i });
        await user.click(deleteButton);

        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Client");
        });

        // Cancel the deletion
        await user.click(screen.getByTestId("confirm-dialog-cancel"));

        // deleteClient should NOT have been called
        expect(serviceApi.deleteClient).not.toHaveBeenCalled();
    });

    it("creates a client through the UI and shows secret dialog", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);
        vi.mocked(serviceApi.createClient).mockResolvedValue({
            data: {
                id: "c2",
                service_id: "s1",
                client_id: "new-client-id",
                client_secret: "super-secret-value",
                name: "New Client",
                created_at: new Date().toISOString(),
            },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        // Open the create client dialog by clicking the + button (DialogTrigger)
        // The DialogTrigger button has a data-state attribute set by Radix
        const triggerButton = screen.getAllByRole("button").find(btn => btn.hasAttribute("data-state"));
        expect(triggerButton).toBeDefined();
        await user.click(triggerButton!);

        await waitFor(() => {
            expect(screen.getByText("Create New Client")).toBeInTheDocument();
        });

        // Fill in the client name
        const nameInput = screen.getByLabelText("Description (Optional)");
        await user.type(nameInput, "New Client");

        // Submit the form
        const createButton = screen.getByRole("button", { name: "Create" });
        await user.click(createButton);

        // After creation, the secret dialog should appear (via useEffect)
        await waitFor(() => {
            expect(serviceApi.createClient).toHaveBeenCalledWith("s1", { name: "New Client" }, undefined);
            expect(screen.getByText("Client Created Successfully")).toBeInTheDocument();
            expect(screen.getByText("super-secret-value")).toBeInTheDocument();
            expect(screen.getByText("new-client-id")).toBeInTheDocument();
        });
    });

    it("copies client ID to clipboard when copy button is clicked", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        // Find the copy button by title attribute
        const copyButton = screen.getByTitle("Copy Client ID");
        await user.click(copyButton);

        // After clicking, the copy handler runs and shows a checkmark
        await waitFor(() => {
            expect(screen.getByText("\u2713")).toBeInTheDocument();
        });
    });

    it("shows empty state when no clients exist", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue({ data: [] });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("No clients found.")).toBeInTheDocument();
        });
    });

    it("closes the secret dialog via the Close button", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);
        vi.mocked(serviceApi.regenerateClientSecret).mockResolvedValue({
            data: { client_id: "client-id-1", client_secret: "regen-secret-abc" },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        // Regenerate to get the secret dialog open
        await user.click(screen.getByRole("button", { name: /Regenerate/i }));
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Regenerate Secret");
        });
        await user.click(screen.getByTestId("confirm-dialog-action"));

        await waitFor(() => {
            expect(screen.getByText("regen-secret-abc")).toBeInTheDocument();
        });

        // Close the secret dialog using the explicit Close button (not the X icon button)
        // There are two "Close" buttons - the explicit one and the sr-only X button
        const closeButtons = screen.getAllByRole("button", { name: "Close" });
        // The explicit Close button is the one without the sr-only span
        const explicitCloseButton = closeButtons.find(btn => btn.textContent === "Close" && !btn.querySelector("span.sr-only"));
        expect(explicitCloseButton).toBeDefined();
        await user.click(explicitCloseButton!);

        await waitFor(() => {
            expect(screen.queryByText("regen-secret-abc")).not.toBeInTheDocument();
        });
    });

    it("copies client ID and secret from the secret dialog", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);
        vi.mocked(serviceApi.regenerateClientSecret).mockResolvedValue({
            data: { client_id: "client-id-1", client_secret: "copy-test-secret" },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByText("client-id-1")).toBeInTheDocument();
        });

        // Open secret dialog via regenerate
        await user.click(screen.getByRole("button", { name: /Regenerate/i }));
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toBeInTheDocument();
        });
        await user.click(screen.getByTestId("confirm-dialog-action"));

        await waitFor(() => {
            expect(screen.getByText("copy-test-secret")).toBeInTheDocument();
        });

        // The secret dialog has two copy icon buttons (h-8 w-8 shrink-0) for client ID and secret
        // Find them by their class pattern within the dialog
        const dialogContent = screen.getByText("Copy the Client Secret now. It will not be shown again.").closest("[role='dialog']");
        expect(dialogContent).not.toBeNull();

        // Get all buttons within the dialog, filter out the "Close" button and the X close button
        const dialogButtons = Array.from(dialogContent!.querySelectorAll("button"));
        const copyButtons = dialogButtons.filter(btn => {
            const text = btn.textContent?.trim() || "";
            // Copy buttons have no meaningful text (just SVG or checkmark)
            return text !== "Close" && !btn.querySelector("span.sr-only") && btn.className.includes("shrink-0");
        });

        expect(copyButtons.length).toBe(2);

        // Click the first copy button (Client ID) - handleCopy shows checkmark on success
        await user.click(copyButtons[0]);
        await waitFor(() => {
            // The checkmark character appears after successful copy
            const checkmarks = screen.getAllByText("\u2713");
            expect(checkmarks.length).toBeGreaterThanOrEqual(1);
        });

        // Click the second copy button (Client Secret) - same checkmark feedback
        await user.click(copyButtons[1]);
        await waitFor(() => {
            const checkmarks = screen.getAllByText("\u2713");
            expect(checkmarks.length).toBeGreaterThanOrEqual(1);
        });
    });

    it("renders service with redirect and logout URIs", async () => {
        const serviceWithUris = {
            ...mockService,
            redirect_uris: ["https://app.com/callback", "https://app.com/auth"],
            logout_uris: ["https://app.com/logout"],
        };
        vi.mocked(serviceApi.get).mockResolvedValue({ data: serviceWithUris });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByDisplayValue("https://app.com/callback, https://app.com/auth")).toBeInTheDocument();
            expect(screen.getByDisplayValue("https://app.com/logout")).toBeInTheDocument();
        });
    });

    it("submits the update service form", async () => {
        vi.mocked(serviceApi.get).mockResolvedValue({ data: mockService });
        vi.mocked(serviceApi.listClients).mockResolvedValue(mockClients);
        vi.mocked(serviceApi.update).mockResolvedValue({
            data: { ...mockService, name: "Updated App" },
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/services/:id",
                Component: WrappedPage,
                loader,
                action,
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/services/s1"]} />);

        await waitFor(() => {
            expect(screen.getByDisplayValue("My App")).toBeInTheDocument();
        });

        // Update the service name
        const nameInput = screen.getByLabelText("Service Name");
        await user.clear(nameInput);
        await user.type(nameInput, "Updated App");

        // Submit the form
        const saveButton = screen.getByRole("button", { name: "Save Changes" });
        await user.click(saveButton);

        await waitFor(() => {
            expect(serviceApi.update).toHaveBeenCalled();
        });
    });
});

describe("action", () => {
    function createFormRequest(data: Record<string, string>) {
        const formData = new FormData();
        for (const [key, value] of Object.entries(data)) {
            formData.append(key, value);
        }
        return new Request("http://localhost/dashboard/services/s1", { method: "POST", body: formData });
    }

    it("returns error when service ID is missing", async () => {
        const request = createFormRequest({ intent: "update_service" });
        const result = await action({ request, params: {}, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Service ID required");
        expect((result as Response).status).toBe(400);
    });

    it("updates a service successfully", async () => {
        vi.mocked(serviceApi.update).mockResolvedValue({
            data: {
                id: "s1",
                name: "Updated App",
                base_url: "https://updated.com",
                redirect_uris: ["https://updated.com/callback"],
                logout_uris: ["https://updated.com/logout"],
                status: "active",
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        });

        const request = createFormRequest({
            intent: "update_service",
            name: "Updated App",
            base_url: "https://updated.com",
            redirect_uris: "https://updated.com/callback",
            logout_uris: "https://updated.com/logout",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toEqual({ success: true, intent: "update_service" });
        expect(serviceApi.update).toHaveBeenCalledWith(
            "s1",
            expect.objectContaining({
                name: "Updated App",
                base_url: "https://updated.com",
                redirect_uris: ["https://updated.com/callback"],
                logout_uris: ["https://updated.com/logout"],
            }),
            undefined,
        );
    });

    it("creates a client and returns secret", async () => {
        vi.mocked(serviceApi.createClient).mockResolvedValue({
            data: {
                id: "c2",
                service_id: "s1",
                client_id: "new-client-id",
                client_secret: "new-client-secret-xyz",
                name: "Production Client",
                created_at: new Date().toISOString(),
            },
        });

        const request = createFormRequest({
            intent: "create_client",
            name: "Production Client",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toEqual({
            success: true,
            intent: "create_client",
            secret: "new-client-secret-xyz",
            clientId: "new-client-id",
        });
        expect(serviceApi.createClient).toHaveBeenCalledWith("s1", { name: "Production Client" }, undefined);
    });

    it("creates a client without a name", async () => {
        vi.mocked(serviceApi.createClient).mockResolvedValue({
            data: {
                id: "c3",
                service_id: "s1",
                client_id: "unnamed-client-id",
                client_secret: "unnamed-secret",
                name: "",
                created_at: new Date().toISOString(),
            },
        });

        const request = createFormRequest({
            intent: "create_client",
            name: "",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toEqual({
            success: true,
            intent: "create_client",
            secret: "unnamed-secret",
            clientId: "unnamed-client-id",
        });
        expect(serviceApi.createClient).toHaveBeenCalledWith("s1", { name: undefined }, undefined);
    });

    it("deletes a client successfully", async () => {
        vi.mocked(serviceApi.deleteClient).mockResolvedValue(undefined);

        const request = createFormRequest({
            intent: "delete_client",
            client_id: "client-id-1",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toEqual({ success: true, intent: "delete_client" });
        expect(serviceApi.deleteClient).toHaveBeenCalledWith("s1", "client-id-1", undefined);
    });

    it("regenerates a client secret successfully", async () => {
        vi.mocked(serviceApi.regenerateClientSecret).mockResolvedValue({
            data: { client_id: "client-id-1", client_secret: "regenerated-secret-456" },
        });

        const request = createFormRequest({
            intent: "regenerate_secret",
            client_id: "client-id-1",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toEqual({
            success: true,
            intent: "regenerate_secret",
            secret: "regenerated-secret-456",
            regeneratedClientId: "client-id-1",
        });
        expect(serviceApi.regenerateClientSecret).toHaveBeenCalledWith("s1", "client-id-1", undefined);
    });

    it("returns error on update_service API failure", async () => {
        vi.mocked(serviceApi.update).mockRejectedValue(new Error("Validation failed"));

        const request = createFormRequest({
            intent: "update_service",
            name: "Bad Name",
            base_url: "",
            redirect_uris: "",
            logout_uris: "",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Validation failed");
        expect((result as Response).status).toBe(400);
    });

    it("returns error on create_client API failure", async () => {
        vi.mocked(serviceApi.createClient).mockRejectedValue(new Error("Client limit reached"));

        const request = createFormRequest({
            intent: "create_client",
            name: "Overflow Client",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Client limit reached");
        expect((result as Response).status).toBe(400);
    });

    it("returns error on delete_client API failure", async () => {
        vi.mocked(serviceApi.deleteClient).mockRejectedValue(new Error("Client not found"));

        const request = createFormRequest({
            intent: "delete_client",
            client_id: "nonexistent-client",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Client not found");
        expect((result as Response).status).toBe(400);
    });

    it("returns error on regenerate_secret API failure", async () => {
        vi.mocked(serviceApi.regenerateClientSecret).mockRejectedValue(new Error("Secret regeneration failed"));

        const request = createFormRequest({
            intent: "regenerate_secret",
            client_id: "client-id-1",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Secret regeneration failed");
        expect((result as Response).status).toBe(400);
    });

    it("returns error for invalid intent", async () => {
        const request = createFormRequest({
            intent: "unknown_action",
        });

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Invalid intent");
        expect((result as Response).status).toBe(400);
    });

    it("returns error when no intent is provided", async () => {
        const request = createFormRequest({});

        const result = await action({ request, params: { id: "s1" }, context: {} });
        expect(result).toBeInstanceOf(Response);
        const body = await (result as Response).json();
        expect(body.error).toBe("Invalid intent");
        expect((result as Response).status).toBe(400);
    });
});
