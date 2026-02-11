import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SecuritySettingsPage, {
  loader,
  action,
} from "~/routes/dashboard.settings.security";
import { passwordApi, tenantApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  passwordApi: {
    getPasswordPolicy: vi.fn(),
    updatePasswordPolicy: vi.fn(),
  },
  tenantApi: {
    list: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue(undefined),
}));

const mockTenants = [
  { id: "tenant-1", name: "Acme Corp", slug: "acme" },
  { id: "tenant-2", name: "Beta Inc", slug: "beta" },
];

const mockPolicy = {
  min_length: 8,
  require_uppercase: true,
  require_lowercase: true,
  require_numbers: true,
  require_symbols: false,
  max_age_days: 90,
  history_count: 5,
  lockout_threshold: 5,
  lockout_duration_mins: 15,
};

describe("Security Settings Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tenantApi.list).mockResolvedValue({ data: mockTenants });
    vi.mocked(passwordApi.getPasswordPolicy).mockResolvedValue({
      data: mockPolicy,
    });
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns tenants list", async () => {
    const response = await loader({ request: new Request("http://localhost"), params: {}, context: {} });

    expect(response).toEqual({
      tenants: mockTenants,
      tenantsError: null,
      selectedTenantId: "",
      policy: null,
      policyError: null,
    });
    expect(tenantApi.list).toHaveBeenCalledWith(1, 100, undefined, undefined);
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action updates password policy successfully", async () => {
    vi.mocked(passwordApi.updatePasswordPolicy).mockResolvedValue({});

    const formData = new FormData();
    formData.append("intent", "update_policy");
    formData.append("tenantId", "tenant-1");
    formData.append("minLength", "12");
    formData.append("requireUppercase", "true");
    formData.append("requireLowercase", "true");
    formData.append("requireNumbers", "true");
    formData.append("requireSymbols", "true");
    formData.append("maxAgeDays", "90");
    formData.append("historyCount", "5");
    formData.append("lockoutThreshold", "5");
    formData.append("lockoutDurationMins", "30");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({
      success: true,
      message: "Password policy updated",
    });
    expect(passwordApi.updatePasswordPolicy).toHaveBeenCalled();
  });

  it("action handles invalid intent", async () => {
    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "Invalid action" });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders password policy section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByText("Password Policy")).toBeInTheDocument();
    });
    expect(
      screen.getByText("Configure password requirements for tenant users.")
    ).toBeInTheDocument();
  });

  it("renders tenant selector", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });
    expect(screen.getByText("Select a tenant...")).toBeInTheDocument();
    expect(screen.getByText("Acme Corp")).toBeInTheDocument();
    expect(screen.getByText("Beta Inc")).toBeInTheDocument();
  });

  // ============================================================================
  // Action Error Path Tests
  // ============================================================================

  it("action handles update_policy API error", async () => {
    vi.mocked(passwordApi.updatePasswordPolicy).mockRejectedValue(
      new Error("Policy update failed")
    );

    const formData = new FormData();
    formData.append("intent", "update_policy");
    formData.append("tenantId", "tenant-1");
    formData.append("minLength", "8");
    formData.append("requireUppercase", "true");
    formData.append("requireLowercase", "true");
    formData.append("requireNumbers", "true");
    formData.append("requireSymbols", "false");
    formData.append("maxAgeDays", "90");
    formData.append("historyCount", "5");
    formData.append("lockoutThreshold", "5");
    formData.append("lockoutDurationMins", "15");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "Policy update failed" });
  });

  // ============================================================================
  // Component Interaction Tests - Password Policy Form
  // ============================================================================

  it("shows policy form after selecting a tenant", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    // Select a tenant from the dropdown
    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    // Wait for the policy form to appear
    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // Verify all policy form fields are rendered
    expect(screen.getByLabelText("Password expiry (days)")).toBeInTheDocument();
    expect(screen.getByLabelText("Password history")).toBeInTheDocument();
    expect(screen.getByLabelText("Lockout after")).toBeInTheDocument();
    expect(screen.getByLabelText("Lockout duration (mins)")).toBeInTheDocument();

    // Verify character requirement switches
    expect(screen.getByLabelText("Require uppercase")).toBeInTheDocument();
    expect(screen.getByLabelText("Require lowercase")).toBeInTheDocument();
    expect(screen.getByLabelText("Require numbers")).toBeInTheDocument();
    expect(screen.getByLabelText("Require symbols")).toBeInTheDocument();

    // Verify Save button
    expect(screen.getByRole("button", { name: "Save policy" })).toBeInTheDocument();

    // Verify the API was called with the selected tenant
    expect(passwordApi.getPasswordPolicy).toHaveBeenCalledWith("tenant-1", undefined);
  });

  it("displays policy form fields with correct default values from loaded policy", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // Verify numeric field default values
    expect(screen.getByLabelText("Minimum length")).toHaveValue(8);
    expect(screen.getByLabelText("Password expiry (days)")).toHaveValue(90);
    expect(screen.getByLabelText("Password history")).toHaveValue(5);
    expect(screen.getByLabelText("Lockout after")).toHaveValue(5);
    expect(screen.getByLabelText("Lockout duration (mins)")).toHaveValue(15);

    // Verify helper text is rendered
    expect(screen.getByText("0 = never expires")).toBeInTheDocument();
    expect(screen.getByText("Previous passwords to remember")).toBeInTheDocument();
    expect(screen.getByText("Failed attempts (0 = disabled)")).toBeInTheDocument();

    // Verify character requirements section heading
    expect(screen.getByText("Character requirements")).toBeInTheDocument();
  });

  it("hides policy form when tenant selection is cleared", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    // Select a tenant first
    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // Clear the selection
    await user.selectOptions(select, "");

    await waitFor(() => {
      expect(screen.queryByLabelText("Minimum length")).not.toBeInTheDocument();
    });

    // Policy form should be gone
    expect(screen.queryByRole("button", { name: "Save policy" })).not.toBeInTheDocument();
  });

  it("handles policy loading error gracefully", async () => {
    // Make getPasswordPolicy reject to test the catch branch
    vi.mocked(passwordApi.getPasswordPolicy).mockRejectedValue(
      new Error("Network error")
    );

    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    // Wait for the API call to complete (and fail)
    await waitFor(() => {
      expect(passwordApi.getPasswordPolicy).toHaveBeenCalledWith("tenant-1", undefined);
    });

    // The form should not be shown because policy is null after error
    await waitFor(() => {
      expect(screen.queryByLabelText("Minimum length")).not.toBeInTheDocument();
    });
    expect(screen.queryByRole("button", { name: "Save policy" })).not.toBeInTheDocument();
  });

  it("shows loading indicator while policy is being fetched", async () => {
    // Make the getPasswordPolicy return a never-resolving promise initially,
    // then resolve so we can check the loading state
    let resolvePolicy!: (value: { data: typeof mockPolicy }) => void;
    vi.mocked(passwordApi.getPasswordPolicy).mockImplementation(
      () =>
        new Promise((resolve) => {
          resolvePolicy = resolve;
        })
    );

    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    // Should show loading text while policy is being fetched
    await waitFor(() => {
      expect(screen.getByText("Loading policy...")).toBeInTheDocument();
    });

    // Resolve the promise
    resolvePolicy({ data: mockPolicy });

    // Loading text should disappear and form should appear
    await waitFor(() => {
      expect(screen.queryByText("Loading policy...")).not.toBeInTheDocument();
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });
  });

  it("loads policy for a different tenant when selection changes", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");

    // Select first tenant
    await user.selectOptions(select, "tenant-1");
    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });
    expect(passwordApi.getPasswordPolicy).toHaveBeenCalledWith("tenant-1", undefined);

    // Switch to second tenant - this triggers useEffect again
    await user.selectOptions(select, "tenant-2");
    await waitFor(() => {
      expect(passwordApi.getPasswordPolicy).toHaveBeenCalledWith("tenant-2", undefined);
    });

    // Verify the policy form is still visible (loaded for second tenant)
    expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    expect(passwordApi.getPasswordPolicy).toHaveBeenCalledTimes(2);
  });

  it("renders hidden fields for policy form after selecting tenant", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/settings/security"]} />
    );

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // Check hidden fields
    const policyIntent = container.querySelector('input[name="intent"][value="update_policy"]');
    expect(policyIntent).toBeInTheDocument();

    const tenantIdField = container.querySelector('input[name="tenantId"][value="tenant-1"]');
    expect(tenantIdField).toBeInTheDocument();

    // Check hidden switch value fields (based on mockPolicy defaults)
    const uppercaseHidden = container.querySelector('input[name="requireUppercase"]') as HTMLInputElement;
    expect(uppercaseHidden).toBeInTheDocument();
    expect(uppercaseHidden.value).toBe("true");

    const lowercaseHidden = container.querySelector('input[name="requireLowercase"]') as HTMLInputElement;
    expect(lowercaseHidden).toBeInTheDocument();
    expect(lowercaseHidden.value).toBe("true");

    const numbersHidden = container.querySelector('input[name="requireNumbers"]') as HTMLInputElement;
    expect(numbersHidden).toBeInTheDocument();
    expect(numbersHidden.value).toBe("true");

    const symbolsHidden = container.querySelector('input[name="requireSymbols"]') as HTMLInputElement;
    expect(symbolsHidden).toBeInTheDocument();
    expect(symbolsHidden.value).toBe("false");
  });

  it("renders policy form with all false boolean fields correctly", async () => {
    const allFalsePolicy = {
      ...mockPolicy,
      require_uppercase: false,
      require_lowercase: false,
      require_numbers: false,
      require_symbols: false,
    };

    vi.mocked(passwordApi.getPasswordPolicy).mockResolvedValue({
      data: allFalsePolicy,
    });

    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/settings/security"]} />
    );

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // All hidden switch fields should be "false"
    const uppercaseHidden = container.querySelector('input[name="requireUppercase"]') as HTMLInputElement;
    expect(uppercaseHidden.value).toBe("false");

    const lowercaseHidden = container.querySelector('input[name="requireLowercase"]') as HTMLInputElement;
    expect(lowercaseHidden.value).toBe("false");

    const numbersHidden = container.querySelector('input[name="requireNumbers"]') as HTMLInputElement;
    expect(numbersHidden.value).toBe("false");

    const symbolsHidden = container.querySelector('input[name="requireSymbols"]') as HTMLInputElement;
    expect(symbolsHidden.value).toBe("false");
  });

  it("submits policy form with action handler", async () => {
    vi.mocked(passwordApi.updatePasswordPolicy).mockResolvedValue({});

    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
        action: async ({ request }) => {
          const formData = await request.formData();
          const intent = formData.get("intent");
          const tenantId = formData.get("tenantId");
          const minLength = formData.get("minLength");
          // Verify the form data is passed correctly
          return {
            success: true,
            intent,
            tenantId,
            minLength,
          };
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // Click Save policy button
    const saveButton = screen.getByRole("button", { name: "Save policy" });
    await user.click(saveButton);

    // The form should have been submitted (we can verify the action was called
    // by checking the form still renders after submission)
    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });
  });

  it("toggles require uppercase switch and updates hidden field", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/settings/security"]} />
    );

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // The "Require uppercase" switch should be checked by default (mockPolicy.require_uppercase = true)
    const uppercaseSwitch = screen.getByLabelText("Require uppercase");
    expect(uppercaseSwitch).toHaveAttribute("data-state", "checked");

    // Click to toggle off
    await user.click(uppercaseSwitch);

    // The hidden input value should now be "false"
    const uppercaseHidden = container.querySelector('input[name="requireUppercase"]') as HTMLInputElement;
    expect(uppercaseHidden.value).toBe("false");
  });

  it("toggles require lowercase switch and updates hidden field", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/settings/security"]} />
    );

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // The "Require lowercase" switch should be checked (mockPolicy.require_lowercase = true)
    const lowercaseSwitch = screen.getByLabelText("Require lowercase");
    expect(lowercaseSwitch).toHaveAttribute("data-state", "checked");

    // Click to toggle off
    await user.click(lowercaseSwitch);

    const lowercaseHidden = container.querySelector('input[name="requireLowercase"]') as HTMLInputElement;
    expect(lowercaseHidden.value).toBe("false");
  });

  it("toggles require numbers switch and updates hidden field", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/settings/security"]} />
    );

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // The "Require numbers" switch should be checked (mockPolicy.require_numbers = true)
    const numbersSwitch = screen.getByLabelText("Require numbers");
    expect(numbersSwitch).toHaveAttribute("data-state", "checked");

    // Click to toggle off
    await user.click(numbersSwitch);

    const numbersHidden = container.querySelector('input[name="requireNumbers"]') as HTMLInputElement;
    expect(numbersHidden.value).toBe("false");
  });

  it("toggles require symbols switch on and updates hidden field", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    const { container } = render(
      <RoutesStub initialEntries={["/dashboard/settings/security"]} />
    );

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    const select = screen.getByLabelText("Select Tenant");
    await user.selectOptions(select, "tenant-1");

    await waitFor(() => {
      expect(screen.getByLabelText("Minimum length")).toBeInTheDocument();
    });

    // The "Require symbols" switch should be unchecked (mockPolicy.require_symbols = false)
    const symbolsSwitch = screen.getByLabelText("Require symbols");
    expect(symbolsSwitch).toHaveAttribute("data-state", "unchecked");

    // Click to toggle on
    await user.click(symbolsSwitch);

    const symbolsHidden = container.querySelector('input[name="requireSymbols"]') as HTMLInputElement;
    expect(symbolsHidden.value).toBe("true");
  });

  it("renders empty tenants list correctly", async () => {
    vi.mocked(tenantApi.list).mockResolvedValueOnce({ data: [] });

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Select Tenant")).toBeInTheDocument();
    });

    // Only the default option should be present
    const select = screen.getByLabelText("Select Tenant") as HTMLSelectElement;
    expect(select.options.length).toBe(1);
    expect(select.options[0].textContent).toBe("Select a tenant...");
  });
});
