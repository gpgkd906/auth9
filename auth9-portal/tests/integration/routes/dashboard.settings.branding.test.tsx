import { createRoutesStub } from "react-router";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import BrandingSettingsPage, { loader, action } from "~/routes/dashboard.settings.branding";
import { brandingApi } from "~/services/api";
import type { BrandingConfig } from "~/services/api";

// Mock branding API
vi.mock("~/services/api", () => ({
  brandingApi: {
    get: vi.fn(),
    update: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("mock-token"),
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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Logo Preview:")).toBeInTheDocument();
      const logoImage = screen.getByAltText("Logo preview") as HTMLImageElement;
      expect(logoImage.src).toBe("https://example.com/logo.png");
    });
  });

  it("shows live preview of branding changes", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview")).toBeInTheDocument();
      expect(screen.getByText("Sign In")).toBeInTheDocument();
      expect(screen.getByText("Forgot password?")).toBeInTheDocument();
    });
  });

  it("displays Save Changes button", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Changes")).toBeInTheDocument();
    });
  });

  it("displays Reset to Defaults button", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Reset to Defaults")).toBeInTheDocument();
    });
  });

  it("disables Reset to Defaults button when using default config", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const resetButton = screen.getByText("Reset to Defaults").closest("button");
      expect(resetButton).toBeDisabled();
    });
  });

  it("enables Reset to Defaults button when using custom config", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const resetButton = screen.getByText("Reset to Defaults").closest("button");
      expect(resetButton).not.toBeDisabled();
    });
  });

  it("handles loader error by returning default branding", async () => {
    vi.mocked(brandingApi.get).mockRejectedValue(new Error("API Error"));

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    // Should fallback to default branding
    await waitFor(() => {
      expect(screen.getByText("Login Page Branding")).toBeInTheDocument();
    });
  });

  it("shows all branding sections", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview Company")).toBeInTheDocument();
    });
  });

  it("renders form with correct input types", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const customCssTextarea = screen.getByPlaceholderText(/\.login-form/);
      expect(customCssTextarea).toHaveValue(".login-form { border-radius: 16px; }");
    });
  });

  it("loads allow_registration value correctly", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      const checkbox = screen.getByLabelText("Allow Registration") as HTMLInputElement;
      expect(checkbox.checked).toBe(true);
    });
  });

  it("updates color via color picker input", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Primary Color")).toBeInTheDocument();
    });

    // Find the hidden color picker input (type="color")
    const colorPicker = screen.getByLabelText("Choose Primary Color");
    // Simulate changing the color via the color picker
    fireEvent.change(colorPicker, { target: { value: "#AA0000" } });

    // The text input should update
    const primaryColorInput = screen.getByLabelText("Primary Color");
    expect(primaryColorInput).toHaveValue("#aa0000");
  });

  it("handles logo image error by hiding the element", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByAltText("Logo preview")).toBeInTheDocument();
    });

    // Trigger onError for the logo preview image
    const logoImg = screen.getByAltText("Logo preview") as HTMLImageElement;
    fireEvent.error(logoImg);

    // After error, the image should be hidden
    expect(logoImg.style.display).toBe("none");
  });

  it("resets form fields after reset action", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockCustomBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
        action: () => ({ success: true, message: "Branding reset to defaults", reset: true }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      // Verify custom values are loaded
      expect(screen.getByDisplayValue("Test Company")).toBeInTheDocument();
    });

    // Click the Reset to Defaults button
    const resetButton = screen.getByText("Reset to Defaults").closest("button")!;
    await user.click(resetButton);

    // After reset, fields should be reset to defaults
    await waitFor(() => {
      expect(screen.getByDisplayValue("#007AFF")).toBeInTheDocument();
    });
  });

  it("displays success message after saving", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
        action: () => ({ success: true, message: "Branding settings saved successfully" }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Changes")).toBeInTheDocument();
    });

    const saveButton = screen.getByText("Save Changes").closest("button")!;
    await user.click(saveButton);

    await waitFor(() => {
      expect(screen.getByText("Branding settings saved successfully")).toBeInTheDocument();
    });
  });

  it("displays error message when action returns error", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
        action: () => ({ error: "Failed to save branding" }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Changes")).toBeInTheDocument();
    });

    const saveButton = screen.getByText("Save Changes").closest("button")!;
    await user.click(saveButton);

    await waitFor(() => {
      expect(screen.getByText("Failed to save branding")).toBeInTheDocument();
    });
  });

  it("allows changing favicon URL", async () => {
    vi.mocked(brandingApi.get).mockResolvedValue(mockDefaultBranding);
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/branding",
        Component: BrandingSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/branding"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Favicon URL")).toBeInTheDocument();
    });

    const faviconInput = screen.getByLabelText("Favicon URL");
    await user.type(faviconInput, "https://example.com/favicon.ico");

    expect(faviconInput).toHaveValue("https://example.com/favicon.ico");
  });
});

