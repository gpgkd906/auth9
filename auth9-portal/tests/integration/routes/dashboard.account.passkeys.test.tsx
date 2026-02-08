import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import AccountPasskeysPage, { loader, action } from "~/routes/dashboard.account.passkeys";
import { webauthnApi } from "~/services/api";

vi.mock("~/services/api", () => ({
    webauthnApi: {
        listPasskeys: vi.fn(),
        deletePasskey: vi.fn(),
        getRegisterUrl: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

const mockPasskeys = [
    {
        id: "pk-1",
        user_label: "MacBook Pro TouchID",
        credential_type: "webauthn-passwordless",
        created_at: "2024-01-15T00:00:00Z",
    },
    {
        id: "pk-2",
        user_label: "",
        credential_type: "webauthn",
        created_at: "2024-02-01T00:00:00Z",
    },
    {
        id: "pk-3",
        user_label: "Security Key",
        credential_type: "custom-type",
        created_at: "2024-03-01T00:00:00Z",
    },
];

function createFormRequest(data: Record<string, string>): Request {
    const formData = new FormData();
    for (const [key, value] of Object.entries(data)) {
        formData.append(key, value);
    }
    return new Request("http://localhost/dashboard/account/passkeys", {
        method: "POST",
        body: formData,
    });
}

describe("Account Passkeys Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader returns passkeys from API", async () => {
        vi.mocked(webauthnApi.listPasskeys).mockResolvedValue({ data: mockPasskeys });

        const request = new Request("http://localhost/dashboard/account/passkeys");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ passkeys: mockPasskeys });
    });

    it("loader returns empty array on error", async () => {
        vi.mocked(webauthnApi.listPasskeys).mockRejectedValue(new Error("fail"));

        const request = new Request("http://localhost/dashboard/account/passkeys");
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({ passkeys: [], error: "Failed to load passkeys" });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action deletes passkey", async () => {
        vi.mocked(webauthnApi.deletePasskey).mockResolvedValue(undefined);

        const request = createFormRequest({ intent: "delete", credentialId: "pk-1" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ success: true, message: "Passkey deleted" });
        expect(webauthnApi.deletePasskey).toHaveBeenCalledWith("pk-1", "test-token");
    });

    it("action returns registration URL", async () => {
        vi.mocked(webauthnApi.getRegisterUrl).mockResolvedValue({
            data: { url: "https://keycloak.example.com/register" },
        });

        const request = createFormRequest({
            intent: "register",
            redirectUri: "http://localhost/dashboard/account/passkeys",
        });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ redirect: "https://keycloak.example.com/register" });
    });

    it("action returns error on delete failure", async () => {
        vi.mocked(webauthnApi.deletePasskey).mockRejectedValue(new Error("Not found"));

        const request = createFormRequest({ intent: "delete", credentialId: "bad" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Not found" });
    });

    it("action returns generic error for non-Error throw", async () => {
        vi.mocked(webauthnApi.deletePasskey).mockRejectedValue("unexpected");

        const request = createFormRequest({ intent: "delete", credentialId: "pk-1" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Operation failed" });
    });

    it("action returns error for invalid intent", async () => {
        const request = createFormRequest({ intent: "invalid" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Invalid action" });
    });

    // ============================================================================
    // Rendering Tests
    // ============================================================================

    it("renders passkeys list with labels and types", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/passkeys",
                Component: AccountPasskeysPage,
                loader: () => ({ passkeys: mockPasskeys }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/passkeys"]} />);

        expect(await screen.findByText("Passkeys")).toBeInTheDocument();
        expect(screen.getByText("MacBook Pro TouchID")).toBeInTheDocument();
        expect(screen.getByText("Passwordless")).toBeInTheDocument();
        expect(screen.getByText("Two-Factor")).toBeInTheDocument();
        expect(screen.getByText("custom-type")).toBeInTheDocument();
    });

    it("shows 'Passkey' as default label when user_label is empty", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/passkeys",
                Component: AccountPasskeysPage,
                loader: () => ({ passkeys: [mockPasskeys[1]] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/passkeys"]} />);
        expect(await screen.findByText("Passkey")).toBeInTheDocument();
    });

    it("renders empty state with add button", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/passkeys",
                Component: AccountPasskeysPage,
                loader: () => ({ passkeys: [] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/passkeys"]} />);

        expect(await screen.findByText("No passkeys yet")).toBeInTheDocument();
        expect(screen.getByText("Add a passkey to sign in faster and more securely.")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /Add your first passkey/i })).toBeInTheDocument();
    });

    it("renders remove buttons for each passkey", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/passkeys",
                Component: AccountPasskeysPage,
                loader: () => ({ passkeys: [mockPasskeys[0]] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/passkeys"]} />);
        expect(await screen.findByRole("button", { name: /Remove/i })).toBeInTheDocument();
    });

    it("renders about passkeys info section", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/passkeys",
                Component: AccountPasskeysPage,
                loader: () => ({ passkeys: [] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/passkeys"]} />);

        expect(await screen.findByText("About Passkeys")).toBeInTheDocument();
        expect(screen.getByText("More secure")).toBeInTheDocument();
        expect(screen.getByText("Fast & easy")).toBeInTheDocument();
        expect(screen.getByText("Works everywhere")).toBeInTheDocument();
    });

    it("displays load error when present", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/passkeys",
                Component: AccountPasskeysPage,
                loader: () => ({ passkeys: [], error: "Failed to load passkeys" }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/passkeys"]} />);
        expect(await screen.findByText("Failed to load passkeys")).toBeInTheDocument();
    });
});
