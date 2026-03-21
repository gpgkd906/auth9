import {
  API_BASE_URL,
  ApiResponseError,
  getHeaders,
  handleResponse,
  type ApiError,
} from "./client";

export interface AttributeMapping {
  source: string;
  saml_attribute: string;
  friendly_name?: string;
}

export type NameIdFormat = "email" | "persistent" | "transient" | "unspecified";

export interface SamlApplication {
  id: string;
  tenant_id: string;
  name: string;
  entity_id: string;
  acs_url: string;
  slo_url?: string;
  name_id_format: string;
  sign_assertions: boolean;
  sign_responses: boolean;
  encrypt_assertions: boolean;
  sp_certificate?: string;
  attribute_mappings: AttributeMapping[];
  backend_client_id: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  sso_url: string;
}

export interface CreateSamlApplicationInput {
  name: string;
  entity_id: string;
  acs_url: string;
  slo_url?: string;
  name_id_format?: NameIdFormat;
  sign_assertions?: boolean;
  sign_responses?: boolean;
  encrypt_assertions?: boolean;
  sp_certificate?: string;
  attribute_mappings?: AttributeMapping[];
}

export interface CertificateInfo {
  certificate_pem: string;
  expires_at: string;
  expires_soon: boolean;
  days_until_expiry: number;
}

export interface UpdateSamlApplicationInput {
  name?: string;
  acs_url?: string;
  slo_url?: string;
  name_id_format?: NameIdFormat;
  sign_assertions?: boolean;
  sign_responses?: boolean;
  encrypt_assertions?: boolean;
  sp_certificate?: string;
  attribute_mappings?: AttributeMapping[];
  enabled?: boolean;
}

export const VALID_ATTRIBUTE_SOURCES = [
  "email",
  "display_name",
  "first_name",
  "last_name",
  "user_id",
  "tenant_roles",
  "tenant_permissions",
] as const;

export const SAML_APPLICATION_API_BASE = API_BASE_URL;

export const samlApplicationApi = {
  list: async (
    tenantId: string,
    accessToken?: string
  ): Promise<{ data: SamlApplication[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/saml-apps`,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },

  get: async (
    tenantId: string,
    appId: string,
    accessToken?: string
  ): Promise<{ data: SamlApplication }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/saml-apps/${appId}`,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },

  create: async (
    tenantId: string,
    input: CreateSamlApplicationInput,
    accessToken?: string
  ): Promise<{ data: SamlApplication }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/saml-apps`,
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
    appId: string,
    input: UpdateSamlApplicationInput,
    accessToken?: string
  ): Promise<{ data: SamlApplication }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/saml-apps/${appId}`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify(input),
      }
    );
    return handleResponse(response);
  },

  getCertificateInfo: async (
    tenantId: string,
    appId: string,
    accessToken?: string
  ): Promise<{ data: CertificateInfo }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/saml-apps/${appId}/certificate-info`,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },

  delete: async (
    tenantId: string,
    appId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/saml-apps/${appId}`,
      {
        method: "DELETE",
        headers: getHeaders(accessToken),
      }
    );
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new ApiResponseError(error, response.status);
    }
  },
};
