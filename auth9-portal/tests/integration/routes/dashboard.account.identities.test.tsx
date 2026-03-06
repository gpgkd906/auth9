import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import AccountIdentitiesPage, { loader, action } from "~/routes/dashboard.account.identities";
import { identityProviderApi } from "~/services/api";

vi.mock("~/services/api", () => ({
    identityProviderApi: {
        list: vi.fn(),
        listMyLinkedIdentities: vi.fn(),
        unlinkIdentity: vi.fn(),
    },
}));

vi.mock("~/services/session.server", () => ({
    getAccessToken: vi.fn().mockResolvedValue("test-token"),
    requireAuthWithUpdate: vi.fn().mockResolvedValue({
        session: {
            accessToken: "test-token",
            refreshToken: "test-refresh-token",
            idToken: [
                "header",
                Buffer.from(JSON.stringify({
                    iss: "http://localhost:8081/realms/auth9",
                    session_state: "session-123",
                    azp: "auth9-portal",
                })).toString("base64url"),
                "signature",
            ].join("."),
            expiresAt: Date.now() + 3600000,
        },
        headers: undefined,
    }),
}));

const mockIdentities = [
    {
        id: "id-1",
        provider_type: "google",
        provider_alias: "google",
        external_user_id: "ext-1",
        external_email: "alice@gmail.com",
        linked_at: "2024-01-01T00:00:00Z",
    },
    {
        id: "id-2",
        provider_type: "github",
        provider_alias: "",
        external_user_id: "ext-2",
        external_email: "",
        linked_at: "2024-02-01T00:00:00Z",
    },
    {
        id: "id-3",
        provider_type: "microsoft",
        provider_alias: "ms-corp",
        external_user_id: "ext-3",
        external_email: "alice@corp.com",
        linked_at: "2024-03-01T00:00:00Z",
    },
    {
        id: "id-4",
        provider_type: "apple",
        provider_alias: "",
        external_user_id: "ext-4",
        external_email: "",
        linked_at: "2024-04-01T00:00:00Z",
    },
    {
        id: "id-5",
        provider_type: "facebook",
        provider_alias: "",
        external_user_id: "ext-5",
        external_email: "alice@fb.com",
        linked_at: "2024-05-01T00:00:00Z",
    },
    {
        id: "id-6",
        provider_type: "saml",
        provider_alias: "okta",
        external_user_id: "ext-6",
        external_email: "",
        linked_at: "2024-06-01T00:00:00Z",
    },
];

function createFormRequest(data: Record<string, string>): Request {
    const formData = new FormData();
    for (const [key, value] of Object.entries(data)) {
        formData.append(key, value);
    }
    return new Request("http://localhost/dashboard/account/identities", {
        method: "POST",
        body: formData,
        headers: { "Accept-Language": "en-US" },
    });
}

