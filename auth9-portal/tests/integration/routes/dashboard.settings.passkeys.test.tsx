import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import PasskeysPage, { loader, action } from "~/routes/dashboard.settings.passkeys";
import { webauthnApi } from "~/services/api";

// Mock the API
vi.mock("~/services/api", () => ({
  webauthnApi: {
    listPasskeys: vi.fn(),
    deletePasskey: vi.fn(),
    getRegisterUrl: vi.fn(),
  },
}));

const mockPasskey = {
  id: "cred-1",
  user_label: "My Macbook",
  credential_type: "webauthn-passwordless",
  created_at: "2024-01-15T10:00:00Z",
};

const mockPasskey2 = {
  id: "cred-2",
  user_label: "iPhone 15",
  credential_type: "webauthn",
  created_at: "2024-02-20T15:30:00Z",
};

describe("Passkeys Settings Page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Loader Tests
  // ============================================================================

  it("loader returns passkeys from API", async () => {
    vi.mocked(webauthnApi.listPasskeys).mockResolvedValue({
      data: [mockPasskey, mockPasskey2],
    });

    const response = await loader();

    expect(response).toEqual({
      passkeys: [mockPasskey, mockPasskey2],
    });
  });

  it("loader returns empty passkeys on API error", async () => {
    vi.mocked(webauthnApi.listPasskeys).mockRejectedValue(new Error("API Error"));

    const response = await loader();

    expect(response).toEqual({
      passkeys: [],
      error: "Failed to load passkeys",
    });
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  it("renders passkeys page header", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      expect(screen.getByText("Passkeys")).toBeInTheDocument();
    });
    expect(
      screen.getByText(/passkeys are a secure, passwordless way to sign in/i)
    ).toBeInTheDocument();
  });

  it("renders add passkey button", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /add passkey/i })).toBeInTheDocument();
    });
  });

  it("renders empty state when no passkeys", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      expect(screen.getByText("No passkeys yet")).toBeInTheDocument();
    });
    expect(screen.getByText(/add a passkey to sign in faster/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /add your first passkey/i })).toBeInTheDocument();
  });

  it("renders passkey list when passkeys exist", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [mockPasskey, mockPasskey2] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      expect(screen.getByText("My Macbook")).toBeInTheDocument();
    });
    expect(screen.getByText("iPhone 15")).toBeInTheDocument();
    expect(screen.getByText("Passwordless")).toBeInTheDocument();
    expect(screen.getByText("Two-Factor")).toBeInTheDocument();
  });

  it("renders remove button for each passkey", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [mockPasskey] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /remove/i })).toBeInTheDocument();
    });
  });

  it("renders about passkeys section", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      expect(screen.getByText("About Passkeys")).toBeInTheDocument();
    });
    expect(screen.getByText("More secure")).toBeInTheDocument();
    expect(screen.getByText("Fast & easy")).toBeInTheDocument();
    expect(screen.getByText("Works everywhere")).toBeInTheDocument();
  });

  it("displays creation date for passkeys", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/settings/passkeys",
        Component: PasskeysPage,
        loader: () => ({ passkeys: [mockPasskey] }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/settings/passkeys"]} />);

    await waitFor(() => {
      // The date formatting will produce "January 15, 2024" or similar
      expect(screen.getByText(/added/i)).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Action Tests
  // ============================================================================

  it("action deletes passkey", async () => {
    vi.mocked(webauthnApi.deletePasskey).mockResolvedValue(undefined);

    const formData = new FormData();
    formData.append("intent", "delete");
    formData.append("credentialId", "cred-1");

    const request = new Request("http://localhost/dashboard/settings/passkeys", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(webauthnApi.deletePasskey).toHaveBeenCalledWith("cred-1", "");
    expect(response).toEqual({ success: true, message: "Passkey deleted" });
  });

  it("action returns register URL for registration", async () => {
    vi.mocked(webauthnApi.getRegisterUrl).mockResolvedValue({
      data: { url: "https://keycloak.example.com/register?action=WEBAUTHN_REGISTER" },
    });

    const formData = new FormData();
    formData.append("intent", "register");
    formData.append("redirectUri", "/dashboard/settings/passkeys");

    const request = new Request("http://localhost/dashboard/settings/passkeys", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(webauthnApi.getRegisterUrl).toHaveBeenCalledWith("/dashboard/settings/passkeys", "");
    expect(response).toEqual({
      redirect: "https://keycloak.example.com/register?action=WEBAUTHN_REGISTER",
    });
  });

  it("action returns error on API failure", async () => {
    vi.mocked(webauthnApi.deletePasskey).mockRejectedValue(new Error("Passkey not found"));

    const formData = new FormData();
    formData.append("intent", "delete");
    formData.append("credentialId", "invalid-id");

    const request = new Request("http://localhost/dashboard/settings/passkeys", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Passkey not found" });
  });

  it("action returns error for invalid intent", async () => {
    const formData = new FormData();
    formData.append("intent", "invalid");

    const request = new Request("http://localhost/dashboard/settings/passkeys", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });

    expect(response).toEqual({ error: "Invalid action" });
  });
});
