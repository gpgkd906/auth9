import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import IdentityProvidersPage, {
  loader,
  action,
} from "~/routes/dashboard.settings.identity-providers";
import { identityProviderApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  identityProviderApi: {
    list: vi.fn(),
    create: vi.fn(),
    update: vi.fn(),
    delete: vi.fn(),
  },
}));

const mockProviders = [
  {
    alias: "google",
    display_name: "Google",
    provider_id: "google",
    enabled: true,
    config: {
      clientId: "google-client-id",
    },
  },
  {
    alias: "github",
    display_name: "GitHub",
    provider_id: "github",
    enabled: false,
    config: {
      clientId: "github-client-id",
    },
  },
];

describe("Identity Providers Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(identityProviderApi.list).mockResolvedValue({
      data: mockProviders,
    });
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns providers list", async () => {
    const response = await loader({ request: new Request("http://localhost"), params: {}, context: {} });

    expect(response).toEqual({ providers: mockProviders });
    expect(identityProviderApi.list).toHaveBeenCalled();
  });

  it("loader handles API error", async () => {
    vi.mocked(identityProviderApi.list).mockRejectedValue(
      new Error("API Error")
    );

    const response = await loader({ request: new Request("http://localhost"), params: {}, context: {} });

    expect(response).toEqual({
      providers: [],
      error: "Failed to load identity providers",
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action creates provider successfully", async () => {
    vi.mocked(identityProviderApi.create).mockResolvedValue({});

    const formData = new FormData();
    formData.append("intent", "create");
    formData.append("providerId", "google");
    formData.append("displayName", "Google Login");
    formData.append("enabled", "true");
    formData.append("clientId", "my-client-id");
    formData.append("clientSecret", "my-secret");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({
      success: true,
      message: "Identity provider created",
    });
    expect(identityProviderApi.create).toHaveBeenCalled();
  });

  it("action updates provider successfully", async () => {
    vi.mocked(identityProviderApi.update).mockResolvedValue({});

    const formData = new FormData();
    formData.append("intent", "update");
    formData.append("alias", "google");
    formData.append("displayName", "Google SSO");
    formData.append("enabled", "true");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({
      success: true,
      message: "Identity provider updated",
    });
    expect(identityProviderApi.update).toHaveBeenCalled();
  });

  it("action deletes provider successfully", async () => {
    vi.mocked(identityProviderApi.delete).mockResolvedValue({});

    const formData = new FormData();
    formData.append("intent", "delete");
    formData.append("alias", "google");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({
      success: true,
      message: "Identity provider deleted",
    });
    expect(identityProviderApi.delete).toHaveBeenCalledWith("google");
  });

  it("action handles API error", async () => {
    vi.mocked(identityProviderApi.create).mockRejectedValue(
      new Error("Provider already exists")
    );

    const formData = new FormData();
    formData.append("intent", "create");
    formData.append("providerId", "google");
    formData.append("displayName", "Google");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "Provider already exists" });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: mockProviders }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Identity Providers")).toBeInTheDocument();
    });
  });

  it("renders add provider button", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: [] }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Add provider")).toBeInTheDocument();
    });
  });

  it("renders configured providers list", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: mockProviders }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Google")).toBeInTheDocument();
    });
    expect(screen.getByText("GitHub")).toBeInTheDocument();
  });

  it("renders provider with switch controls", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: mockProviders }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      // Check for switch components (role="switch")
      const switches = screen.getAllByRole("switch");
      expect(switches.length).toBe(2);
    });
  });

  it("renders empty state when no providers configured", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: [] }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(
        screen.getByText("No identity providers configured")
      ).toBeInTheDocument();
    });
  });

  it("renders error message", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({
          providers: [],
          error: "Failed to load identity providers",
        }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(
        screen.getByText("Failed to load identity providers")
      ).toBeInTheDocument();
    });
  });
});
