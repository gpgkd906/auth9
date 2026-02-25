import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import TenantsIndexPage, { action, loader, meta } from "~/routes/dashboard.tenants._index";
import { tenantApi } from "~/services/api";
import { ConfirmProvider } from "~/hooks/useConfirm";

// Mock the tenant API
vi.mock("~/services/api", () => ({
    tenantApi: {
        list: vi.fn(),
        create: vi.fn(),
        update: vi.fn(),
        delete: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessTokenWithUpdate: vi.fn().mockResolvedValue({
        token: "test-token",
        headers: undefined,
    }),
}));

function WrappedPage() {
    return (
        <ConfirmProvider>
            <TenantsIndexPage />
        </ConfirmProvider>
    );
}

describe("Tenants Page", () => {
    const mockTenants = {
        data: [
            {
                id: "1",
                name: "Acme Corp",
                slug: "acme",
                settings: {},
                status: "active" as const,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
            {
                id: "2",
                name: "Globex",
                slug: "globex",
                settings: {},
                status: "inactive" as const,
                created_at: new Date().toISOString(),
                updated_at: new Date().toISOString(),
            },
        ],
        pagination: {
            total: 2,
            page: 1,
            per_page: 20,
            total_pages: 1,
        },
    };

    beforeEach(() => {
        vi.clearAllMocks();
    });

    it("renders tenant list from loader", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Both mobile and desktop views render, so use getAllByText
        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
            expect(screen.getAllByText("Globex").length).toBeGreaterThan(0);
        });
    });

    it("displays create tenant dialog when button clicked", async () => {
        vi.mocked(tenantApi.list).mockResolvedValue(mockTenants);

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for the button to be rendered
        const createButton = await screen.findByText("Create Tenant");
        await user.click(createButton);

        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add a new tenant to the system. Slug must be unique.")).toBeInTheDocument();
    });

    it("renders empty state when no tenants found", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    data: [],
                    pagination: { total: 0, page: 1, per_page: 20, total_pages: 1 },
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Both mobile and desktop views show empty state
        await waitFor(() => {
            expect(screen.getAllByText("No tenants found").length).toBeGreaterThan(0);
        });
    });

    // ============================================================================
    // Meta Tests
    // ============================================================================

    it("meta returns correct title", () => {
        const result = meta({} as Parameters<typeof meta>[0]);
        expect(result).toEqual([{ title: "Tenants - Auth9" }]);
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    describe("loader", () => {
        it("calls tenantApi.list with default pagination", async () => {
            vi.mocked(tenantApi.list).mockResolvedValue({
                ...mockTenants,
            });

            const request = new Request("http://localhost/dashboard/tenants");
            const result = await loader({ request, params: {}, context: {} });

            expect(tenantApi.list).toHaveBeenCalledWith(1, 20, undefined, "test-token");
            expect(result).toEqual({ ...mockTenants, search: "" });
        });

        it("calls tenantApi.list with custom page and search", async () => {
            vi.mocked(tenantApi.list).mockResolvedValue({
                ...mockTenants,
            });

            const request = new Request("http://localhost/dashboard/tenants?page=2&perPage=10&search=acme");
            const result = await loader({ request, params: {}, context: {} });

            expect(tenantApi.list).toHaveBeenCalledWith(2, 10, "acme", "test-token");
            expect(result).toEqual({ ...mockTenants, search: "acme" });
        });
    });

    // ============================================================================
    // Component Interaction Tests
    // ============================================================================

    it("displays page header and description", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByText("Tenants")).toBeInTheDocument();
            expect(screen.getByText("Manage tenant lifecycle and settings")).toBeInTheDocument();
        });
    });

    it("displays table headers", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Desktop table has headers; mobile cards also have "Status" labels
        await waitFor(() => {
            expect(screen.getAllByText("Name").length).toBeGreaterThan(0);
            expect(screen.getAllByText("Slug").length).toBeGreaterThan(0);
            expect(screen.getAllByText("Status").length).toBeGreaterThan(0);
            expect(screen.getAllByText("Updated").length).toBeGreaterThan(0);
        });
    });

    it("displays pagination information", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    pagination: { total: 50, page: 3, per_page: 20, total_pages: 3 },
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByText(/50 tenants/)).toBeInTheDocument();
            expect(screen.getAllByText(/Page 3 of/).length).toBeGreaterThan(0);
        });
    });

    it("displays tenant status", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Both mobile and desktop views render status text
        await waitFor(() => {
            expect(screen.getAllByText("active").length).toBeGreaterThan(0);
            expect(screen.getAllByText("inactive").length).toBeGreaterThan(0);
        });
    });

    it("renders tenant logo when logo_url is present", async () => {
        const tenantsWithLogo = {
            data: [
                {
                    id: "1",
                    name: "Logo Corp",
                    slug: "logo-corp",
                    logo_url: "https://example.com/logo.png",
                    settings: {},
                    status: "active" as const,
                    created_at: new Date().toISOString(),
                    updated_at: new Date().toISOString(),
                },
            ],
            pagination: { total: 1, page: 1, per_page: 20, total_pages: 1 },
        };

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...tenantsWithLogo,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getAllByText("Logo Corp").length).toBeGreaterThan(0);
        });

        // Find the img element - desktop view renders logo inside an <a> link
        const allLinks = screen.getAllByText("Logo Corp")
            .map((el) => el.closest("a"))
            .filter(Boolean);
        // Find the link that contains an img (desktop table view)
        const linkWithImg = allLinks.find((a) => a?.querySelector("img"));
        expect(linkWithImg).toBeTruthy();
        const img = linkWithImg?.querySelector("img");
        expect(img?.getAttribute("src")).toBe("https://example.com/logo.png");
    });

    it("closes create dialog when cancel button is clicked", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Open the create dialog
        const createButton = await screen.findByText("Create Tenant");
        await user.click(createButton);

        // Verify dialog is open
        expect(screen.getByRole("dialog")).toBeInTheDocument();

        // Click cancel
        const cancelButton = screen.getByRole("button", { name: /Cancel/i });
        await user.click(cancelButton);

        // Verify dialog is closed
        await waitFor(() => {
            expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        });
    });

    it("opens edit dialog when clicking Edit in dropdown menu", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for tenant list to load
        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Click the dropdown menu button (first one for Acme Corp)
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);

        // Click Edit option in the dropdown menu (use menuitem role to target dropdown)
        const editOptions = await screen.findAllByText("Edit");
        const dropdownEdit = editOptions.find((el) => el.closest("[role='menuitem']"));
        await user.click(dropdownEdit!);

        // Verify edit dialog opens with tenant data
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Edit Tenant")).toBeInTheDocument();
        expect(screen.getByText("Update tenant details.")).toBeInTheDocument();
        expect(screen.getByDisplayValue("Acme Corp")).toBeInTheDocument();
        expect(screen.getByDisplayValue("acme")).toBeInTheDocument();
    });

    it("closes edit dialog when cancel button is clicked", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for tenant list
        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Open dropdown and click Edit
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        const editOptions = await screen.findAllByText("Edit");
        const dropdownEdit = editOptions.find((el) => el.closest("[role='menuitem']"));
        await user.click(dropdownEdit!);

        // Verify dialog is open
        expect(screen.getByRole("dialog")).toBeInTheDocument();

        // Click cancel
        const cancelButton = screen.getByRole("button", { name: /Cancel/i });
        await user.click(cancelButton);

        // Verify dialog is closed
        await waitFor(() => {
            expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        });
    });

    it("opens delete confirmation dialog when clicking Delete in dropdown", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for tenant list
        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Open dropdown and click Delete
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        const deleteOption = await screen.findByText("Delete");
        await user.click(deleteOption);

        // Verify confirmation dialog appears
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Tenant");
            expect(screen.getByText("Are you sure you want to delete this tenant?")).toBeInTheDocument();
        });
    });

    it("cancels delete when cancel is clicked in confirmation dialog", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
                action: () => ({ success: true }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for tenant list
        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Open dropdown and click Delete
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        const deleteOption = await screen.findByText("Delete");
        await user.click(deleteOption);

        // Wait for confirm dialog
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Tenant");
        });

        // Click Cancel
        const cancelButton = screen.getByTestId("confirm-dialog-cancel");
        await user.click(cancelButton);

        // Verify confirmation dialog is closed and tenant still exists
        await waitFor(() => {
            expect(screen.queryByTestId("confirm-dialog-title")).not.toBeInTheDocument();
        });
        expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
    });

    it("submits delete when confirmed in confirmation dialog", async () => {
        let actionCalled = false;
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
                action: async ({ request }: { request: Request }) => {
                    actionCalled = true;
                    const formData = await request.formData();
                    expect(formData.get("intent")).toBe("delete");
                    expect(formData.get("id")).toBe("1");
                    return { success: true };
                },
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Wait for tenant list
        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Open dropdown and click Delete
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);
        const deleteOption = await screen.findByText("Delete");
        await user.click(deleteOption);

        // Wait for confirm dialog
        await waitFor(() => {
            expect(screen.getByTestId("confirm-dialog-title")).toHaveTextContent("Delete Tenant");
        });

        // Click Delete (confirm)
        const confirmButton = screen.getByTestId("confirm-dialog-action");
        await user.click(confirmButton);

        // Verify action was called
        await waitFor(() => {
            expect(actionCalled).toBe(true);
        });
    });

    it("updates search input value when typing", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Type in search input
        const searchInput = screen.getByPlaceholderText("Search tenants...");
        await user.type(searchInput, "acme");

        expect(searchInput).toHaveValue("acme");
    });

    it("shows clear button when search query is active", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "acme",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants?search=acme"]} />);

        await waitFor(() => {
            expect(screen.getByText("Clear")).toBeInTheDocument();
        });
    });

    it("does not show clear button when no search query is active", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        expect(screen.queryByText("Clear")).not.toBeInTheDocument();
    });

    it("initializes search input with search value from loader data", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "existing-search",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants?search=existing-search"]} />);

        await waitFor(() => {
            const searchInput = screen.getByPlaceholderText("Search tenants...");
            expect(searchInput).toHaveValue("existing-search");
        });
    });

    it("shows dropdown menu items including Invitations and Webhooks links", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getAllByText("Acme Corp").length).toBeGreaterThan(0);
        });

        // Open dropdown menu for first tenant
        const menuButtons = await screen.findAllByRole("button", { name: /open menu/i });
        await user.click(menuButtons[0]);

        // Verify all menu items are present (some may appear in both mobile/desktop views)
        await waitFor(() => {
            expect(screen.getByText("Actions")).toBeInTheDocument();
            expect(screen.getAllByText("Edit").length).toBeGreaterThan(0);
            expect(screen.getAllByText("Invitations").length).toBeGreaterThan(0);
            expect(screen.getByText("Webhooks")).toBeInTheDocument();
            expect(screen.getAllByText("Delete").length).toBeGreaterThan(0);
        });
    });

    it("closes create dialog on successful action (useEffect)", async () => {
        let resolveAction: (value: { success: boolean }) => void;
        void new Promise<{ success: boolean }>((resolve) => {
            resolveAction = resolve;
        });

        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
                action: async () => {
                    resolveAction!({ success: true });
                    return { success: true };
                },
            },
        ]);

        const user = userEvent.setup();
        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        // Open create dialog
        const createButton = await screen.findByText("Create Tenant");
        await user.click(createButton);

        // Verify dialog is open
        expect(screen.getByRole("dialog")).toBeInTheDocument();

        // Fill in form and submit
        await user.type(screen.getByLabelText("Name"), "New Tenant");
        await user.type(screen.getByLabelText("Slug"), "new-tenant");

        const submitButton = screen.getByRole("button", { name: /Create$/i });
        await user.click(submitButton);

        // The dialog should close after successful action
        await waitFor(() => {
            expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
        });
    });

    it("displays search button", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            expect(screen.getByRole("button", { name: /Search/i })).toBeInTheDocument();
        });
    });

    it("shows tenant name as link to detail page", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/tenants",
                Component: WrappedPage,
                loader: () => ({
                    ...mockTenants,
                    search: "",
                }),
            },
            {
                path: "/dashboard/tenants/:id",
                Component: () => <div>Tenant Detail</div>,
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/tenants"]} />);

        await waitFor(() => {
            const elements = screen.getAllByText("Acme Corp");
            const link = elements.find((el) => el.closest("a"))?.closest("a");
            expect(link).toHaveAttribute("href", "/dashboard/tenants/1");
        });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    describe("action", () => {
        function createFormRequest(data: Record<string, string>) {
            const formData = new FormData();
            for (const [key, value] of Object.entries(data)) {
                formData.append(key, value);
            }
            return new Request("http://localhost/dashboard/tenants", {
                method: "POST",
                body: formData,
            });
        }

        it("create tenant calls tenantApi.create", async () => {
            vi.mocked(tenantApi.create).mockResolvedValue({
                data: { id: "3", name: "New Corp", slug: "new-corp", settings: {}, status: "active", created_at: "", updated_at: "" },
            });

            const request = createFormRequest({
                intent: "create",
                name: "New Corp",
                slug: "new-corp",
                logo_url: "https://example.com/logo.png",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(tenantApi.create).toHaveBeenCalledWith(
                { name: "New Corp", slug: "new-corp", logo_url: "https://example.com/logo.png" },
                "test-token"
            );
        });

        it("create tenant without logo_url", async () => {
            vi.mocked(tenantApi.create).mockResolvedValue({
                data: { id: "3", name: "No Logo", slug: "no-logo", settings: {}, status: "active", created_at: "", updated_at: "" },
            });

            const request = createFormRequest({
                intent: "create",
                name: "No Logo",
                slug: "no-logo",
                logo_url: "",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(tenantApi.create).toHaveBeenCalledWith(
                { name: "No Logo", slug: "no-logo", logo_url: undefined },
                "test-token"
            );
        });

        it("update tenant calls tenantApi.update", async () => {
            vi.mocked(tenantApi.update).mockResolvedValue({
                data: { id: "1", name: "Updated", slug: "updated", settings: {}, status: "active", created_at: "", updated_at: "" },
            });

            const request = createFormRequest({
                intent: "update",
                id: "1",
                name: "Updated",
                slug: "updated",
                logo_url: "",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(tenantApi.update).toHaveBeenCalledWith(
                "1",
                { name: "Updated", slug: "updated", logo_url: undefined },
                "test-token"
            );
        });

        it("delete tenant calls tenantApi.delete", async () => {
            vi.mocked(tenantApi.delete).mockResolvedValue(undefined);

            const request = createFormRequest({
                intent: "delete",
                id: "1",
            });

            const result = await action({ request, params: {}, context: {} });
            expect(result).toEqual({ success: true });
            expect(tenantApi.delete).toHaveBeenCalledWith("1", "test-token");
        });

        it("returns error on API failure", async () => {
            vi.mocked(tenantApi.create).mockRejectedValue(new Error("Slug already exists"));

            const request = createFormRequest({
                intent: "create",
                name: "Dup",
                slug: "dup",
                logo_url: "",
            });

            const response = await action({ request, params: {}, context: {} });
            expect(response).toBeInstanceOf(Response);
            const data = await (response as Response).json();
            expect(data.error).toBe("Slug already exists");
        });

        it("returns error for invalid intent", async () => {
            const request = createFormRequest({ intent: "invalid" });

            const response = await action({ request, params: {}, context: {} });
            expect(response).toBeInstanceOf(Response);
            const data = await (response as Response).json();
            expect(data.error).toBe("Invalid intent");
        });
    });
});