describe("Branding action", () => {
  const mockBrandingConfig: BrandingConfig = {
    primary_color: "#007AFF",
    secondary_color: "#5856D6",
    background_color: "#F5F5F7",
    text_color: "#1D1D1F",
    allow_registration: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  function createFormRequest(data: Record<string, string>) {
    const formData = new FormData();
    for (const [key, value] of Object.entries(data)) {
      formData.append(key, value);
    }
    return new Request("http://localhost/dashboard/settings/branding", { method: "POST", body: formData });
  }

  it("saves branding config successfully", async () => {
    vi.mocked(brandingApi.update).mockResolvedValue({ data: mockBrandingConfig });

    const request = createFormRequest({
      intent: "save",
      logo_url: "https://logo.com/img.png",
      primary_color: "#FF0000",
      secondary_color: "#00FF00",
      background_color: "#0000FF",
      text_color: "#FFFFFF",
      custom_css: ".test { color: red; }",
      company_name: "My Company",
      favicon_url: "https://example.com/favicon.ico",
      allow_registration: "true",
    });

    const result = await action({ request, params: {}, context: {} });
    expect(result).toEqual({ success: true, message: "Branding settings saved successfully" });
    expect(brandingApi.update).toHaveBeenCalledWith(
      expect.objectContaining({
        logo_url: "https://logo.com/img.png",
        primary_color: "#FF0000",
        secondary_color: "#00FF00",
        background_color: "#0000FF",
        text_color: "#FFFFFF",
        custom_css: ".test { color: red; }",
        company_name: "My Company",
        favicon_url: "https://example.com/favicon.ico",
        allow_registration: true,
      }),
      "mock-token",
    );
  });

  it("resets branding to defaults", async () => {
    vi.mocked(brandingApi.update).mockResolvedValue({ data: mockBrandingConfig });

    const request = createFormRequest({
      intent: "reset",
    });

    const result = await action({ request, params: {}, context: {} });
    expect(result).toEqual({ success: true, message: "Branding reset to defaults", reset: true });
  });

  it("returns error on API failure", async () => {
    vi.mocked(brandingApi.update).mockRejectedValue(new Error("Permission denied"));

    const request = createFormRequest({
      intent: "save",
      primary_color: "#FF0000",
      secondary_color: "#00FF00",
      background_color: "#0000FF",
      text_color: "#FFFFFF",
    });

    const result = await action({ request, params: {}, context: {} });
    expect(result).toBeInstanceOf(Response);
    const body = await (result as Response).json();
    expect(body.error).toBe("Permission denied");
  });

  it("returns error for invalid intent", async () => {
    const request = createFormRequest({
      intent: "invalid",
    });

    const result = await action({ request, params: {}, context: {} });
    expect(result).toBeInstanceOf(Response);
    const body = await (result as Response).json();
    expect(body.error).toBe("Invalid intent");
  });

  it("omits empty optional fields", async () => {
    vi.mocked(brandingApi.update).mockResolvedValue({ data: mockBrandingConfig });

    const request = createFormRequest({
      intent: "save",
      logo_url: "",
      primary_color: "#007AFF",
      secondary_color: "#5856D6",
      background_color: "#F5F5F7",
      text_color: "#1D1D1F",
      custom_css: "",
      company_name: "",
      favicon_url: "",
    });

    const result = await action({ request, params: {}, context: {} });
    expect(result).toEqual({ success: true, message: "Branding settings saved successfully" });
    expect(brandingApi.update).toHaveBeenCalledWith(
      expect.objectContaining({
        logo_url: undefined,
        custom_css: undefined,
        company_name: undefined,
        favicon_url: undefined,
        allow_registration: false,
      }),
      "mock-token",
    );
  });
});
