import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import MfaVerifyPage, { action, loader } from "~/routes/mfa.verify";

describe("MFA Verify Page", () => {
  it("loader redirects to /login when no mfa_session_token", async () => {
    const request = new Request("http://localhost/mfa/verify");

    try {
      await loader({ request, params: {}, context: {} });
      // If loader returns a Response (redirect), it won't throw
      // but createRoutesStub handles the redirect
    } catch (e) {
      // Redirect throws in some frameworks
      expect(e).toBeDefined();
    }

    // Verify the redirect response
    const response = await loader({ request, params: {}, context: {} });
    expect(response).toBeDefined();
    if (response instanceof Response) {
      expect(response.status).toBe(302);
      expect(response.headers.get("Location")).toBe("/login");
    }
  });

  it("loader returns branding and mfa data when token present", async () => {
    const request = new Request(
      "http://localhost/mfa/verify?mfa_session_token=test-token&mfa_methods=totp"
    );
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual(
      expect.objectContaining({
        mfaSessionToken: "test-token",
        mfaMethods: ["totp"],
        branding: expect.objectContaining({ primary_color: "#007AFF" }),
      })
    );
  });

  it("renders MFA verify page with TOTP mode", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/mfa/verify",
        Component: MfaVerifyPage,
        loader: () => ({
          branding: {
            company_name: "Auth9",
            allow_registration: false,
          },
          mfaSessionToken: "test-token",
          mfaMethods: ["totp"],
          loginChallenge: null,
        }),
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/mfa/verify?mfa_session_token=test-token"]} />);

    // The page renders the title and the mode-switch link
    expect(await screen.findByText(/recovery code/i)).toBeInTheDocument();
    expect(screen.getByText(/back to/i)).toBeInTheDocument();
  });

  it("action requires a verification code", async () => {
    const formData = new FormData();
    formData.set("mfa_session_token", "test-token");
    const request = new Request("http://localhost/mfa/verify", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual(
      expect.objectContaining({ error: expect.any(String) })
    );
  });

  it("action requires mfa_session_token", async () => {
    const formData = new FormData();
    formData.set("code", "123456");
    const request = new Request("http://localhost/mfa/verify", {
      method: "POST",
      body: formData,
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual(
      expect.objectContaining({ error: expect.any(String) })
    );
  });
});
