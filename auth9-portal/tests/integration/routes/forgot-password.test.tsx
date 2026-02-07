import { createRoutesStub } from "react-router";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import ForgotPasswordPage, { action } from "~/routes/forgot-password";
import { passwordApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  passwordApi: {
    forgotPassword: vi.fn(),
  },
}));

describe("Forgot Password Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders forgot password form", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    expect(screen.getByText("Forgot password?")).toBeInTheDocument();
    expect(
      screen.getByText(/enter your email address and we will send you a link/i)
    ).toBeInTheDocument();
  });

  it("renders email input field", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const emailInput = screen.getByPlaceholderText("you@example.com");
    expect(emailInput).toBeInTheDocument();
    expect(emailInput).toHaveAttribute("type", "email");
    expect(emailInput).toHaveAttribute("required");
  });

  it("renders submit button with correct text", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    expect(submitButton).toBeInTheDocument();
    expect(submitButton).not.toBeDisabled();
  });

  it("has link back to login page", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const loginLink = screen.getByRole("link", { name: /back to login/i });
    expect(loginLink).toBeInTheDocument();
    expect(loginLink).toHaveAttribute("href", "/login");
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action returns error when email is missing", async () => {
    const formData = new FormData();
    // Don't add email

    const request = new Request("http://localhost/forgot-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Email is required" });
  });

  it("action returns success on valid email", async () => {
    vi.mocked(passwordApi.forgotPassword).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("email", "test@example.com");

    const request = new Request("http://localhost/forgot-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(passwordApi.forgotPassword).toHaveBeenCalledWith("test@example.com");
    expect(response).toEqual({ success: true });
  });

  it("action returns success even when API fails (to prevent email enumeration)", async () => {
    vi.mocked(passwordApi.forgotPassword).mockRejectedValue(new Error("User not found"));

    const formData = new FormData();
    formData.append("email", "unknown@example.com");

    const request = new Request("http://localhost/forgot-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    // Should still return success to prevent email enumeration
    expect(response).toEqual({ success: true });
  });

  it("action returns success even on network error", async () => {
    vi.mocked(passwordApi.forgotPassword).mockRejectedValue(new Error("Network error"));

    const formData = new FormData();
    formData.append("email", "test@example.com");

    const request = new Request("http://localhost/forgot-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(passwordApi.forgotPassword).toHaveBeenCalledWith("test@example.com");
    expect(response).toEqual({ success: true });
  });

  it("action does not call API when email is empty", async () => {
    const formData = new FormData();
    // Explicitly not appending email field

    const request = new Request("http://localhost/forgot-password", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(passwordApi.forgotPassword).not.toHaveBeenCalled();
    expect(response).toEqual({ error: "Email is required" });
  });

  // ============================================================================
  // Component Interaction Tests (covers lines 35-62: success state, 90-92: error display)
  // ============================================================================

  it("shows success state after form submission", async () => {
    const user = userEvent.setup();
    vi.mocked(passwordApi.forgotPassword).mockResolvedValue(undefined);

    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    // Type email into the input
    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "test@example.com");

    // Submit the form
    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    await user.click(submitButton);

    // After successful submission, the success state should be rendered (lines 35-62)
    await waitFor(() => {
      expect(screen.getByText("Check your email")).toBeInTheDocument();
    });

    expect(
      screen.getByText(/if an account exists for/i)
    ).toBeInTheDocument();
  });

  it("success state shows the submitted email address", async () => {
    const user = userEvent.setup();
    vi.mocked(passwordApi.forgotPassword).mockResolvedValue(undefined);

    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "user@domain.com");

    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText("Check your email")).toBeInTheDocument();
    });

    // The email address should be displayed in the success message
    expect(screen.getByText("user@domain.com")).toBeInTheDocument();
  });

  it("success state has try again link", async () => {
    const user = userEvent.setup();
    vi.mocked(passwordApi.forgotPassword).mockResolvedValue(undefined);

    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "test@example.com");

    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText("Check your email")).toBeInTheDocument();
    });

    // Verify the "try again" link exists
    const tryAgainLink = screen.getByRole("link", { name: /try again/i });
    expect(tryAgainLink).toBeInTheDocument();
    expect(tryAgainLink).toHaveAttribute("href", "/forgot-password");
  });

  it("success state has back to login button", async () => {
    const user = userEvent.setup();
    vi.mocked(passwordApi.forgotPassword).mockResolvedValue(undefined);

    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "test@example.com");

    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText("Check your email")).toBeInTheDocument();
    });

    // Verify the "Back to login" button/link exists in the success state
    const backToLoginLink = screen.getByRole("link", { name: /back to login/i });
    expect(backToLoginLink).toBeInTheDocument();
    expect(backToLoginLink).toHaveAttribute("href", "/login");
  });

  it("success state shows spam folder notice", async () => {
    const user = userEvent.setup();
    vi.mocked(passwordApi.forgotPassword).mockResolvedValue(undefined);

    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "test@example.com");

    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText("Check your email")).toBeInTheDocument();
    });

    // Verify the spam folder notice text (lines 47-48)
    expect(
      screen.getByText(/did not receive the email\? check your spam folder/i)
    ).toBeInTheDocument();
  });

  it("displays error message when action returns error", async () => {
    const user = userEvent.setup();

    // Use a stub action that always returns an error to exercise lines 90-92
    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
        action: async () => {
          return { error: "Email is required" };
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    // Type an email to satisfy the required attribute, then submit
    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "test@example.com");

    const submitButton = screen.getByRole("button", { name: /send reset link/i });
    await user.click(submitButton);

    // The error message should be displayed (lines 90-92)
    await waitFor(() => {
      expect(screen.getByText("Email is required")).toBeInTheDocument();
    });
  });

  it("email input tracks typed value via onChange", async () => {
    const user = userEvent.setup();

    const RoutesStub = createRoutesStub([
      {
        path: "/forgot-password",
        Component: ForgotPasswordPage,
      },
    ]);

    render(<RoutesStub initialEntries={["/forgot-password"]} />);

    const emailInput = screen.getByPlaceholderText("you@example.com");
    await user.type(emailInput, "typing@test.com");

    expect(emailInput).toHaveValue("typing@test.com");
  });
});
