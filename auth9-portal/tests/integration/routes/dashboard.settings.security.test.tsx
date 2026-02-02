import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import SecuritySettingsPage, {
  loader,
  action,
} from "~/routes/dashboard.settings.security";
import { passwordApi, tenantApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  passwordApi: {
    changePassword: vi.fn(),
    getPasswordPolicy: vi.fn(),
    updatePasswordPolicy: vi.fn(),
  },
  tenantApi: {
    list: vi.fn(),
  },
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

    expect(response).toEqual({ tenants: mockTenants });
    expect(tenantApi.list).toHaveBeenCalledWith(1, 100);
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action validates password change - missing fields", async () => {
    const formData = new FormData();
    formData.append("intent", "change_password");
    formData.append("currentPassword", "");
    formData.append("newPassword", "");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "All password fields are required" });
  });

  it("action validates password change - too short", async () => {
    const formData = new FormData();
    formData.append("intent", "change_password");
    formData.append("currentPassword", "oldpass");
    formData.append("newPassword", "short");
    formData.append("confirmPassword", "short");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({
      error: "New password must be at least 8 characters",
    });
  });

  it("action validates password change - mismatch", async () => {
    const formData = new FormData();
    formData.append("intent", "change_password");
    formData.append("currentPassword", "oldpassword");
    formData.append("newPassword", "newpassword123");
    formData.append("confirmPassword", "differentpassword");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "New passwords do not match" });
  });

  it("action changes password successfully", async () => {
    vi.mocked(passwordApi.changePassword).mockResolvedValue({});

    const formData = new FormData();
    formData.append("intent", "change_password");
    formData.append("currentPassword", "oldpassword");
    formData.append("newPassword", "newpassword123");
    formData.append("confirmPassword", "newpassword123");

    const request = new Request("http://localhost/dashboard/settings/security", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({
      success: true,
      message: "Password changed successfully",
    });
    expect(passwordApi.changePassword).toHaveBeenCalled();
  });

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

  it("renders change password section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader: () => ({ tenants: mockTenants }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(screen.getByText("Change Password")).toBeInTheDocument();
    });
    expect(screen.getByLabelText("Current password")).toBeInTheDocument();
    expect(screen.getByLabelText("New password")).toBeInTheDocument();
    expect(screen.getByLabelText("Confirm new password")).toBeInTheDocument();
  });

  it("renders password policy section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader: () => ({ tenants: mockTenants }),
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
        loader: () => ({ tenants: mockTenants }),
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

  it("renders password requirements hint", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/security",
        Component: SecuritySettingsPage,
        loader: () => ({ tenants: mockTenants }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/security"]} />);

    await waitFor(() => {
      expect(
        screen.getByText("Must be at least 8 characters")
      ).toBeInTheDocument();
    });
  });
});
