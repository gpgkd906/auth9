import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ResetPasswordPage, { loader, action } from "~/routes/reset-password";
import { passwordApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  passwordApi: {
    resetPassword: vi.fn(),
  },
}));

describe("Reset Password Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns error when token is missing", async () => {
    const request = new Request("http://localhost/reset-password");

    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Invalid or missing reset token" });
  });

  it("loader returns token when present", async () => {
    const request = new Request("http://localhost/reset-password?token=abc123");

    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({ token: "abc123" });
  });

  it("loader returns error when token param is empty string", async () => {
    const request = new Request("http://localhost/reset-password?token=");

    const response = await loader({ request, params: {}, context: {} });

    // Empty string is falsy, should return error
    expect(response).toEqual({ error: "Invalid or missing reset token" });
  });

  it("loader preserves full token value with special characters", async () => {
    const request = new Request(
      "http://localhost/reset-password?token=abc-123_def.456"
    );

    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual({ token: "abc-123_def.456" });
  });

  // ============================================================================
  // Rendering Tests - Invalid Token
  // ============================================================================

  it("renders error state when token is missing", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ error: "Invalid or missing reset token" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invalid Link")).toBeInTheDocument();
    });
    expect(screen.getByText("Invalid or missing reset token")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /request new reset link/i })).toBeInTheDocument();
  });

  // ============================================================================
  // Rendering Tests - Valid Token
  // ============================================================================

  it("renders password form when token is valid", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByText("Set new password")).toBeInTheDocument();
    });
    expect(screen.getByText(/enter your new password below/i)).toBeInTheDocument();
  });

  it("renders password input fields", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText(/new password/i)).toBeInTheDocument();
    });
    expect(screen.getByLabelText(/confirm password/i)).toBeInTheDocument();
    expect(screen.getByText(/must be at least 8 characters/i)).toBeInTheDocument();
  });

  it("renders submit button", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reset password/i })).toBeInTheDocument();
    });
  });

  it("has link back to login page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      const loginLink = screen.getByRole("link", { name: /back to login/i });
      expect(loginLink).toBeInTheDocument();
      expect(loginLink).toHaveAttribute("href", "/login");
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action returns error when token is missing", async () => {
    const formData = new FormData();
    formData.append("password", "newpassword123");
    formData.append("confirmPassword", "newpassword123");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Invalid reset token" });
  });

  it("action returns error when password is too short", async () => {
    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "short");
    formData.append("confirmPassword", "short");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Password must be at least 8 characters" });
  });

  it("action returns error when passwords do not match", async () => {
    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "newpassword123");
    formData.append("confirmPassword", "differentpassword");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Passwords do not match" });
  });

  it("action returns success on valid reset", async () => {
    vi.mocked(passwordApi.resetPassword).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "newpassword123");
    formData.append("confirmPassword", "newpassword123");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(passwordApi.resetPassword).toHaveBeenCalledWith("valid-token", "newpassword123");
    expect(response).toEqual({ success: true });
  });

  it("action returns error when API fails", async () => {
    vi.mocked(passwordApi.resetPassword).mockRejectedValue(new Error("Token expired"));

    const formData = new FormData();
    formData.append("token", "expired-token");
    formData.append("password", "newpassword123");
    formData.append("confirmPassword", "newpassword123");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Token expired" });
  });

  it("action returns generic error when non-Error is thrown", async () => {
    vi.mocked(passwordApi.resetPassword).mockRejectedValue("unexpected error");

    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "newpassword123");
    formData.append("confirmPassword", "newpassword123");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Failed to reset password" });
  });

  it("action returns error when password is empty", async () => {
    const formData = new FormData();
    formData.append("token", "valid-token");
    // password not appended
    formData.append("confirmPassword", "newpassword123");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Password must be at least 8 characters" });
  });

  it("action succeeds with password exactly 8 characters", async () => {
    vi.mocked(passwordApi.resetPassword).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "exactly8");
    formData.append("confirmPassword", "exactly8");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(passwordApi.resetPassword).toHaveBeenCalledWith("valid-token", "exactly8");
    expect(response).toEqual({ success: true });
  });

  it("action returns error with password of 7 characters", async () => {
    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "seven77");
    formData.append("confirmPassword", "seven77");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Password must be at least 8 characters" });
  });

  it("action does not call API when validation fails", async () => {
    const formData = new FormData();
    formData.append("token", "valid-token");
    formData.append("password", "newpassword123");
    formData.append("confirmPassword", "mismatch12345");

    const request = new Request("http://localhost/reset-password", {
      method: "POST",
      body: formData,
    });

    await action({ request, params: {}, context: {} });

    expect(passwordApi.resetPassword).not.toHaveBeenCalled();
  });

  // ============================================================================
  // Rendering Tests - Success State
  // ============================================================================

  it("renders success state after password reset", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
        action: () => ({ success: true }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByText("Set new password")).toBeInTheDocument();
    });

    // Fill in passwords
    const passwordInput = screen.getByLabelText(/new password/i);
    const confirmInput = screen.getByLabelText(/confirm password/i);
    await user.type(passwordInput, "newpassword123");
    await user.type(confirmInput, "newpassword123");

    // Submit the form
    const submitButton = screen.getByRole("button", { name: /reset password/i });
    await user.click(submitButton);

    // Success state should appear
    await waitFor(() => {
      expect(screen.getByText("Password reset successful")).toBeInTheDocument();
      expect(screen.getByText(/your password has been updated/i)).toBeInTheDocument();
    });

    // Sign in button should be present
    expect(screen.getByRole("button", { name: /sign in/i })).toBeInTheDocument();
  });

  it("renders sign in link on success page pointing to /login", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
        action: () => ({ success: true }),
      },
      {
        path: "/login",
        Component: () => <div>Login Page</div>,
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByText("Set new password")).toBeInTheDocument();
    });

    const passwordInput = screen.getByLabelText(/new password/i);
    const confirmInput = screen.getByLabelText(/confirm password/i);
    await user.type(passwordInput, "newpassword123");
    await user.type(confirmInput, "newpassword123");

    const submitButton = screen.getByRole("button", { name: /reset password/i });
    await user.click(submitButton);

    await waitFor(() => {
      const signInLink = screen.getByRole("link", { name: /sign in/i });
      expect(signInLink).toHaveAttribute("href", "/login");
    });
  });

  // ============================================================================
  // Rendering Tests - Error Display
  // ============================================================================

  it("renders error message from action in the form", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
        action: () => ({ error: "Token has expired" }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByText("Set new password")).toBeInTheDocument();
    });

    const passwordInput = screen.getByLabelText(/new password/i);
    const confirmInput = screen.getByLabelText(/confirm password/i);
    await user.type(passwordInput, "newpassword123");
    await user.type(confirmInput, "newpassword123");

    const submitButton = screen.getByRole("button", { name: /reset password/i });
    await user.click(submitButton);

    // Error message should appear
    await waitFor(() => {
      expect(screen.getByText("Token has expired")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Password input interaction tests
  // ============================================================================

  it("allows typing in password fields", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ token: "valid-token" }),
      },
    ]);

    const user = userEvent.setup();
    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText(/new password/i)).toBeInTheDocument();
    });

    const passwordInput = screen.getByLabelText(/new password/i);
    const confirmInput = screen.getByLabelText(/confirm password/i);

    await user.type(passwordInput, "test1234");
    await user.type(confirmInput, "test1234");

    expect(passwordInput).toHaveValue("test1234");
    expect(confirmInput).toHaveValue("test1234");
  });

  it("renders forgot password link on error page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/reset-password",
        Component: ResetPasswordPage,
        loader: () => ({ error: "Invalid or missing reset token" }),
      },
      {
        path: "/forgot-password",
        Component: () => <div>Forgot Password Page</div>,
      },
    ]);

    render(<RoutesStub initialEntries={["/reset-password"]} />);

    await waitFor(() => {
      expect(screen.getByText("Invalid Link")).toBeInTheDocument();
    });

    const requestLink = screen.getByRole("link", { name: /request new reset link/i });
    expect(requestLink).toHaveAttribute("href", "/forgot-password");
  });
});
