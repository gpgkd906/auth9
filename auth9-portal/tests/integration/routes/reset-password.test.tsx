import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
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
});
