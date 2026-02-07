import { createRoutesStub } from "react-router";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
    expect(identityProviderApi.delete).toHaveBeenCalledWith("google", undefined);
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

  it("action toggles provider enabled state", async () => {
    vi.mocked(identityProviderApi.update).mockResolvedValue({});

    const formData = new FormData();
    formData.append("intent", "toggle");
    formData.append("alias", "google");
    formData.append("enabled", "false");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ success: true });
    expect(identityProviderApi.update).toHaveBeenCalledWith(
      "google",
      { enabled: false },
      undefined
    );
  });

  it("action returns error for invalid intent", async () => {
    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "Invalid action" });
  });

  it("action handles toggle API error", async () => {
    vi.mocked(identityProviderApi.update).mockRejectedValue(
      new Error("Toggle failed")
    );

    const formData = new FormData();
    formData.append("intent", "toggle");
    formData.append("alias", "google");
    formData.append("enabled", "true");

    const request = new Request(
      "http://localhost/dashboard/settings/identity-providers",
      {
        method: "POST",
        body: formData,
      }
    );

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "Toggle failed" });
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

  it("renders provider alias and provider_id info", async () => {
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
    // Provider info line: "alias • provider_id"
    expect(screen.getByText(/google.*google/)).toBeInTheDocument();
    expect(screen.getByText(/github.*github/)).toBeInTheDocument();
  });

  it("renders provider with unknown template using fallback icon", async () => {
    const customProvider = {
      alias: "custom-ldap",
      display_name: "",
      provider_id: "ldap",
      enabled: true,
      config: {},
    };
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: [customProvider] }),
      },
    ]);

    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      // Fallback: provider_id.slice(0, 2).toUpperCase() => "LD"
      expect(screen.getByText("LD")).toBeInTheDocument();
    });
    // When display_name is empty and no template, falls back to alias
    expect(screen.getByText("custom-ldap")).toBeInTheDocument();
  });

  it("renders action error message from action data", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: mockProviders }),
        action: () => ({ error: "Something went wrong" }),
      },
    ]);

    const user = userEvent.setup();
    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Google")).toBeInTheDocument();
    });

    // Submit the delete form to trigger action
    const deleteButtons = screen.getAllByRole("button").filter(
      (btn) => btn.getAttribute("type") === "submit"
    );
    await user.click(deleteButtons[0]);

    await waitFor(() => {
      expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    });
  });

  it("renders action success message from action data", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/identity-providers",
        Component: IdentityProvidersPage,
        loader: () => ({ providers: mockProviders }),
        action: () => ({ success: true, message: "Identity provider created" }),
      },
    ]);

    const user = userEvent.setup();
    render(
      <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
    );

    await waitFor(() => {
      expect(screen.getByText("Google")).toBeInTheDocument();
    });

    // Submit delete form to trigger action
    const deleteButtons = screen.getAllByRole("button").filter(
      (btn) => btn.getAttribute("type") === "submit"
    );
    await user.click(deleteButtons[0]);

    await waitFor(() => {
      expect(screen.getByText("Identity provider created")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Create Dialog Tests
  // ============================================================================

  describe("create dialog", () => {
    it("opens create dialog when clicking 'Add provider' header button", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Identity Providers")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add Identity Provider")).toBeInTheDocument();
        expect(
          screen.getByText("Choose a provider type and configure its settings.")
        ).toBeInTheDocument();
      });
    });

    it("opens create dialog from empty state 'Add your first provider' button", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("No identity providers configured")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add your first provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Add Identity Provider")).toBeInTheDocument();
      });
    });

    it("shows all five provider templates in create dialog", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        const dialog = screen.getByRole("dialog");
        expect(within(dialog).getByText("Google")).toBeInTheDocument();
        expect(within(dialog).getByText("GitHub")).toBeInTheDocument();
        expect(within(dialog).getByText("Microsoft")).toBeInTheDocument();
        expect(within(dialog).getByText("OpenID Connect")).toBeInTheDocument();
        expect(within(dialog).getByText("SAML 2.0")).toBeInTheDocument();
      });
    });

    it("shows common fields after selecting a provider template", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Select Google template
      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByLabelText("Alias (identifier)")).toBeInTheDocument();
        expect(screen.getByLabelText("Display Name")).toBeInTheDocument();
        expect(screen.getByLabelText("Enabled")).toBeInTheDocument();
      });
    });

    it("shows clientId and clientSecret fields for Google template", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
        expect(screen.getByPlaceholderText("OAuth Client Secret")).toBeInTheDocument();
      });
    });

    it("auto-fills alias with provider_id when selecting a template", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const githubButton = within(dialog).getByText("GitHub").closest("button")!;
      await user.click(githubButton);

      await waitFor(() => {
        const aliasInput = screen.getByLabelText("Alias (identifier)") as HTMLInputElement;
        expect(aliasInput.value).toBe("github");
      });
    });

    it("shows OIDC-specific fields (authorizationUrl, tokenUrl) for OpenID Connect template", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const oidcButton = within(dialog).getByText("OpenID Connect").closest("button")!;
      await user.click(oidcButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
        expect(screen.getByPlaceholderText("OAuth Client Secret")).toBeInTheDocument();
        expect(
          screen.getByPlaceholderText("https://provider.com/oauth/authorize")
        ).toBeInTheDocument();
        expect(
          screen.getByPlaceholderText("https://provider.com/oauth/token")
        ).toBeInTheDocument();
      });
    });

    it("disables submit button when required config fields are not filled", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Select Google template
      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        // "Add provider" button in dialog should be disabled since clientId/clientSecret are empty
        const addBtn = within(dialog).getByRole("button", { name: "Add provider" });
        expect(addBtn).toBeDisabled();
      });
    });

    it("enables submit button when all required config fields are filled", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Select Google template
      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
      });

      // Fill in required fields
      await user.type(screen.getByPlaceholderText("OAuth Client ID"), "my-client-id");
      await user.type(screen.getByPlaceholderText("OAuth Client Secret"), "my-secret");

      await waitFor(() => {
        const addBtn = within(dialog).getByRole("button", { name: "Add provider" });
        expect(addBtn).not.toBeDisabled();
      });
    });

    it("disables submit button when no template is selected", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        const dialog = screen.getByRole("dialog");
        const addBtn = within(dialog).getByRole("button", { name: "Add provider" });
        expect(addBtn).toBeDisabled();
      });
    });

    it("allows typing in alias and displayName fields", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Select Microsoft template
      const dialog = screen.getByRole("dialog");
      const msButton = within(dialog).getByText("Microsoft").closest("button")!;
      await user.click(msButton);

      await waitFor(() => {
        expect(screen.getByLabelText("Alias (identifier)")).toBeInTheDocument();
      });

      // Clear the auto-filled alias and type a custom one
      const aliasInput = screen.getByLabelText("Alias (identifier)");
      await user.clear(aliasInput);
      await user.type(aliasInput, "ms-enterprise");

      const displayNameInput = screen.getByLabelText("Display Name");
      await user.type(displayNameInput, "Microsoft Enterprise SSO");

      expect(aliasInput).toHaveValue("ms-enterprise");
      expect(displayNameInput).toHaveValue("Microsoft Enterprise SSO");
    });

    it("closes create dialog when clicking Cancel", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Cancel" }));

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });

    it("resets form data when closing and reopening create dialog", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      // Open dialog
      await user.click(screen.getByText("Add provider"));
      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Select Google template
      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
      });

      // Type something in Client ID
      await user.type(screen.getByPlaceholderText("OAuth Client ID"), "test-id");

      // Cancel
      await user.click(screen.getByRole("button", { name: "Cancel" }));
      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });

      // Reopen
      await user.click(screen.getByText("Add provider"));
      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Provider type grid should be visible (no template selected), no config fields
      expect(screen.queryByPlaceholderText("OAuth Client ID")).not.toBeInTheDocument();
    });

    it("submits create form with correct hidden fields", async () => {
      vi.mocked(identityProviderApi.create).mockResolvedValue({});

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Select Google template
      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
      });

      // Fill required fields
      await user.type(screen.getByPlaceholderText("OAuth Client ID"), "my-client-id");
      await user.type(screen.getByPlaceholderText("OAuth Client Secret"), "my-secret");
      await user.type(screen.getByLabelText("Display Name"), "Google Login");

      // Submit
      const addBtn = within(dialog).getByRole("button", { name: "Add provider" });
      await user.click(addBtn);

      await waitFor(() => {
        expect(identityProviderApi.create).toHaveBeenCalled();
      });
    });

    it("switches between provider templates correctly", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");

      // Select Google first
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
        expect(
          screen.queryByPlaceholderText("https://provider.com/oauth/authorize")
        ).not.toBeInTheDocument();
      });

      // Switch to OIDC
      const oidcButton = within(dialog).getByText("OpenID Connect").closest("button")!;
      await user.click(oidcButton);

      await waitFor(() => {
        // OIDC has authorizationUrl and tokenUrl in addition to clientId/clientSecret
        expect(
          screen.getByPlaceholderText("https://provider.com/oauth/authorize")
        ).toBeInTheDocument();
        expect(
          screen.getByPlaceholderText("https://provider.com/oauth/token")
        ).toBeInTheDocument();
      });

      // Alias should update to oidc
      const aliasInput = screen.getByLabelText("Alias (identifier)") as HTMLInputElement;
      expect(aliasInput.value).toBe("oidc");
    });
  });

  // ============================================================================
  // Edit Dialog Tests
  // ============================================================================

  describe("edit dialog", () => {
    it("opens edit dialog when clicking the edit (pencil) button", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      // Find all edit (pencil icon) buttons - they are ghost variant buttons without type="submit"
      // The pencil buttons are between the switch and the trash button
      const allButtons = screen.getAllByRole("button");
      // Filter to find the edit buttons (they are ghost buttons not of type submit, not switches)
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });

      await user.click(editButtons[0]);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Edit Identity Provider")).toBeInTheDocument();
        expect(
          screen.getByText("Update the configuration for this identity provider.")
        ).toBeInTheDocument();
      });
    });

    it("populates form fields with existing provider data when editing", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      // Click edit button for Google provider
      const allButtons = screen.getAllByRole("button");
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });
      await user.click(editButtons[0]);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Alias should be pre-filled and disabled
      const aliasInput = screen.getByLabelText("Alias (identifier)") as HTMLInputElement;
      expect(aliasInput.value).toBe("google");
      expect(aliasInput).toBeDisabled();

      // Display name should be pre-filled
      const displayNameInput = screen.getByLabelText("Display Name") as HTMLInputElement;
      expect(displayNameInput.value).toBe("Google");

      // Config field (clientId) should be pre-filled
      const clientIdInput = screen.getByPlaceholderText("OAuth Client ID") as HTMLInputElement;
      expect(clientIdInput.value).toBe("google-client-id");
    });

    it("shows 'Save changes' button text in edit mode", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      const allButtons = screen.getAllByRole("button");
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });
      await user.click(editButtons[0]);

      await waitFor(() => {
        const dialog = screen.getByRole("dialog");
        expect(within(dialog).getByRole("button", { name: "Save changes" })).toBeInTheDocument();
      });
    });

    it("does not show provider type selection grid in edit mode", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      const allButtons = screen.getAllByRole("button");
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });
      await user.click(editButtons[0]);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // "Provider Type" label should not be in dialog (only shown in create mode)
      expect(screen.queryByText("Provider Type")).not.toBeInTheDocument();
    });

    it("submits update form when editing and clicking Save changes", async () => {
      vi.mocked(identityProviderApi.update).mockResolvedValue({});

      // Provide a provider with all required fields filled so Save is enabled
      const completeProvider = {
        alias: "google",
        display_name: "Google",
        provider_id: "google",
        enabled: true,
        config: {
          clientId: "google-client-id",
          clientSecret: "google-client-secret",
        },
      };

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [completeProvider] }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      const allButtons = screen.getAllByRole("button");
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });
      await user.click(editButtons[0]);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Modify the display name
      const displayNameInput = screen.getByLabelText("Display Name");
      await user.clear(displayNameInput);
      await user.type(displayNameInput, "Google Enterprise");

      const dialog = screen.getByRole("dialog");
      const saveBtn = within(dialog).getByRole("button", { name: "Save changes" });
      expect(saveBtn).not.toBeDisabled();
      await user.click(saveBtn);

      await waitFor(() => {
        expect(identityProviderApi.update).toHaveBeenCalled();
      });
    });

    it("closes edit dialog when clicking Cancel", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      const allButtons = screen.getAllByRole("button");
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });
      await user.click(editButtons[0]);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Cancel" }));

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Delete Form Tests
  // ============================================================================

  describe("delete provider", () => {
    it("submits delete form when clicking trash button", async () => {
      vi.mocked(identityProviderApi.delete).mockResolvedValue({});

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
          action,
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      // Delete buttons are the type="submit" buttons
      const deleteButtons = screen.getAllByRole("button").filter(
        (btn) => btn.getAttribute("type") === "submit"
      );
      expect(deleteButtons.length).toBe(2); // One per provider

      await user.click(deleteButtons[0]);

      await waitFor(() => {
        expect(identityProviderApi.delete).toHaveBeenCalled();
      });
    });
  });

  // ============================================================================
  // Form Field Interaction Tests
  // ============================================================================

  describe("form field interactions", () => {
    it("updates config fields when typing in Client ID and Client Secret", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));
      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText("OAuth Client ID")).toBeInTheDocument();
      });

      const clientIdInput = screen.getByPlaceholderText("OAuth Client ID");
      const clientSecretInput = screen.getByPlaceholderText("OAuth Client Secret");

      await user.type(clientIdInput, "test-client-123");
      await user.type(clientSecretInput, "secret-456");

      expect(clientIdInput).toHaveValue("test-client-123");
      expect(clientSecretInput).toHaveValue("secret-456");
    });

    it("updates OIDC-specific config fields when typing", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));
      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const oidcButton = within(dialog).getByText("OpenID Connect").closest("button")!;
      await user.click(oidcButton);

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("https://provider.com/oauth/authorize")
        ).toBeInTheDocument();
      });

      const authUrlInput = screen.getByPlaceholderText(
        "https://provider.com/oauth/authorize"
      );
      const tokenUrlInput = screen.getByPlaceholderText(
        "https://provider.com/oauth/token"
      );

      await user.type(authUrlInput, "https://example.com/authorize");
      await user.type(tokenUrlInput, "https://example.com/token");

      expect(authUrlInput).toHaveValue("https://example.com/authorize");
      expect(tokenUrlInput).toHaveValue("https://example.com/token");
    });

    it("enables submit after filling all 4 required OIDC fields", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));
      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const oidcButton = within(dialog).getByText("OpenID Connect").closest("button")!;
      await user.click(oidcButton);

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("https://provider.com/oauth/authorize")
        ).toBeInTheDocument();
      });

      // Initially disabled
      const addBtn = within(dialog).getByRole("button", { name: "Add provider" });
      expect(addBtn).toBeDisabled();

      // Fill all 4 required fields
      await user.type(screen.getByPlaceholderText("OAuth Client ID"), "cid");
      await user.type(screen.getByPlaceholderText("OAuth Client Secret"), "sec");
      await user.type(
        screen.getByPlaceholderText("https://provider.com/oauth/authorize"),
        "https://auth.example.com/authorize"
      );
      await user.type(
        screen.getByPlaceholderText("https://provider.com/oauth/token"),
        "https://auth.example.com/token"
      );

      await waitFor(() => {
        expect(addBtn).not.toBeDisabled();
      });
    });

    it("toggles enabled switch in create dialog form", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));
      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      const dialog = screen.getByRole("dialog");
      const googleButton = within(dialog).getByText("Google").closest("button")!;
      await user.click(googleButton);

      await waitFor(() => {
        expect(screen.getByLabelText("Enabled")).toBeInTheDocument();
      });

      // The enabled switch should be checked by default (formData.enabled = true)
      const enabledSwitch = screen.getByLabelText("Enabled");
      expect(enabledSwitch).toHaveAttribute("data-state", "checked");

      // Toggle it off
      await user.click(enabledSwitch);

      await waitFor(() => {
        expect(enabledSwitch).toHaveAttribute("data-state", "unchecked");
      });
    });
  });

  // ============================================================================
  // Dialog Close via Escape / Overlay
  // ============================================================================

  describe("dialog close via escape key", () => {
    it("closes create dialog when pressing Escape key", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: [] }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Add provider")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Add provider"));

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
      });

      // Press Escape to close (triggers onOpenChange(false))
      await user.keyboard("{Escape}");

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });

    it("closes edit dialog when pressing Escape key", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/identity-providers",
          Component: IdentityProvidersPage,
          loader: () => ({ providers: mockProviders }),
        },
      ]);

      const user = userEvent.setup();
      render(
        <RoutesStub initialEntries={["/dashboard/settings/identity-providers"]} />
      );

      await waitFor(() => {
        expect(screen.getByText("Google")).toBeInTheDocument();
      });

      // Click edit button
      const allButtons = screen.getAllByRole("button");
      const editButtons = allButtons.filter((btn) => {
        const isSvgChild = btn.querySelector("svg");
        const isSubmit = btn.getAttribute("type") === "submit";
        const isSwitch = btn.getAttribute("role") === "switch";
        const hasText = btn.textContent?.includes("Add");
        return isSvgChild && !isSubmit && !isSwitch && !hasText;
      });
      await user.click(editButtons[0]);

      await waitFor(() => {
        expect(screen.getByRole("dialog")).toBeInTheDocument();
        expect(screen.getByText("Edit Identity Provider")).toBeInTheDocument();
      });

      // Press Escape to close (triggers onOpenChange(false))
      await user.keyboard("{Escape}");

      await waitFor(() => {
        expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Action Error Handling in Component Context
  // ============================================================================

  describe("action error handling with non-Error objects", () => {
    it("action handles non-Error thrown object", async () => {
      vi.mocked(identityProviderApi.create).mockRejectedValue("string error");

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
      expect(response).toEqual({ error: "Operation failed" });
    });

    it("action handles delete API error", async () => {
      vi.mocked(identityProviderApi.delete).mockRejectedValue(
        new Error("Delete failed")
      );

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
      expect(response).toEqual({ error: "Delete failed" });
    });

    it("action handles update API error", async () => {
      vi.mocked(identityProviderApi.update).mockRejectedValue(
        new Error("Update failed")
      );

      const formData = new FormData();
      formData.append("intent", "update");
      formData.append("alias", "google");
      formData.append("displayName", "New Name");
      formData.append("enabled", "true");

      const request = new Request(
        "http://localhost/dashboard/settings/identity-providers",
        {
          method: "POST",
          body: formData,
        }
      );

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({ error: "Update failed" });
    });
  });
});
