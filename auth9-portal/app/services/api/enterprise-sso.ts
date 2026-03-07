import { API_BASE_URL, getHeaders, handleResponse, type ApiError } from "./client";

export interface EnterpriseSsoDiscoveryInput {
  email: string;
}

export interface EnterpriseSsoDiscoveryResponse {
  tenant_id: string;
  tenant_slug: string;
  connector_alias: string;
  authorize_url: string;
}

export const enterpriseSsoApi = {
  discover: async (
    input: EnterpriseSsoDiscoveryInput,
    query: {
      response_type: string;
      client_id: string;
      redirect_uri: string;
      scope: string;
      state: string;
      nonce?: string;
      ui_locales?: string;
    }
  ): Promise<{ data: EnterpriseSsoDiscoveryResponse }> => {
    const url = new URL(`${API_BASE_URL}/api/v1/enterprise-sso/discovery`);
    url.searchParams.set("response_type", query.response_type);
    url.searchParams.set("client_id", query.client_id);
    url.searchParams.set("redirect_uri", query.redirect_uri);
    url.searchParams.set("scope", query.scope);
    url.searchParams.set("state", query.state);
    if (query.nonce) url.searchParams.set("nonce", query.nonce);
    if (query.ui_locales) url.searchParams.set("ui_locales", query.ui_locales);

    const response = await fetch(url.toString(), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },
};

export interface TenantSsoConnector {
  id: string;
  tenant_id: string;
  alias: string;
  display_name?: string;
  provider_type: "saml" | "oidc";
  enabled: boolean;
  priority: number;
  keycloak_alias: string;
  config: Record<string, string>;
  domains: string[];
  created_at: string;
  updated_at: string;
}

export interface CreateTenantSsoConnectorInput {
  alias: string;
  display_name?: string;
  provider_type: "saml" | "oidc";
  enabled?: boolean;
  priority?: number;
  config: Record<string, string>;
  domains: string[];
}

export interface UpdateTenantSsoConnectorInput {
  display_name?: string;
  enabled?: boolean;
  priority?: number;
  config?: Record<string, string>;
  domains?: string[];
}

export const tenantSsoApi = {
  list: async (
    tenantId: string,
    accessToken?: string
  ): Promise<{ data: TenantSsoConnector[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  create: async (
    tenantId: string,
    input: CreateTenantSsoConnectorInput,
    accessToken?: string
  ): Promise<{ data: TenantSsoConnector }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify(input),
      }
    );
    return handleResponse(response);
  },

  update: async (
    tenantId: string,
    connectorId: string,
    input: UpdateTenantSsoConnectorInput,
    accessToken?: string
  ): Promise<{ data: TenantSsoConnector }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify(input),
      }
    );
    return handleResponse(response);
  },

  delete: async (
    tenantId: string,
    connectorId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
      {
        method: "DELETE",
        headers: getHeaders(accessToken),
      }
    );
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  test: async (
    tenantId: string,
    connectorId: string,
    accessToken?: string
  ): Promise<{ data: { ok: boolean; message: string } }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}/test`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },
};
