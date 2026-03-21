import { createRoutesStub } from "react-router";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import TenantSsoPage, { action, loader } from "~/routes/dashboard.tenants.$tenantId.sso";
import { I18nProvider } from "~/i18n";
import { tenantApi, tenantSsoApi } from "~/services/api";

vi.mock("~/services/api", () => ({
  tenantApi: {
    get: vi.fn(),
  },
  tenantSsoApi: {
    list: vi.fn(),
    create: vi.fn(),
    delete: vi.fn(),
    update: vi.fn(),
    test: vi.fn(),
  },
}));

vi.mock("~/services/session.server", () => ({
  getAccessToken: vi.fn().mockResolvedValue("test-token"),
}));

function WrappedPage() {
  return (
    <I18nProvider locale="en-US">
      <TenantSsoPage />
    </I18nProvider>
  );
}

function buildEnglishRequest(url: string, init?: RequestInit) {
  const headers = new Headers(init?.headers);
  headers.set("Accept-Language", "en-US");
  return new Request(url, { ...init, headers });
}

if (!HTMLElement.prototype.hasPointerCapture) {
  HTMLElement.prototype.hasPointerCapture = () => false;
}

if (!HTMLElement.prototype.setPointerCapture) {
  HTMLElement.prototype.setPointerCapture = () => {};
}

if (!HTMLElement.prototype.releasePointerCapture) {
  HTMLElement.prototype.releasePointerCapture = () => {};
}

describe("Tenant SSO Page", () => {
  const mockTenant = {
    id: "tenant-1",
    name: "Acme Corp",
    slug: "acme",
    status: "active",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(tenantApi.get).mockResolvedValue({ data: mockTenant });
    vi.mocked(tenantSsoApi.list).mockResolvedValue({ data: [] });
  });

  it("loader returns tenant and connectors", async () => {
    const result = await loader({
      request: buildEnglishRequest("http://localhost/dashboard/tenants/tenant-1/sso"),
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(result).toEqual({
      tenant: mockTenant,
      connectors: [],
      corePublicUrl: "http://localhost:8080",
    });
  });

  it("action creates oidc connector with selected provider type", async () => {
    vi.mocked(tenantSsoApi.create).mockResolvedValue({ data: { id: "connector-1" } });

    const formData = new FormData();
    formData.append("intent", "create");
    formData.append("enabled", "true");
    formData.append("alias", "acme-oidc");
    formData.append("display_name", "Acme OIDC");
    formData.append("provider_type", "oidc");
    formData.append("priority", "100");
    formData.append("domains", "acme.example.com, acme2.example.com");
    formData.append("client_id", "client-id");
    formData.append("client_secret", "client-credential-placeholder");
    formData.append("authorization_url", "https://idp.example.com/auth");
    formData.append("token_url", "https://idp.example.com/token");
    formData.append("userinfo_url", "https://idp.example.com/userinfo");

    const request = buildEnglishRequest("http://localhost/dashboard/tenants/tenant-1/sso", {
      method: "POST",
      body: formData,
    });

    const result = await action({
      request,
      params: { tenantId: "tenant-1" },
      context: {},
    });

    expect(result).toEqual({ success: true, message: "Connector created" });
    expect(tenantSsoApi.create).toHaveBeenCalledWith(
      "tenant-1",
      {
        alias: "acme-oidc",
        display_name: "Acme OIDC",
        provider_type: "oidc",
        enabled: true,
        priority: 100,
        domains: ["acme.example.com", "acme2.example.com"],
        config: {
          clientId: "client-id",
          clientSecret: "client-credential-placeholder", // pragma: allowlist secret
          authorizationUrl: "https://idp.example.com/auth",
          tokenUrl: "https://idp.example.com/token",
          userInfoUrl: "https://idp.example.com/userinfo",
        },
      },
      "test-token"
    );
  });

  it("renders provider type selector with saml as default", async () => {
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/sso",
        Component: WrappedPage,
        loader: () => ({ tenant: mockTenant, connectors: [], corePublicUrl: "http://localhost:8080" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/sso"]} />);

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Provider Type" })).toBeInTheDocument();
    });

    expect(screen.getByRole("combobox", { name: "Provider Type" })).toHaveTextContent("SAML");
    expect(screen.getByLabelText("SAML Entity ID")).toBeInTheDocument();
    expect(screen.queryByLabelText("OIDC Client ID")).not.toBeInTheDocument();
  });

  it("switches provider type to oidc and updates visible fields", async () => {
    const user = userEvent.setup();
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/sso",
        Component: WrappedPage,
        loader: () => ({ tenant: mockTenant, connectors: [], corePublicUrl: "http://localhost:8080" }),
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/sso"]} />);

    await waitFor(() => {
      expect(screen.getByRole("combobox", { name: "Provider Type" })).toBeInTheDocument();
    });

    await user.click(screen.getByRole("combobox", { name: "Provider Type" }));
    await user.click(await screen.findByRole("option", { name: "OIDC" }));

    expect(screen.getByRole("combobox", { name: "Provider Type" })).toHaveTextContent("OIDC");
    expect(screen.getByLabelText("OIDC Client ID")).toBeInTheDocument();
    expect(screen.queryByLabelText("SAML Entity ID")).not.toBeInTheDocument();
  });

  it("submits selected provider type from the form", async () => {
    const user = userEvent.setup();
    const RoutesStub = createRoutesStub([
      {
        path: "/dashboard/tenants/:tenantId/sso",
        Component: WrappedPage,
        loader: () => ({ tenant: mockTenant, connectors: [], corePublicUrl: "http://localhost:8080" }),
        action: async ({ request }) => {
          const formData = await request.formData();
          expect(formData.get("intent")).toBe("create");
          expect(formData.get("provider_type")).toBe("oidc");
          expect(formData.get("alias")).toBe("acme-oidc");
          expect(formData.get("domains")).toBe("acme.example.com");
          return { success: true, message: "ok" };
        },
      },
    ]);

    render(<RoutesStub initialEntries={["/dashboard/tenants/tenant-1/sso"]} />);

    await waitFor(() => {
      expect(screen.getByLabelText("Alias")).toBeInTheDocument();
    });

    await user.type(screen.getByLabelText("Alias"), "acme-oidc");
    await user.type(screen.getByLabelText("Domains"), "acme.example.com");
    await user.click(screen.getByRole("combobox", { name: "Provider Type" }));
    await user.click(await screen.findByRole("option", { name: "OIDC" }));
    await user.click(screen.getByRole("button", { name: "Create Connector" }));

    await waitFor(() => {
      expect(screen.getByText("ok")).toBeInTheDocument();
    });
  });
});
