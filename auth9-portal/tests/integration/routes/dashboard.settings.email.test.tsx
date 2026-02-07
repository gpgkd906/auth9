import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import EmailSettingsPage, { loader, action, meta } from "~/routes/dashboard.settings.email";
import { systemApi } from "~/services/api";
import type { EmailProviderConfig } from "~/services/api";

// Polyfill pointer capture methods for Radix UI Select in happy-dom
if (typeof window !== "undefined") {
  if (!Element.prototype.hasPointerCapture) {
    Element.prototype.hasPointerCapture = () => false;
  }
  if (!Element.prototype.setPointerCapture) {
    Element.prototype.setPointerCapture = () => {};
  }
  if (!Element.prototype.releasePointerCapture) {
    Element.prototype.releasePointerCapture = () => {};
  }
}

// Mock system API
vi.mock("~/services/api", () => ({
  systemApi: {
    getEmailSettings: vi.fn(),
    updateEmailSettings: vi.fn(),
    testEmailConnection: vi.fn(),
    sendTestEmail: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

describe("Email Settings Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("meta returns correct page title", () => {
    const result = meta({} as Parameters<typeof meta>[0]);
    expect(result).toEqual([{ title: "Email Settings - Auth9" }]);
  });

  // Mock data matches the SystemSettingResponse structure from backend
  const mockSmtpConfig = {
    data: {
      category: "email",
      setting_key: "provider",
      value: {
        type: "smtp" as const,
        host: "smtp.example.com",
        port: 587,
        username: "user@example.com",
        password: "***",
        use_tls: true,
        from_email: "noreply@example.com",
        from_name: "Auth9",
      },
      description: "Email provider configuration",
      updated_at: new Date().toISOString(),
    },
  };

  const mockNoneConfig = {
    data: {
      category: "email",
      setting_key: "provider",
      value: { type: "none" as const },
      description: "Email provider configuration",
      updated_at: new Date().toISOString(),
    },
  };

  it("renders email settings page with provider selection", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Configuration")).toBeInTheDocument();
      expect(screen.getByText("Provider Type")).toBeInTheDocument();
    });
  });

  it("shows SMTP configuration fields when SMTP is selected", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("SMTP Configuration")).toBeInTheDocument();
      expect(screen.getByLabelText("Server Host")).toBeInTheDocument();
      expect(screen.getByLabelText("Port")).toBeInTheDocument();
      expect(screen.getByLabelText("Username")).toBeInTheDocument();
      expect(screen.getByLabelText("Password")).toBeInTheDocument();
      expect(screen.getByLabelText("From Email")).toBeInTheDocument();
      expect(screen.getByLabelText("From Name")).toBeInTheDocument();
      expect(screen.getByText("Use TLS encryption")).toBeInTheDocument();
    });
  });

  it("loads existing SMTP configuration values", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("smtp.example.com")).toBeInTheDocument();
      expect(screen.getByDisplayValue("587")).toBeInTheDocument();
      expect(screen.getByDisplayValue("user@example.com")).toBeInTheDocument();
      expect(screen.getByDisplayValue("noreply@example.com")).toBeInTheDocument();
      expect(screen.getByDisplayValue("Auth9")).toBeInTheDocument();
    });
  });

  it("shows action buttons when provider is not none", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Settings")).toBeInTheDocument();
      expect(screen.getByText("Test Connection")).toBeInTheDocument();
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });
  });

  it("hides test buttons when provider is none", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Settings")).toBeInTheDocument();
      expect(screen.queryByText("Test Connection")).not.toBeInTheDocument();
      expect(screen.queryByText("Send Test Email")).not.toBeInTheDocument();
    });
  });

  it("has test email button when provider is configured", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      // Verify test email button is rendered
      const testButton = screen.getByText("Send Test Email");
      expect(testButton).toBeInTheDocument();
      expect(testButton.closest("button")).toBeInTheDocument();
    });
  });

  it("shows AWS SES configuration fields when SES is selected", async () => {
    const sesConfig = {
      data: {
        category: "email",
        setting_key: "provider",
        value: {
          type: "ses" as const,
          region: "us-east-1",
          access_key_id: "AKIA***",
          secret_access_key: "***",
          from_email: "noreply@example.com",
          from_name: "Auth9",
        },
        description: "Email provider configuration",
        updated_at: new Date().toISOString(),
      },
    };

    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(sesConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("AWS SES Configuration")).toBeInTheDocument();
      expect(screen.getByLabelText("AWS Region")).toBeInTheDocument();
      expect(screen.getByLabelText("Access Key ID")).toBeInTheDocument();
      expect(screen.getByLabelText("Secret Access Key")).toBeInTheDocument();
      expect(screen.getByLabelText("Configuration Set")).toBeInTheDocument();
    });
  });

  it("shows Oracle configuration fields when Oracle is selected", async () => {
    const oracleConfig = {
      data: {
        category: "email",
        setting_key: "provider",
        value: {
          type: "oracle" as const,
          smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com",
          port: 587,
          username: "ocid1.user",
          password: "***",
          from_email: "noreply@example.com",
          from_name: "Auth9",
        },
        description: "Email provider configuration",
        updated_at: new Date().toISOString(),
      },
    };

    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(oracleConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Oracle Email Delivery Configuration")).toBeInTheDocument();
      expect(screen.getByLabelText("SMTP Endpoint")).toBeInTheDocument();
      expect(screen.getByLabelText("SMTP Username")).toBeInTheDocument();
      expect(screen.getByLabelText("SMTP Password")).toBeInTheDocument();
    });
  });

  it("handles API error gracefully", async () => {
    vi.mocked(systemApi.getEmailSettings).mockRejectedValue(new Error("API Error"));

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    // Should fallback to "none" config
    await waitFor(() => {
      expect(screen.getByText("Email Provider Configuration")).toBeInTheDocument();
    });
  });

  it("shows status banner for configured provider", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Active")).toBeInTheDocument();
      expect(screen.getByText(/smtp.example.com:587/)).toBeInTheDocument();
    });
  });

  it("shows unconfigured status banner when no provider", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Not Configured")).toBeInTheDocument();
    });
  });

  it("opens test email dialog when clicking Send Test Email button", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Send Test Email"));

    await waitFor(() => {
      expect(screen.getByText("Enter an email address to receive a test email and verify your configuration.")).toBeInTheDocument();
      expect(screen.getByLabelText("Email Address")).toBeInTheDocument();
    });
  });

  it("has provider type select with correct options", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Provider Type")).toBeInTheDocument();
      // Verify select trigger exists
      expect(screen.getByRole("combobox")).toBeInTheDocument();
    });
  });

  it("shows info banner about single provider configuration", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Single Provider Configuration")).toBeInTheDocument();
    });
  });

  it("closes test email dialog when clicking Cancel", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    await user.click(screen.getByText("Send Test Email"));

    await waitFor(() => {
      expect(screen.getByLabelText("Email Address")).toBeInTheDocument();
    });

    // Click Cancel button in dialog
    const cancelButton = screen.getByRole("button", { name: "Cancel" });
    await user.click(cancelButton);

    await waitFor(() => {
      expect(screen.queryByLabelText("Email Address")).not.toBeInTheDocument();
    });
  });

  it("displays password hint when SMTP password exists", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Leave blank to keep existing password")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  describe("action", () => {
    it("saves SMTP configuration successfully", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockResolvedValue({
        data: {
          category: "email",
          setting_key: "provider",
          value: { type: "smtp", host: "smtp.example.com", port: 587, use_tls: true, from_email: "noreply@example.com" } as EmailProviderConfig,
          updated_at: "2025-01-01T00:00:00Z",
        },
      });

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "smtp");
      formData.append("host", "smtp.example.com");
      formData.append("port", "587");
      formData.append("username", "user@example.com");
      formData.append("password", "secret");
      formData.append("use_tls", "on");
      formData.append("from_email", "noreply@example.com");
      formData.append("from_name", "Auth9");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Email settings saved successfully",
      });
      expect(systemApi.updateEmailSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "smtp",
          host: "smtp.example.com",
          port: 587,
          use_tls: true,
          from_email: "noreply@example.com",
          from_name: "Auth9",
        }),
        "test-token"
      );
    });

    it("saves none configuration", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockResolvedValue({
        data: {
          category: "email",
          setting_key: "provider",
          value: { type: "none" } as EmailProviderConfig,
          updated_at: "2025-01-01T00:00:00Z",
        },
      });

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "none");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Email settings saved successfully",
      });
      expect(systemApi.updateEmailSettings).toHaveBeenCalledWith(
        { type: "none" },
        "test-token"
      );
    });

    it("saves AWS SES configuration", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockResolvedValue({
        data: {
          category: "email",
          setting_key: "provider",
          value: { type: "ses", region: "us-east-1", from_email: "a@b.com" } as EmailProviderConfig,
          updated_at: "2025-01-01T00:00:00Z",
        },
      });

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "ses");
      formData.append("region", "us-east-1");
      formData.append("access_key_id", "AKIAEXAMPLE");
      formData.append("secret_access_key", "secret-key");
      formData.append("from_email", "a@b.com");
      formData.append("from_name", "Auth9");
      formData.append("configuration_set", "my-set");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Email settings saved successfully",
      });
      expect(systemApi.updateEmailSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "ses",
          region: "us-east-1",
          access_key_id: "AKIAEXAMPLE",
          secret_access_key: "secret-key",
          from_email: "a@b.com",
          from_name: "Auth9",
          configuration_set: "my-set",
        }),
        "test-token"
      );
    });

    it("saves Oracle Email Delivery configuration", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockResolvedValue({
        data: {
          category: "email",
          setting_key: "provider",
          value: { type: "oracle", smtp_endpoint: "smtp.oracle.com", port: 587, username: "u", password: "p", from_email: "a@b.com" } as EmailProviderConfig,
          updated_at: "2025-01-01T00:00:00Z",
        },
      });

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "oracle");
      formData.append("smtp_endpoint", "smtp.oracle.com");
      formData.append("port", "587");
      formData.append("username", "ocid1.user");
      formData.append("password", "oracle-pass");
      formData.append("from_email", "a@b.com");
      formData.append("from_name", "Auth9");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Email settings saved successfully",
      });
      expect(systemApi.updateEmailSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "oracle",
          smtp_endpoint: "smtp.oracle.com",
          port: 587,
          username: "ocid1.user",
          password: "oracle-pass",
          from_email: "a@b.com",
          from_name: "Auth9",
        }),
        "test-token"
      );
    });

    it("returns error for invalid provider type", async () => {
      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "invalid_provider");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Invalid provider type" });
    });

    it("tests email connection successfully", async () => {
      vi.mocked(systemApi.testEmailConnection).mockResolvedValue({
        success: true,
        message: "Connection OK",
      });

      const formData = new FormData();
      formData.append("intent", "test_connection");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Connection test successful",
      });
      expect(systemApi.testEmailConnection).toHaveBeenCalledWith("test-token");
    });

    it("returns error when test connection fails", async () => {
      vi.mocked(systemApi.testEmailConnection).mockResolvedValue({
        success: false,
        message: "Connection refused",
      });

      const formData = new FormData();
      formData.append("intent", "test_connection");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Connection refused" });
    });

    it("sends test email successfully", async () => {
      vi.mocked(systemApi.sendTestEmail).mockResolvedValue({
        success: true,
        message: "Email sent",
      });

      const formData = new FormData();
      formData.append("intent", "send_test");
      formData.append("test_email", "test@example.com");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Test email sent to test@example.com",
      });
      expect(systemApi.sendTestEmail).toHaveBeenCalledWith(
        "test@example.com",
        "test-token"
      );
    });

    it("returns error for invalid test email address", async () => {
      const formData = new FormData();
      formData.append("intent", "send_test");
      formData.append("test_email", "not-an-email");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Please enter a valid email address" });
    });

    it("returns error when send test email fails", async () => {
      vi.mocked(systemApi.sendTestEmail).mockResolvedValue({
        success: false,
        message: "Mailbox not found",
      });

      const formData = new FormData();
      formData.append("intent", "send_test");
      formData.append("test_email", "test@example.com");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Mailbox not found" });
    });

    it("returns error for invalid intent", async () => {
      const formData = new FormData();
      formData.append("intent", "invalid");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Invalid intent" });
    });

    it("handles API error with Error instance", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockRejectedValue(
        new Error("Server error")
      );

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "none");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Server error" });
    });

    it("handles API error with non-Error exception", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockRejectedValue("string error");

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "none");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Unknown error" });
    });

    it("saves SMTP config with use_tls off when checkbox not checked", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockResolvedValue({
        data: {
          category: "email",
          setting_key: "provider",
          value: { type: "smtp", host: "h", port: 587, use_tls: false, from_email: "a@b.com" } as EmailProviderConfig,
          updated_at: "2025-01-01T00:00:00Z",
        },
      });

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "smtp");
      formData.append("host", "smtp.example.com");
      formData.append("port", "587");
      formData.append("from_email", "noreply@example.com");
      // No use_tls field means it's not checked

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Email settings saved successfully",
      });
      expect(systemApi.updateEmailSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "smtp",
          use_tls: false,
          username: undefined,
          password: undefined,
          from_name: undefined,
        }),
        "test-token"
      );
    });

    it("saves SES config with optional fields empty", async () => {
      vi.mocked(systemApi.updateEmailSettings).mockResolvedValue({
        data: {
          category: "email",
          setting_key: "provider",
          value: { type: "ses", region: "us-west-2", from_email: "a@b.com" } as EmailProviderConfig,
          updated_at: "2025-01-01T00:00:00Z",
        },
      });

      const formData = new FormData();
      formData.append("intent", "save");
      formData.append("provider_type", "ses");
      formData.append("region", "us-west-2");
      formData.append("access_key_id", "");
      formData.append("secret_access_key", "");
      formData.append("from_email", "a@b.com");
      formData.append("from_name", "");
      formData.append("configuration_set", "");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toEqual({
        success: true,
        message: "Email settings saved successfully",
      });
      expect(systemApi.updateEmailSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          type: "ses",
          access_key_id: undefined,
          secret_access_key: undefined,
          from_name: undefined,
          configuration_set: undefined,
        }),
        "test-token"
      );
    });

    it("returns error for empty test email", async () => {
      const formData = new FormData();
      formData.append("intent", "send_test");
      formData.append("test_email", "");

      const request = new Request("http://localhost/dashboard/settings/email", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const json = await (response as Response).json();
      expect(json).toEqual({ error: "Please enter a valid email address" });
    });
  });

  // ============================================================================
  // Component Interaction Tests
  // ============================================================================

  describe("component interactions", () => {
    it("switches from none to SMTP and shows SMTP fields", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      // Open the select dropdown
      await user.click(screen.getByRole("combobox"));

      // Select SMTP
      const smtpOption = await screen.findByRole("option", { name: "SMTP" });
      await user.click(smtpOption);

      // SMTP fields should appear
      await waitFor(() => {
        expect(screen.getByText("SMTP Configuration")).toBeInTheDocument();
      });
      expect(screen.getByLabelText("Server Host")).toBeInTheDocument();
      expect(screen.getByLabelText("Port")).toBeInTheDocument();
    });

    it("switches from none to SES and shows SES fields", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("combobox"));
      const sesOption = await screen.findByRole("option", { name: "AWS SES" });
      await user.click(sesOption);

      await waitFor(() => {
        expect(screen.getByText("AWS SES Configuration")).toBeInTheDocument();
      });
      expect(screen.getByLabelText("AWS Region")).toBeInTheDocument();
    });

    it("switches from none to Oracle and shows Oracle fields", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("combobox"));
      const oracleOption = await screen.findByRole("option", { name: "Oracle Email Delivery" });
      await user.click(oracleOption);

      await waitFor(() => {
        expect(screen.getByText("Oracle Email Delivery Configuration")).toBeInTheDocument();
      });
      expect(screen.getByLabelText("SMTP Endpoint")).toBeInTheDocument();
    });

    it("shows Test Connection and Send Test Email buttons when a provider is selected", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      // Initially no test buttons
      expect(screen.queryByText("Test Connection")).not.toBeInTheDocument();

      await user.click(screen.getByRole("combobox"));
      const smtpOption = await screen.findByRole("option", { name: "SMTP" });
      await user.click(smtpOption);

      // Now test buttons should appear
      await waitFor(() => {
        expect(screen.getByText("Test Connection")).toBeInTheDocument();
        expect(screen.getByText("Send Test Email")).toBeInTheDocument();
      });
    });

    it("shows provider switch confirmation when switching between configured providers", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      // Switch from SMTP to SES (both non-none, both configured)
      await user.click(screen.getByRole("combobox"));
      const sesOption = await screen.findByRole("option", { name: "AWS SES" });
      await user.click(sesOption);

      // Should show confirmation dialog
      await waitFor(() => {
        expect(screen.getByText("Switch Email Provider?")).toBeInTheDocument();
      });
    });

    it("confirms provider switch and shows new provider fields", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("combobox"));
      const sesOption = await screen.findByRole("option", { name: "AWS SES" });
      await user.click(sesOption);

      await waitFor(() => {
        expect(screen.getByText("Switch Email Provider?")).toBeInTheDocument();
      });

      // Click Switch Provider to confirm
      await user.click(screen.getByRole("button", { name: "Switch Provider" }));

      // After confirming, SES fields should appear
      await waitFor(() => {
        expect(screen.getByText("AWS SES Configuration")).toBeInTheDocument();
      });
    });

    it("cancels provider switch and keeps current provider", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("combobox"));
      const sesOption = await screen.findByRole("option", { name: "AWS SES" });
      await user.click(sesOption);

      await waitFor(() => {
        expect(screen.getByText("Switch Email Provider?")).toBeInTheDocument();
      });

      // Click Cancel
      await user.click(screen.getByRole("button", { name: "Cancel" }));

      // Dialog should close and SMTP fields should still be visible
      await waitFor(() => {
        expect(screen.queryByText("Switch Email Provider?")).not.toBeInTheDocument();
      });
      expect(screen.getByText("SMTP Configuration")).toBeInTheDocument();
    });

    it("does not show switch confirmation when going from configured to none", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("combobox"));
      const noneOption = await screen.findByRole("option", { name: "None (Email disabled)" });
      await user.click(noneOption);

      // Should NOT show confirmation dialog; should switch directly
      await waitFor(() => {
        expect(screen.queryByText("Switch Email Provider?")).not.toBeInTheDocument();
      });
    });

    it("shows replacement warning after confirming provider switch", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("combobox")).toBeInTheDocument();
      });

      // Switch from SMTP to Oracle
      await user.click(screen.getByRole("combobox"));
      const oracleOption = await screen.findByRole("option", { name: "Oracle Email Delivery" });
      await user.click(oracleOption);

      await waitFor(() => {
        expect(screen.getByText("Switch Email Provider?")).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Switch Provider" }));

      // After switching, the warning text about replacing config should appear
      await waitFor(() => {
        expect(screen.getByText(/Saving will replace your current/)).toBeInTheDocument();
      });
    });

    it("opens and interacts with the Send Test Email dialog", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByText("Send Test Email")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Send Test Email"));

      await waitFor(() => {
        expect(screen.getByLabelText("Email Address")).toBeInTheDocument();
      });

      // Type an email address
      const emailInput = screen.getByLabelText("Email Address");
      await user.type(emailInput, "test@example.com");
      expect(emailInput).toHaveValue("test@example.com");
    });

    it("disables Send Test Email dialog button when email has no @", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByText("Send Test Email")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Send Test Email"));

      await waitFor(() => {
        expect(screen.getByLabelText("Email Address")).toBeInTheDocument();
      });

      // The dialog's Send Test Email button should be disabled initially (empty email)
      const dialogButtons = screen.getAllByRole("button", { name: "Send Test Email" });
      const dialogSendButton = dialogButtons[dialogButtons.length - 1];
      expect(dialogSendButton).toBeDisabled();

      // Type an invalid email (no @)
      await user.type(screen.getByLabelText("Email Address"), "nope");
      expect(dialogSendButton).toBeDisabled();

      // Clear and type a valid email
      await user.clear(screen.getByLabelText("Email Address"));
      await user.type(screen.getByLabelText("Email Address"), "valid@email.com");
      expect(dialogSendButton).not.toBeDisabled();
    });

    it("clicking Test Connection triggers submit with test_connection intent", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();
      let capturedFormData: Record<string, string> = {};

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
          action: async ({ request }) => {
            const formData = await request.formData();
            capturedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
            return { success: true, message: "Connection test successful" };
          },
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByText("Test Connection")).toBeInTheDocument();
      });

      await user.click(screen.getByText("Test Connection"));

      await waitFor(() => {
        expect(capturedFormData.intent).toBe("test_connection");
      });
    });

    it("clicking Send Test Email in dialog triggers submit with send_test intent", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();
      let capturedFormData: Record<string, string> = {};

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
          action: async ({ request }) => {
            const formData = await request.formData();
            capturedFormData = Object.fromEntries(formData.entries()) as Record<string, string>;
            return { success: true, message: "Test email sent to test@example.com" };
          },
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByText("Send Test Email")).toBeInTheDocument();
      });

      // Open the dialog
      await user.click(screen.getByText("Send Test Email"));

      await waitFor(() => {
        expect(screen.getByLabelText("Email Address")).toBeInTheDocument();
      });

      // Type a valid email
      await user.type(screen.getByLabelText("Email Address"), "test@example.com");

      // Click the dialog's Send Test Email button
      const dialogButtons = screen.getAllByRole("button", { name: "Send Test Email" });
      const dialogSendButton = dialogButtons[dialogButtons.length - 1];
      await user.click(dialogSendButton);

      await waitFor(() => {
        expect(capturedFormData.intent).toBe("send_test");
        expect(capturedFormData.test_email).toBe("test@example.com");
      });
    });
  });

  // ============================================================================
  // Action Data Display Tests
  // ============================================================================

  describe("action data display", () => {
    it("renders success message from action data", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
          action: () => ({
            success: true,
            message: "Email settings saved successfully",
          }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: "Save Settings" })).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Save Settings" }));

      await waitFor(() => {
        expect(screen.getByText("Email settings saved successfully")).toBeInTheDocument();
      });
    });

    it("renders error message from action data", async () => {
      vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
      const user = userEvent.setup();

      const RoutesStub = createRoutesStub([
        {
          path: "/dashboard/settings/email",
          Component: EmailSettingsPage,
          loader,
          action: () => ({ error: "Something went wrong" }),
        },
      ]);

      render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

      await waitFor(() => {
        expect(screen.getByRole("button", { name: "Save Settings" })).toBeInTheDocument();
      });

      await user.click(screen.getByRole("button", { name: "Save Settings" }));

      await waitFor(() => {
        expect(screen.getByText("Something went wrong")).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Oracle Password Hint Test
  // ============================================================================

  it("shows password hint when Oracle password exists", async () => {
    const oracleConfig = {
      data: {
        category: "email",
        setting_key: "provider",
        value: {
          type: "oracle" as const,
          smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com",
          port: 587,
          username: "ocid1.user",
          password: "***",
          from_email: "noreply@example.com",
          from_name: "Auth9",
        },
        description: "Email provider configuration",
        updated_at: new Date().toISOString(),
      },
    };

    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(oracleConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Leave blank to keep existing password")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // SES Provider Info Display Test
  // ============================================================================

  it("shows AWS SES provider status with region info", async () => {
    const sesConfig = {
      data: {
        category: "email",
        setting_key: "provider",
        value: {
          type: "ses" as const,
          region: "us-east-1",
          access_key_id: "AKIA***",
          secret_access_key: "***",
          from_email: "noreply@example.com",
          from_name: "Auth9",
        },
        description: "Email provider configuration",
        updated_at: new Date().toISOString(),
      },
    };

    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(sesConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Active")).toBeInTheDocument();
    });
    expect(screen.getByText(/Region: us-east-1/)).toBeInTheDocument();
  });

  // ============================================================================
  // Oracle Provider Info Display Test
  // ============================================================================

  it("shows Oracle provider status with endpoint info", async () => {
    const oracleConfig = {
      data: {
        category: "email",
        setting_key: "provider",
        value: {
          type: "oracle" as const,
          smtp_endpoint: "smtp.us-ashburn-1.oraclecloud.com",
          port: 587,
          username: "ocid1.user",
          password: "***",
          from_email: "noreply@example.com",
          from_name: "Auth9",
        },
        description: "Email provider configuration",
        updated_at: new Date().toISOString(),
      },
    };

    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(oracleConfig);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Active")).toBeInTheDocument();
    });
    expect(screen.getByText(/smtp.us-ashburn-1.oraclecloud.com/)).toBeInTheDocument();
  });
});