describe("Account Identities Page", () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.mocked(identityProviderApi.list).mockResolvedValue({
            data: [],
        });
    });

    // ============================================================================
    // Loader Tests
    // ============================================================================

    it("loader returns identities from API", async () => {
        vi.mocked(identityProviderApi.listMyLinkedIdentities).mockResolvedValue({
            data: mockIdentities,
        });
        vi.mocked(identityProviderApi.list).mockResolvedValue({
            data: [
                {
                    alias: "linkedin",
                    provider_id: "linkedin",
                    display_name: "LinkedIn",
                    enabled: true,
                    config: {},
                },
                {
                    alias: "google",
                    provider_id: "google",
                    display_name: "Google",
                    enabled: true,
                    config: {},
                },
            ],
        });

        const request = new Request("http://localhost/dashboard/account/identities", {
            headers: { "Accept-Language": "en-US" },
        });
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({
            identities: mockIdentities,
            availableProviders: [
                {
                    alias: "linkedin",
                    provider_id: "linkedin",
                    display_name: "LinkedIn",
                },
            ],
        });
    });

    it("loader redirects when no access token", async () => {
        const { getAccessToken } = await import("~/services/session.server");
        vi.mocked(getAccessToken).mockResolvedValueOnce(null);

        const request = new Request("http://localhost/dashboard/account/identities", {
            headers: { "Accept-Language": "en-US" },
        });
        try {
            await loader({ request, params: {}, context: {} });
            expect.fail("Expected redirect");
        } catch (response) {
            expect((response as Response).status).toBe(302);
            expect((response as Response).headers.get("Location")).toBe("/login");
        }
    });

    it("loader returns empty array on error", async () => {
        vi.mocked(identityProviderApi.listMyLinkedIdentities).mockRejectedValue(new Error("fail"));

        const request = new Request("http://localhost/dashboard/account/identities", {
            headers: { "Accept-Language": "en-US" },
        });
        const result = await loader({ request, params: {}, context: {} });

        expect(result).toEqual({
            identities: [],
            availableProviders: [],
            error: "Failed to load linked identities",
        });
    });

    // ============================================================================
    // Action Tests
    // ============================================================================

    it("action unlinks identity successfully", async () => {
        vi.mocked(identityProviderApi.unlinkIdentity).mockResolvedValue(undefined);

        const request = createFormRequest({ intent: "unlink", identityId: "id-1" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ success: true, message: "Identity unlinked successfully" });
        expect(identityProviderApi.unlinkIdentity).toHaveBeenCalledWith("id-1", "test-token");
    });

    it("action redirects to provider linking flow", async () => {
        const request = createFormRequest({ intent: "link", providerAlias: "github" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toBeInstanceOf(Response);
        expect((result as Response).status).toBe(302);
        const location = (result as Response).headers.get("Location");
        expect(location).toContain("/realms/auth9/broker/github/link");
        expect(location).toContain("client_id=auth9-portal");
        expect(location).toContain(
            "redirect_uri=http%3A%2F%2Flocalhost%2Fdashboard%2Faccount%2Fidentities"
        );
        expect(location).toContain("nonce=");
        expect(location).toContain("hash=");
    });

    it("action returns error when not authenticated", async () => {
        const { getAccessToken } = await import("~/services/session.server");
        vi.mocked(getAccessToken).mockResolvedValueOnce(null);

        const request = createFormRequest({ intent: "unlink", identityId: "id-1" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Not authenticated" });
    });

    it("action returns error on unlink failure", async () => {
        vi.mocked(identityProviderApi.unlinkIdentity).mockRejectedValue(new Error("Cannot unlink primary"));

        const request = createFormRequest({ intent: "unlink", identityId: "id-1" });
        const result = await action({ request, params: {}, context: {} });

        expect(result).toEqual({ error: "Cannot unlink primary" });
    });

    it("action returns generic error for non-Error throw", async () => {
        vi.mocked(identityProviderApi.unlinkIdentity).mockRejectedValue("unexpected");

        const request = createFormRequest({ intent: "unlink", identityId: "id-1" });
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

    it("renders linked identities list", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({ identities: mockIdentities, availableProviders: [] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);

        expect(await screen.findByText("Linked Identities")).toBeInTheDocument();
        expect(screen.getByText("alice@gmail.com")).toBeInTheDocument();
        expect(screen.getByText("Google")).toBeInTheDocument();
    });

    it("renders provider icons for all types", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({ identities: mockIdentities, availableProviders: [] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);

        await screen.findByText("Linked Identities");
        expect(screen.getByText("G")).toBeInTheDocument();
        expect(screen.getByText("GH")).toBeInTheDocument();
        expect(screen.getByText("MS")).toBeInTheDocument();
        expect(screen.getByText("AP")).toBeInTheDocument();
        expect(screen.getByText("FB")).toBeInTheDocument();
        expect(screen.getByText("SA")).toBeInTheDocument();
    });

    it("renders external_user_id when no email", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({
                    identities: [{
                        id: "id-x",
                        provider_type: "github",
                        provider_alias: "",
                        external_user_id: "github-user-123",
                        external_email: "",
                        linked_at: "2024-01-01T00:00:00Z",
                    }],
                    availableProviders: [],
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);
        expect(await screen.findByText("github-user-123")).toBeInTheDocument();
    });

    it("renders empty state when no identities", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({ identities: [], availableProviders: [] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);
        expect(await screen.findByText("No linked identities")).toBeInTheDocument();
    });

    it("renders unlink buttons for each identity", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({ identities: [mockIdentities[0]], availableProviders: [] }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);
        expect(await screen.findByRole("button", { name: /Unlink/i })).toBeInTheDocument();
    });

    it("displays load error when present", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({
                    identities: [],
                    availableProviders: [],
                    error: "Failed to load linked identities",
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);
        expect(await screen.findByText("Failed to load linked identities")).toBeInTheDocument();
    });

    it("renders available link providers", async () => {
        const RoutesStub = createRoutesStub([
            {
                path: "/dashboard/account/identities",
                Component: AccountIdentitiesPage,
                loader: () => ({
                    identities: [],
                    availableProviders: [
                        { alias: "github", provider_id: "github", display_name: "GitHub" },
                    ],
                }),
            },
        ]);

        render(<RoutesStub initialEntries={["/dashboard/account/identities"]} />);

        expect(await screen.findByText("Link another identity")).toBeInTheDocument();
        expect(screen.getByRole("button", { name: /link github/i })).toBeInTheDocument();
    });
});
