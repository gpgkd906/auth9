import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import EmailTemplatesPage, { loader } from "~/routes/dashboard.settings.email-templates._index";
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

describe("Email Templates Page", () => {
  const mockTemplates = {
    data: [
      {
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
          html_body: "<html>...</html>",
          text_body: "You're Invited!",
        },
        is_customized: true,
        updated_at: "2026-01-31T10:00:00Z",
      },
      {
        metadata: {
          template_type: "password_reset" as const,
          name: "Password Reset",
          description: "Sent when a user requests to reset their password",
          variables: [
            { name: "user_name", description: "Name of the user", example: "Jane Smith" },
            { name: "reset_link", description: "Password reset URL", example: "https://example.com/reset/xyz" },
            { name: "app_name", description: "Application name", example: "Auth9" },
            { name: "year", description: "Current year", example: "2026" },
          ],
        },
        content: {
          subject: "Reset your password",
          html_body: "<html>...</html>",
          text_body: "Reset your password",
        },
        is_customized: false,
        updated_at: undefined,
      },
      {
        metadata: {
          template_type: "email_mfa" as const,
          name: "Email MFA",
          description: "Sent when using email-based MFA verification",
          variables: [
            { name: "user_name", description: "Name of the user", example: "Jane Smith" },
            { name: "verification_code", description: "MFA code", example: "123456" },
            { name: "app_name", description: "Application name", example: "Auth9" },
            { name: "year", description: "Current year", example: "2026" },
          ],
        },
        content: {
          subject: "Your verification code: {{verification_code}}",
          html_body: "<html>...</html>",
          text_body: "Verification code",
        },
        is_customized: false,
        updated_at: undefined,
      },
    ],
  };

  it("renders email templates list", async () => {
    vi.mocked(emailTemplateApi.list).mockResolvedValue(mockTemplates);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Templates")).toBeInTheDocument();
      expect(screen.getByText(/Customize the content and appearance of emails/)).toBeInTheDocument();
    });
  });

  it("renders template table with names and descriptions", async () => {
    vi.mocked(emailTemplateApi.list).mockResolvedValue(mockTemplates);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      expect(screen.getByText("User Invitation")).toBeInTheDocument();
      expect(screen.getByText("Password Reset")).toBeInTheDocument();
      expect(screen.getByText("Email MFA")).toBeInTheDocument();
      expect(screen.getByText("Sent when inviting users to join a tenant")).toBeInTheDocument();
    });
  });

  it("shows Custom badge for customized templates", async () => {
    vi.mocked(emailTemplateApi.list).mockResolvedValue(mockTemplates);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      expect(screen.getByText("Custom")).toBeInTheDocument();
      expect(screen.getAllByText("Default")).toHaveLength(2);
    });
  });

  it("renders Edit buttons for each template", async () => {
    vi.mocked(emailTemplateApi.list).mockResolvedValue(mockTemplates);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      const editButtons = screen.getAllByText("Edit");
      expect(editButtons).toHaveLength(3);
    });
  });

  it("shows template variables info card", async () => {
    vi.mocked(emailTemplateApi.list).mockResolvedValue(mockTemplates);

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      expect(screen.getByText("Template Variables")).toBeInTheDocument();
      expect(screen.getByText("Common variables available in all templates:")).toBeInTheDocument();
    });
  });

  it("handles API error gracefully", async () => {
    vi.mocked(emailTemplateApi.list).mockRejectedValue(new Error("API Error"));

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      expect(screen.getByText("API Error")).toBeInTheDocument();
    });
  });

  it("renders empty state when no templates", async () => {
    vi.mocked(emailTemplateApi.list).mockResolvedValue({ data: [] });

    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/email-templates",
        Component: EmailTemplatesPage,
        loader,
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/email-templates"]} />);

    await waitFor(() => {
      expect(screen.getByText("Email Templates")).toBeInTheDocument();
      // Table headers should still be present
      expect(screen.getByText("Template")).toBeInTheDocument();
      expect(screen.getByText("Description")).toBeInTheDocument();
      expect(screen.getByText("Status")).toBeInTheDocument();
    });
  });
});
