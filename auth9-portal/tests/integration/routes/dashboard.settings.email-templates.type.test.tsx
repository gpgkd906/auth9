import { createRemixStub } from "@remix-run/testing";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import EmailTemplateEditorPage, { loader, action } from "~/routes/dashboard.settings.email-templates.$type";
import { emailTemplateApi } from "~/services/api";

// Mock email template API
vi.mock("~/services/api", () => ({
  emailTemplateApi: {
    list: vi.fn(),
    get: vi.fn(),
    update: vi.fn(),
    reset: vi.fn(),
    preview: vi.fn(),
  },
}));

describe("Email Template Editor Page", () => {
  const mockTemplate = {
    data: {
      metadata: {
        template_type: "invitation" as const,
        name: "User Invitation",
        description: "Sent when inviting users to join a tenant",
        variables: [
          { name: "inviter_name", description: "Name of the person sending invitation", example: "John Doe" },
          { name: "tenant_name", description: "Organization name", example: "Acme Corp" },
          { name: "app_name", description: "Application name", example: "Auth9" },
          { name: "year", description: "Current year", example: "2026" },
        ],
      },
      content: {
        subject: "You've been invited to join {{tenant_name}}",
        html_body: "<html><body>Hello {{inviter_name}}</body></html>",
        text_body: "You're Invited!",
      },
      is_customized: true,
      updated_at: "2026-01-31T10:00:00Z",
    },
  };

  const mockDefaultTemplate = {
    data: {
      ...mockTemplate.data,
      is_customized: false,
      updated_at: undefined,
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders template editor with header and back button", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("User Invitation")).toBeInTheDocument();
      expect(screen.getByText("Sent when inviting users to join a tenant")).toBeInTheDocument();
      expect(screen.getByText("Back")).toBeInTheDocument();
    });
  });

  it("loads template content into form fields", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("You've been invited to join {{tenant_name}}")).toBeInTheDocument();
      expect(screen.getByDisplayValue("<html><body>Hello {{inviter_name}}</body></html>")).toBeInTheDocument();
      expect(screen.getByDisplayValue("You're Invited!")).toBeInTheDocument();
    });
  });

  it("displays available variables sidebar", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Available Variables")).toBeInTheDocument();
      expect(screen.getByText("{{inviter_name}}")).toBeInTheDocument();
      expect(screen.getByText("{{tenant_name}}")).toBeInTheDocument();
      expect(screen.getByText("{{app_name}}")).toBeInTheDocument();
      expect(screen.getByText("Name of the person sending invitation")).toBeInTheDocument();
    });
  });

  it("shows Reset to Default button for customized template", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Reset to Default")).toBeInTheDocument();
    });
  });

  it("hides Reset to Default button for default template", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockDefaultTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("User Invitation")).toBeInTheDocument();
    });

    expect(screen.queryByText("Reset to Default")).not.toBeInTheDocument();
  });

  it("renders Save Template and Preview buttons", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Template")).toBeInTheDocument();
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });
  });

  it("renders form labels correctly", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Subject Line")).toBeInTheDocument();
      expect(screen.getByText("HTML Body")).toBeInTheDocument();
      expect(screen.getByText("Plain Text Body")).toBeInTheDocument();
    });
  });

  it("allows editing template subject", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("You've been invited to join {{tenant_name}}")).toBeInTheDocument();
    });

    const subjectInput = screen.getByLabelText("Subject Line");
    fireEvent.change(subjectInput, { target: { value: "New Subject Line" } });

    expect(screen.getByDisplayValue("New Subject Line")).toBeInTheDocument();
  });

  it("displays help text for plain text body", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText(/Shown to recipients whose email clients don't support HTML/)).toBeInTheDocument();
    });
  });

  it("displays template content card with description", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Template Content")).toBeInTheDocument();
      expect(screen.getByText(/Edit the subject line and body content/)).toBeInTheDocument();
    });
  });

  it("saves template successfully", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.update).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Template")).toBeInTheDocument();
    });

    const saveButton = screen.getByText("Save Template");
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(screen.getByText("Template saved successfully")).toBeInTheDocument();
    });
  });

  it("displays error message on save failure", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.update).mockRejectedValue(new Error("Save failed"));

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Template")).toBeInTheDocument();
    });

    const saveButton = screen.getByText("Save Template");
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(screen.getByText("Save failed")).toBeInTheDocument();
    });
  });

  it("opens reset confirmation dialog", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Reset to Default")).toBeInTheDocument();
    });

    const resetButton = screen.getByText("Reset to Default");
    fireEvent.click(resetButton);

    await waitFor(() => {
      expect(screen.getByText("Reset Template?")).toBeInTheDocument();
      expect(screen.getByText(/This will restore the default template content/)).toBeInTheDocument();
    });
  });

  it("shows preview when preview button is clicked", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.preview).mockResolvedValue({
      data: {
        subject: "You've been invited to join Acme Corp",
        html_body: "<html><body>Hello John Doe</body></html>",
        text_body: "Hello John Doe, You're Invited!",
      },
    });

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });

    const previewButton = screen.getByText("Preview");
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText("You've been invited to join Acme Corp")).toBeInTheDocument();
    });
  });

  it("displays preview tabs after preview is loaded", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.preview).mockResolvedValue({
      data: {
        subject: "Test Subject",
        html_body: "<html><body>HTML Content</body></html>",
        text_body: "Plain text content",
      },
    });

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });

    // Click preview button
    const previewButton = screen.getByText("Preview");
    fireEvent.click(previewButton);

    // After preview action, the preview card with tabs should appear
    await waitFor(() => {
      expect(screen.getByText("Test Subject")).toBeInTheDocument();
    });

    // Verify both tabs exist
    expect(screen.getByRole("tab", { name: "HTML" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Text" })).toBeInTheDocument();
  });

  it("handles password_reset template type", async () => {
    const passwordResetTemplate = {
      data: {
        metadata: {
          template_type: "password_reset" as const,
          name: "Password Reset",
          description: "Sent when a user requests to reset their password",
          variables: [
            { name: "user_name", description: "Name of the user", example: "Jane Smith" },
            { name: "reset_link", description: "Password reset URL", example: "https://example.com/reset" },
          ],
        },
        content: {
          subject: "Reset your password",
          html_body: "<html>Reset password</html>",
          text_body: "Reset your password",
        },
        is_customized: false,
        updated_at: undefined,
      },
    };

    vi.mocked(emailTemplateApi.get).mockResolvedValue(passwordResetTemplate);

    const RemixStub = createRemixStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RemixStub initialEntries={["/dashboard/settings/email-templates/password_reset"]} />);

    await waitFor(() => {
      expect(screen.getByText("Password Reset")).toBeInTheDocument();
      expect(screen.getByText("{{user_name}}")).toBeInTheDocument();
      expect(screen.getByText("{{reset_link}}")).toBeInTheDocument();
    });
  });
});
