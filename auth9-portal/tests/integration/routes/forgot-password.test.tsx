import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
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
});
