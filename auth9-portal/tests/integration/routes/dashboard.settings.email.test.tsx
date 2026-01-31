import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import EmailSettingsPage, { loader } from "~/routes/dashboard.settings.email";
import { systemApi } from "~/services/api";

// Mock system API
vi.mock("~/services/api", () => ({
  systemApi: {
    getEmailSettings: vi.fn(),
    updateEmailSettings: vi.fn(),
    testEmailConnection: vi.fn(),
    sendTestEmail: vi.fn(),
  },
}));

describe("Email Settings Page", () => {
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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Configuration")).toBeInTheDocument();
      expect(screen.getByText("Provider Type")).toBeInTheDocument();
    });
  });

  it("shows SMTP configuration fields when SMTP is selected", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Settings")).toBeInTheDocument();
      expect(screen.getByText("Test Connection")).toBeInTheDocument();
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });
  });

  it("hides test buttons when provider is none", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Settings")).toBeInTheDocument();
      expect(screen.queryByText("Test Connection")).not.toBeInTheDocument();
      expect(screen.queryByText("Send Test Email")).not.toBeInTheDocument();
    });
  });

  it("has test email button when provider is configured", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Oracle Email Delivery Configuration")).toBeInTheDocument();
      expect(screen.getByLabelText("SMTP Endpoint")).toBeInTheDocument();
      expect(screen.getByLabelText("SMTP Username")).toBeInTheDocument();
      expect(screen.getByLabelText("SMTP Password")).toBeInTheDocument();
    });
  });

  it("handles API error gracefully", async () => {
    vi.mocked(systemApi.getEmailSettings).mockRejectedValue(new Error("API Error"));

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    // Should fallback to "none" config
    await waitFor(() => {
      expect(screen.getByText("Email Provider Configuration")).toBeInTheDocument();
    });
  });

  it("shows status banner for configured provider", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Active")).toBeInTheDocument();
      expect(screen.getByText(/smtp.example.com:587/)).toBeInTheDocument();
    });
  });

  it("shows unconfigured status banner when no provider", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Provider Not Configured")).toBeInTheDocument();
    });
  });

  it("opens test email dialog when clicking Send Test Email button", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
    const user = userEvent.setup();

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Provider Type")).toBeInTheDocument();
      // Verify select trigger exists
      expect(screen.getByRole("combobox")).toBeInTheDocument();
    });
  });

  it("shows info banner about single provider configuration", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockNoneConfig);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Single Provider Configuration")).toBeInTheDocument();
    });
  });

  it("closes test email dialog when clicking Cancel", async () => {
    vi.mocked(systemApi.getEmailSettings).mockResolvedValue(mockSmtpConfig);
    const user = userEvent.setup();

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

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

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email",
        Component: EmailSettingsPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email"]} />);

    await waitFor(() => {
      expect(screen.getByText("Leave blank to keep existing password")).toBeInTheDocument();
    });
  });
});
