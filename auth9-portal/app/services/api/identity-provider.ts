import { API_BASE_URL, ApiResponseError, getHeaders, handleResponse, type ApiError } from "./client";

export interface IdentityProvider {
  alias: string;
  provider_id: string;
  display_name?: string;
  enabled: boolean;
  config: Record<string, string>;
}

export interface CreateIdentityProviderInput {
  alias: string;
  provider_id: string;
  display_name?: string;
  enabled?: boolean;
  config: Record<string, string>;
}

export interface LinkedIdentity {
  id: string;
  provider_type: string;
  provider_alias: string;
  external_user_id: string;
  external_email?: string;
  linked_at: string;
}

export interface PublicSocialProvider {
  alias: string;
  display_name?: string;
  provider_id: string;
}

export interface IdpTemplate {
  provider_id: string;
  name: string;
  description?: string;
  required_fields: string[];
  optional_fields: string[];
}

export const identityProviderApi = {
  list: async (
    accessToken?: string
  ): Promise<{ data: IdentityProvider[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/identity-providers`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  get: async (
    alias: string,
    accessToken?: string
  ): Promise<{ data: IdentityProvider }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/identity-providers/${alias}`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  create: async (
    input: CreateIdentityProviderInput,
    accessToken?: string
  ): Promise<{ data: IdentityProvider }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/identity-providers`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify(input),
      }
    );
    return handleResponse(response);
  },

  update: async (
    alias: string,
    input: Partial<CreateIdentityProviderInput>,
    accessToken?: string
  ): Promise<{ data: IdentityProvider }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/identity-providers/${alias}`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify(input),
      }
    );
    return handleResponse(response);
  },

  delete: async (alias: string, accessToken?: string): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/identity-providers/${alias}`,
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

  listMyLinkedIdentities: async (
    accessToken: string
  ): Promise<{ data: LinkedIdentity[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/me/linked-identities`,
      {
        headers: { Authorization: `Bearer ${accessToken}` },
      }
    );
    return handleResponse(response);
  },

  unlinkIdentity: async (
    id: string,
    accessToken: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/me/linked-identities/${id}`,
      {
        method: "DELETE",
        headers: { Authorization: `Bearer ${accessToken}` },
      }
    );
    return handleResponse(response);
  },

  /** List enabled social providers (public, no auth required). */
  listEnabledPublic: async (): Promise<{ data: PublicSocialProvider[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/social-login/providers`
    );
    return handleResponse(response);
  },

  templates: async (
    accessToken?: string
  ): Promise<{ data: IdpTemplate[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/identity-providers/templates`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },
};
