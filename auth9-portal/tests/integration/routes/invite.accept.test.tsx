import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import InviteAcceptPage, {
  loader,
  action,
} from "~/routes/invite.accept";
import { invitationApi } from "~/services/api";

// Mock APIs
vi.mock("~/services/api", () => ({
  invitationApi: {
    accept: vi.fn(),
  },
}));

describe("Invite Accept Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  describe("loader", () => {
    it("returns token from URL search params", async () => {
      const request = new Request(
        "http://localhost/invite/accept?token=abc123"
      );
      const result = await loader({ request, params: {}, context: {} });
      expect(result).toEqual({ token: "abc123" });
    });

    it("returns null token when not provided", async () => {
      const request = new Request("http://localhost/invite/accept");
      const result = await loader({ request, params: {}, context: {} });
      expect(result).toEqual({ token: null });
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  describe("action", () => {
    it("returns 400 when token is missing", async () => {
      const formData = new FormData();
      // No token field
      const request = new Request("http://localhost/invite/accept", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const data = await (response as Response).json();
      expect(data.error).toBe("Invitation token is missing");
      expect((response as Response).status).toBe(400);
    });

    it("accepts invitation successfully", async () => {
      vi.mocked(invitationApi.accept).mockResolvedValue({
        data: { id: "inv-1", status: "accepted" },
      });

      const formData = new FormData();
      formData.append("token", "valid-token");
      formData.append("email", "test@example.com");
      formData.append("display_name", "Test User");
      formData.append("password", "Password123!");

      const request = new Request("http://localhost/invite/accept", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(result).toEqual({
        success: true,
        invitation: { id: "inv-1", status: "accepted" },
      });

      expect(invitationApi.accept).toHaveBeenCalledWith({
        token: "valid-token",
        email: "test@example.com",
        display_name: "Test User",
        password: "Password123!",
      });
    });

    it("accepts invitation with optional fields omitted", async () => {
      vi.mocked(invitationApi.accept).mockResolvedValue({
        data: { id: "inv-2", status: "accepted" },
      });

      const formData = new FormData();
      formData.append("token", "valid-token");
      // No email, display_name, password

      const request = new Request("http://localhost/invite/accept", {
        method: "POST",
        body: formData,
      });

      const result = await action({ request, params: {}, context: {} });
      expect(result).toEqual({
        success: true,
        invitation: { id: "inv-2", status: "accepted" },
      });

      expect(invitationApi.accept).toHaveBeenCalledWith({
        token: "valid-token",
        email: undefined,
        display_name: undefined,
        password: undefined,
      });
    });

    it("returns error when API throws", async () => {
      vi.mocked(invitationApi.accept).mockRejectedValue(
        new Error("Token expired")
      );

      const formData = new FormData();
      formData.append("token", "expired-token");

      const request = new Request("http://localhost/invite/accept", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      expect(response).toBeInstanceOf(Response);
      const data = await (response as Response).json();
      expect(data.error).toBe("Token expired");
    });

    it("returns 'Unknown error' for non-Error exceptions", async () => {
      vi.mocked(invitationApi.accept).mockRejectedValue("string error");

      const formData = new FormData();
      formData.append("token", "some-token");

      const request = new Request("http://localhost/invite/accept", {
        method: "POST",
        body: formData,
      });

      const response = await action({ request, params: {}, context: {} });
      const data = await (response as Response).json();
      expect(data.error).toBe("Unknown error");
    });
  });

  // ============================================================================
  // Component Tests
  // ============================================================================

  describe("component", () => {
    it("renders 'Invalid Invitation' when no token", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/invite/accept",
          Component: InviteAcceptPage,
          loader: () => ({ token: null }),
        },
      ]);

      render(<RoutesStub initialEntries={["/invite/accept"]} />);

      await waitFor(() => {
        expect(screen.getByText("Invalid Invitation")).toBeInTheDocument();
        expect(
          screen.getByText("The invitation link is missing or malformed.")
        ).toBeInTheDocument();
        expect(screen.getByText("Go to login")).toBeInTheDocument();
      });
    });

    it("renders accept invitation form when token exists", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/invite/accept",
          Component: InviteAcceptPage,
          loader: () => ({ token: "valid-token" }),
        },
      ]);

      render(<RoutesStub initialEntries={["/invite/accept"]} />);

      await waitFor(() => {
        expect(
          screen.getByText(
            "Create your account or confirm your details to join the tenant."
          )
        ).toBeInTheDocument();
      });

      // Check form fields
      expect(screen.getByLabelText("Email (optional)")).toBeInTheDocument();
      expect(screen.getByLabelText("Display Name")).toBeInTheDocument();
      expect(screen.getByLabelText("Password")).toBeInTheDocument();
      // "Accept Invitation" appears both as heading and button text
      expect(
        screen.getByRole("button", { name: "Accept Invitation" })
      ).toBeInTheDocument();
    });

    it("renders sign in link", async () => {
      const RoutesStub = createRoutesStub([
        {
          path: "/invite/accept",
          Component: InviteAcceptPage,
          loader: () => ({ token: "valid-token" }),
        },
      ]);

      render(<RoutesStub initialEntries={["/invite/accept"]} />);

      await waitFor(() => {
        expect(screen.getByText("Sign in")).toBeInTheDocument();
      });
    });
  });
});
