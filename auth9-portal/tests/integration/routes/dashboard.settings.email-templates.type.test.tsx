import { createRoutesStub } from "react-router";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
    sendTestEmail: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue(null),
    requireAuthWithUpdate: vi.fn().mockResolvedValue({
        session: {
            accessToken: "test-token",
            refreshToken: "test-refresh-token",
            idToken: "test-id-token",
            expiresAt: Date.now() + 3600000,
        },
        headers: undefined,
    }),
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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("User Invitation")).toBeInTheDocument();
      expect(screen.getByText("Sent when inviting users to join a tenant")).toBeInTheDocument();
      expect(screen.getByText("Back")).toBeInTheDocument();
    });
  });

  it("loads template content into form fields", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("You've been invited to join {{tenant_name}}")).toBeInTheDocument();
      expect(screen.getByDisplayValue("<html><body>Hello {{inviter_name}}</body></html>")).toBeInTheDocument();
      expect(screen.getByDisplayValue("You're Invited!")).toBeInTheDocument();
    });
  });

  it("displays available variables sidebar", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Reset to Default")).toBeInTheDocument();
    });
  });

  it("hides Reset to Default button for default template", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockDefaultTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("User Invitation")).toBeInTheDocument();
    });

    expect(screen.queryByText("Reset to Default")).not.toBeInTheDocument();
  });

  it("renders Save Template and Preview buttons", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Template")).toBeInTheDocument();
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });
  });

  it("renders form labels correctly", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Subject Line")).toBeInTheDocument();
      expect(screen.getByText("HTML Body")).toBeInTheDocument();
      expect(screen.getByText("Plain Text Body")).toBeInTheDocument();
    });
  });

  it("allows editing template subject", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("You've been invited to join {{tenant_name}}")).toBeInTheDocument();
    });

    const subjectInput = screen.getByLabelText("Subject Line");
    fireEvent.change(subjectInput, { target: { value: "New Subject Line" } });

    expect(screen.getByDisplayValue("New Subject Line")).toBeInTheDocument();
  });

  it("displays help text for plain text body", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText(/Shown to recipients whose email clients don't support HTML/)).toBeInTheDocument();
    });
  });

  it("displays template content card with description", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Template Content")).toBeInTheDocument();
      expect(screen.getByText(/Edit the subject line and body content/)).toBeInTheDocument();
    });
  });

  it("saves template successfully", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.update).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

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

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/password_reset"]} />);

    await waitFor(() => {
      expect(screen.getByText("Password Reset")).toBeInTheDocument();
      expect(screen.getByText("{{user_name}}")).toBeInTheDocument();
      expect(screen.getByText("{{reset_link}}")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Component Interaction Tests (covers uncovered functions/lines)
  // ============================================================================

  it("opens send test email dialog when clicking Send Test Email button", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    // Click "Send Test Email" button to open the dialog
    const sendTestButton = screen.getByRole("button", { name: /send test email/i });
    await user.click(sendTestButton);

    // The dialog should appear with recipient email field and variable inputs
    await waitFor(() => {
      expect(screen.getByText("Send a test email using the current template content with custom variable values.")).toBeInTheDocument();
    });

    expect(screen.getByLabelText("Recipient Email")).toBeInTheDocument();
  });

  it("send test email dialog shows template variables with example values", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    const sendTestButton = screen.getByRole("button", { name: /send test email/i });
    await user.click(sendTestButton);

    await waitFor(() => {
      expect(screen.getByText("Template Variables")).toBeInTheDocument();
    });

    // Variables should be pre-filled with example values
    expect(screen.getByDisplayValue("John Doe")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Acme Corp")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Auth9")).toBeInTheDocument();
  });

  it("allows editing test variable values in send test email dialog", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    // Open the dialog
    const sendTestButton = screen.getByRole("button", { name: /send test email/i });
    await user.click(sendTestButton);

    await waitFor(() => {
      expect(screen.getByDisplayValue("John Doe")).toBeInTheDocument();
    });

    // Edit a test variable value (covers lines 390-394: setTestVariables onChange handler)
    const inviterNameInput = screen.getByDisplayValue("John Doe");
    await user.clear(inviterNameInput);
    await user.type(inviterNameInput, "Jane Smith");

    expect(screen.getByDisplayValue("Jane Smith")).toBeInTheDocument();
  });

  it("closes send test email dialog when cancel button is clicked", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    // Open the dialog
    const sendTestButton = screen.getByRole("button", { name: /send test email/i });
    await user.click(sendTestButton);

    await waitFor(() => {
      expect(screen.getByText("Recipient Email")).toBeInTheDocument();
    });

    // Click Cancel to close the dialog (covers setSendTestDialogOpen(false))
    const cancelButton = screen.getByRole("button", { name: /^cancel$/i });
    await user.click(cancelButton);

    await waitFor(() => {
      expect(screen.queryByText("Recipient Email")).not.toBeInTheDocument();
    });
  });

  it("switches preview tabs between HTML and Text", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.preview).mockResolvedValue({
      data: {
        subject: "Preview Subject",
        html_body: "<html><body>HTML Preview</body></html>",
        text_body: "Text Preview Content",
      },
    });

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });

    // Click Preview button to trigger preview
    const previewButton = screen.getByText("Preview");
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(screen.getByText("Preview Subject")).toBeInTheDocument();
    });

    // HTML tab should be active by default
    const htmlTab = screen.getByRole("tab", { name: "HTML" });
    const textTab = screen.getByRole("tab", { name: "Text" });

    expect(htmlTab).toHaveAttribute("data-state", "active");

    // Switch to Text tab using userEvent (covers setPreviewTab handler, line 466)
    await user.click(textTab);

    await waitFor(() => {
      expect(textTab).toHaveAttribute("data-state", "active");
    });

    // The plain text content should now be visible
    expect(screen.getByText("Text Preview Content")).toBeInTheDocument();
  });

  it("allows editing HTML body content", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("<html><body>Hello {{inviter_name}}</body></html>")).toBeInTheDocument();
    });

    const htmlBodyTextarea = screen.getByLabelText("HTML Body");
    fireEvent.change(htmlBodyTextarea, { target: { value: "<html><body>Updated</body></html>" } });

    expect(screen.getByDisplayValue("<html><body>Updated</body></html>")).toBeInTheDocument();
  });

  it("allows editing plain text body content", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("You're Invited!")).toBeInTheDocument();
    });

    const textBodyTextarea = screen.getByLabelText("Plain Text Body");
    fireEvent.change(textBodyTextarea, { target: { value: "Updated plain text" } });

    expect(screen.getByDisplayValue("Updated plain text")).toBeInTheDocument();
  });

  it("action handles sendTest intent successfully", async () => {
    vi.mocked(emailTemplateApi.sendTestEmail).mockResolvedValue({
      success: true,
      message: "Test email sent successfully",
    });

    const formData = new FormData();
    formData.append("intent", "sendTest");
    formData.append("to_email", "recipient@example.com");
    formData.append("subject", "Test subject");
    formData.append("html_body", "<html>Test</html>");
    formData.append("text_body", "Test");
    formData.append("variables", JSON.stringify({ inviter_name: "John" }));

    const request = new Request("http://localhost/dashboard/settings/email-templates/invitation", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { type: "invitation" }, context: {} });

    expect(emailTemplateApi.sendTestEmail).toHaveBeenCalledWith(
      "invitation",
      {
        to_email: "recipient@example.com",
        subject: "Test subject",
        html_body: "<html>Test</html>",
        text_body: "Test",
        variables: { inviter_name: "John" },
      },
      undefined,
    );
    expect(response).toEqual({
      testEmailSuccess: true,
      testEmailMessage: "Test email sent successfully",
    });
  });

  it("action handles sendTest intent failure", async () => {
    vi.mocked(emailTemplateApi.sendTestEmail).mockResolvedValue({
      success: false,
      message: "SMTP connection failed",
    });

    const formData = new FormData();
    formData.append("intent", "sendTest");
    formData.append("to_email", "recipient@example.com");
    formData.append("subject", "Test subject");
    formData.append("html_body", "<html>Test</html>");
    formData.append("text_body", "Test");
    formData.append("variables", "{}");

    const request = new Request("http://localhost/dashboard/settings/email-templates/invitation", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { type: "invitation" }, context: {} });

    expect(response).toEqual({
      testEmailSuccess: false,
      testEmailError: "SMTP connection failed",
    });
  });

  it("action handles sendTest with empty variables", async () => {
    vi.mocked(emailTemplateApi.sendTestEmail).mockResolvedValue({
      success: true,
      message: "Sent",
    });

    const formData = new FormData();
    formData.append("intent", "sendTest");
    formData.append("to_email", "test@example.com");
    formData.append("subject", "Subject");
    formData.append("html_body", "<p>Body</p>");
    formData.append("text_body", "Body");
    // No variables field - should default to empty object

    const request = new Request("http://localhost/dashboard/settings/email-templates/invitation", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { type: "invitation" }, context: {} });

    expect(emailTemplateApi.sendTestEmail).toHaveBeenCalledWith(
      "invitation",
      expect.objectContaining({ variables: {} }),
      undefined,
    );
    expect(response).toEqual({
      testEmailSuccess: true,
      testEmailMessage: "Sent",
    });
  });

  it("action returns invalid intent error for unknown intent", async () => {
    const formData = new FormData();
    formData.append("intent", "unknown");

    const request = new Request("http://localhost/dashboard/settings/email-templates/invitation", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { type: "invitation" }, context: {} });

    // action returns a plain object { error: "Invalid intent" }
    expect(response).toEqual({ error: "Invalid intent" });
  });

  it("displays test email success message in component", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action: async () => {
          return { testEmailSuccess: true, testEmailMessage: "Test email sent to recipient@example.com" };
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Template")).toBeInTheDocument();
    });

    // Trigger the action by clicking save (the stub action always returns testEmail success)
    fireEvent.click(screen.getByText("Save Template"));

    await waitFor(() => {
      expect(screen.getByText("Test email sent to recipient@example.com")).toBeInTheDocument();
    });
  });

  it("displays test email error message in component", async () => {
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action: async () => {
          return { testEmailSuccess: false, testEmailError: "SMTP server unreachable" };
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Save Template")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText("Save Template"));

    await waitFor(() => {
      expect(screen.getByText("SMTP server unreachable")).toBeInTheDocument();
    });
  });

  it("action handles reset intent with redirect", async () => {
    vi.mocked(emailTemplateApi.reset).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("intent", "reset");

    const request = new Request("http://localhost/dashboard/settings/email-templates/invitation", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: { type: "invitation" }, context: {} });

    expect(emailTemplateApi.reset).toHaveBeenCalledWith("invitation", undefined);
    // The action returns a redirect Response
    expect(response).toBeInstanceOf(Response);
    expect((response as Response).status).toBe(302);
  });

  it("confirms reset template via dialog and triggers reset", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);
    vi.mocked(emailTemplateApi.reset).mockResolvedValue(undefined);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Reset to Default")).toBeInTheDocument();
    });

    // Open the reset confirmation dialog
    await user.click(screen.getByText("Reset to Default"));

    await waitFor(() => {
      expect(screen.getByText("Reset Template?")).toBeInTheDocument();
    });

    // Click the "Reset Template" confirmation button (covers lines 224-228: resetFetcher.submit)
    const confirmButton = screen.getByRole("button", { name: "Reset Template" });
    await user.click(confirmButton);

    // The fetcher should have been triggered with the reset intent
    await waitFor(() => {
      expect(emailTemplateApi.reset).toHaveBeenCalled();
    });
  });

  it("allows typing recipient email in send test dialog", async () => {
    const user = userEvent.setup();
    vi.mocked(emailTemplateApi.get).mockResolvedValue(mockTemplate);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates/:type",
        Component: EmailTemplateEditorPage,
        loader,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates/invitation"]} />);

    await waitFor(() => {
      expect(screen.getByText("Send Test Email")).toBeInTheDocument();
    });

    const sendTestButton = screen.getByRole("button", { name: /send test email/i });
    await user.click(sendTestButton);

    await waitFor(() => {
      expect(screen.getByLabelText("Recipient Email")).toBeInTheDocument();
    });

    // Type in the recipient email field
    const recipientInput = screen.getByLabelText("Recipient Email");
    await user.type(recipientInput, "qa@test.com");

    expect(recipientInput).toHaveValue("qa@test.com");
  });
});
