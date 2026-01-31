import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import BrandingSettingsPage, { loader } from "~/routes/dashboard.settings.branding";
import { brandingApi } from "~/services/api";

// Mock branding API
vi.mock("~/services/api", () => ({
  brandingApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
}));

describe("Branding Settings Page", () => {
  const mockDefaultBranding = {
    data: {
      primary_color: "#007AFF",
      secondary_color: "#5856D6",
      background_color: "#F5F5F7",
      text_color: "#1D1D1F",
      allow_registration: false,
    },
  };

  const mockCustomBranding = {
    data: {
      logo_url: "https://example.com/logo.png",
      primary_color: "#FF0000",
      secondary_color: "#00FF00",
      background_color: "#0000FF",
      text_color: "#FFFFFF",
      custom_css: ".login-form { border-radius: 16px; }",
      company_name: "Test Company",
      favicon_url: "https://example.com/favicon.ico",
      allow_registration: true,
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders branding settings page with default values", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Login Page Branding")).toBeInTheDocument();
      expect(screen.getByLabelText("Company Name")).toBeInTheDocument();
      expect(screen.getByLabelText("Logo URL")).toBeInTheDocument();
      expect(screen.getByLabelText("Favicon URL")).toBeInTheDocument();
      expect(screen.getByLabelText("Allow Registration")).toBeInTheDocument();
    });
  });

  it("loads and displays custom branding configuration", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("Test Company")).toBeInTheDocument();
      expect(screen.getByDisplayValue("https://example.com/logo.png")).toBeInTheDocument();
      expect(screen.getByDisplayValue("https://example.com/favicon.ico")).toBeInTheDocument();
      expect(screen.getByDisplayValue("#FF0000")).toBeInTheDocument();
      expect(screen.getByDisplayValue("#00FF00")).toBeInTheDocument();
      expect(screen.getByDisplayValue("#0000FF")).toBeInTheDocument();
      expect(screen.getByDisplayValue("#FFFFFF")).toBeInTheDocument();
    });
  });

  it("displays color pickers for all color fields", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Primary Color")).toBeInTheDocument();
      expect(screen.getByLabelText("Secondary Color")).toBeInTheDocument();
      expect(screen.getByLabelText("Background Color")).toBeInTheDocument();
      expect(screen.getByLabelText("Text Color")).toBeInTheDocument();
    });
  });

  it("allows user to change company name", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);
    const user = userEvent.setup();

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Company Name")).toBeInTheDocument();
    });

    const companyNameInput = screen.getByLabelText("Company Name");
    await user.clear(companyNameInput);
    await user.type(companyNameInput, "New Company Name");

    expect(companyNameInput).toHaveValue("New Company Name");
  });

  it("allows user to change logo URL", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);
    const user = userEvent.setup();

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Logo URL")).toBeInTheDocument();
    });

    const logoInput = screen.getByLabelText("Logo URL");
    await user.type(logoInput, "https://newlogo.com/logo.png");

    expect(logoInput).toHaveValue("https://newlogo.com/logo.png");
  });

  it("allows user to change primary color", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);
    const user = userEvent.setup();

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Primary Color")).toBeInTheDocument();
    });

    const primaryColorInput = screen.getByLabelText("Primary Color") as HTMLInputElement;
    await user.clear(primaryColorInput);
    await user.type(primaryColorInput, "#FF0000");

    expect(primaryColorInput).toHaveValue("#FF0000");
  });

  it("allows user to toggle allow registration checkbox", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);
    const user = userEvent.setup();

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Allow Registration")).toBeInTheDocument();
    });

    const checkbox = screen.getByLabelText("Allow Registration") as HTMLInputElement;
    expect(checkbox.checked).toBe(false);

    await user.click(checkbox);
    expect(checkbox.checked).toBe(true);
  });

  it("allows user to enter custom CSS", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Custom CSS")).toBeInTheDocument();
    });

    const customCssTextarea = screen.getByPlaceholderText(/\.login-form/);
    // Use fireEvent.change instead of user.type to avoid issues with special characters
    fireEvent.change(customCssTextarea, { target: { value: ".custom { color: red; }" } });

    expect(customCssTextarea).toHaveValue(".custom { color: red; }");
  });

  it("shows logo preview when logo URL is provided", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Logo Preview:")).toBeInTheDocument();
      const logoImage = screen.getByAltText("Logo preview") as HTMLImageElement;
      expect(logoImage.src).toBe("https://example.com/logo.png");
    });
  });

  it("shows live preview of branding changes", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview")).toBeInTheDocument();
      expect(screen.getByText("Sign In")).toBeInTheDocument();
      expect(screen.getByText("Forgot password?")).toBeInTheDocument();
    });
  });

  it("displays Save Changes button", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Changes")).toBeInTheDocument();
    });
  });

  it("displays Reset to Defaults button", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Reset to Defaults")).toBeInTheDocument();
    });
  });

  it("disables Reset to Defaults button when using default config", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const resetButton = screen.getByText("Reset to Defaults").closest("button");
      expect(resetButton).toBeDisabled();
    });
  });

  it("enables Reset to Defaults button when using custom config", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const resetButton = screen.getByText("Reset to Defaults").closest("button");
      expect(resetButton).not.toBeDisabled();
    });
  });

  it("handles loader error by returning default branding", async () => {
    vi.mocked(brandingApi.get).mockRejectedValue(new Error("API Error"));

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    // Should fallback to default branding
    await waitFor(() => {
      expect(screen.getByText("Login Page Branding")).toBeInTheDocument();
    });
  });

  it("shows all branding sections", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Company Identity")).toBeInTheDocument();
      expect(screen.getByText("Login Options")).toBeInTheDocument();
      expect(screen.getByText("Colors")).toBeInTheDocument();
      expect(screen.getByText("Preview")).toBeInTheDocument();
      expect(screen.getByText("Custom CSS")).toBeInTheDocument();
    });
  });

  it("displays helper text for form fields", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Displayed on the login page")).toBeInTheDocument();
      expect(screen.getByText("Recommended size: 200x50 pixels")).toBeInTheDocument();
      expect(screen.getByText("Browser tab icon (ICO or PNG)")).toBeInTheDocument();
      expect(screen.getByText('Show "Create account" link on the login page')).toBeInTheDocument();
      expect(screen.getByText("Add custom CSS rules to further customize the login page. Maximum 50KB.")).toBeInTheDocument();
    });
  });

  it("displays company name in preview when logo URL is not provided", async () => {
    const brandingWithCompanyName = {
      data: {
        ...mockDefaultBranding.data,
        company_name: "Preview Company",
      },
    };

    vi.mocked(brandingApi.get).mockResolvedValue(brandingWithCompanyName);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview Company")).toBeInTheDocument();
    });
  });

  it("renders form with correct input types", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const logoInput = screen.getByLabelText("Logo URL");
      expect(logoInput).toHaveAttribute("type", "url");

      const faviconInput = screen.getByLabelText("Favicon URL");
      expect(faviconInput).toHaveAttribute("type", "url");

      const companyNameInput = screen.getByLabelText("Company Name");
      expect(companyNameInput).toHaveAttribute("maxLength", "100");
    });
  });

  it("has correct page title", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Login Page Branding")).toBeInTheDocument();
      expect(
        screen.getByText(
          "Customize the appearance of your login pages. Changes will be applied to all Keycloak login forms."
        )
      ).toBeInTheDocument();
    });
  });

  it("displays all four color input fields with correct labels", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const primaryColorInput = screen.getByLabelText("Primary Color");
      expect(primaryColorInput).toHaveAttribute("name", "primary_color");

      const secondaryColorInput = screen.getByLabelText("Secondary Color");
      expect(secondaryColorInput).toHaveAttribute("name", "secondary_color");

      const backgroundColorInput = screen.getByLabelText("Background Color");
      expect(backgroundColorInput).toHaveAttribute("name", "background_color");

      const textColorInput = screen.getByLabelText("Text Color");
      expect(textColorInput).toHaveAttribute("name", "text_color");
    });
  });

  it("loads custom CSS value correctly", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const customCssTextarea = screen.getByPlaceholderText(/\.login-form/);
      expect(customCssTextarea).toHaveValue(".login-form { border-radius: 16px; }");
    });
  });

  it("loads allow_registration value correctly", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const checkbox = screen.getByLabelText("Allow Registration") as HTMLInputElement;
      expect(checkbox.checked).toBe(true);
    });
  });
});
