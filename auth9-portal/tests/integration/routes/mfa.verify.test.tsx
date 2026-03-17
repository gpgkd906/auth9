import { createRoutesStub } from "react-router";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect } from "vitest";
import MfaVerifyPage, { action, loader } from "~/routes/mfa.verify";

describe("MFA Verify Page", () => {
  it("loader returns branding payload", async () => {
    const request = new Request("http://localhost/mfa/verify");
    const response = await loader({ request, params: {}, context: {} });

    expect(response).toEqual(
      expect.objectContaining({
        branding: expect.objectContaining({ primary_color: "#007AFF" }),
      }),
    );
  });

  it("renders hosted MFA shell", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/mfa/verify",
        Component: MfaVerifyPage,
        loader: () => ({
          branding: {
            company_name: "Auth9",
            allow_registration: false,
          },
        }),
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/mfa/verify"]} />);

    expect(await screen.findByText("Verify MFA")).toBeInTheDocument();
    expect(screen.getByText(/hosted MFA flow/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/verification code/i)).toBeInTheDocument();
  });

  it("action requires a verification code", async () => {
    const request = new Request("http://localhost/mfa/verify", {
      method: "POST",
      body: new FormData(),
    });

    const response = await action({ request, params: {}, context: {} });
    expect(response).toEqual({ error: "请输入验证码。" });
  });

  it("shows compatibility message after submit", async () => {
    const user = userEvent.setup();
    const RoutesStub = createRoutesStub([
      {
        path: "/mfa/verify",
        Component: MfaVerifyPage,
        loader: () => ({
          branding: {
            company_name: "Auth9",
            allow_registration: false,
          },
        }),
        action,
      },
    ]);

    render(<RoutesStub initialEntries={["/mfa/verify"]} />);

    await user.type(await screen.findByLabelText(/verification code/i), "123456");
    await user.click(screen.getByRole("button", { name: /continue/i }));

    expect(await screen.findByText(/Hosted MFA 验证尚未接入/)).toBeInTheDocument();
  });
});
