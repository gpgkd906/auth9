import {
  API_BASE_URL,
  ApiResponseError,
  getHeaders,
  handleResponse,
  type ApiError,
  type PaginatedResponse,
} from "./client";

export interface User {
  id: string;
  email: string;
  display_name?: string;
  avatar_url?: string;
  mfa_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateUserInput {
  email: string;
  display_name?: string;
  avatar_url?: string;
}

// Tenant user with tenant details (for org switcher)
export interface TenantUserWithTenant {
  id: string;
  tenant_id: string;
  user_id: string;
  role_in_tenant: string;
  joined_at: string;
  tenant: {
    id: string;
    name: string;
    slug: string;
    domain?: string;
    logo_url?: string;
    status: string;
  };
}

export const userApi = {
  list: async (
    page = 1,
    perPage = 20,
    search?: string,
    accessToken?: string
  ): Promise<PaginatedResponse<User>> => {
    let url = `${API_BASE_URL}/api/v1/users?page=${page}&per_page=${perPage}`;
    if (search) url += `&search=${encodeURIComponent(search)}`;
    const response = await fetch(url, { headers: getHeaders(accessToken) });
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`);
    return handleResponse(response);
  },

  create: async (
    input: CreateUserInput & { password?: string; tenant_id?: string },
    accessToken?: string
  ): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (
    id: string,
    input: Partial<CreateUserInput>,
    accessToken?: string
  ): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new ApiResponseError(error, response.status);
    }
  },

  getTenants: async (
    userId: string,
    accessToken?: string
  ): Promise<{
    data: {
      id: string;
      tenant_id: string;
      user_id: string;
      role_in_tenant: string;
      joined_at: string;
      tenant: {
        id: string;
        name: string;
        slug: string;
        logo_url?: string;
        status: string;
      };
    }[];
  }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  addToTenant: async (
    userId: string,
    tenantId: string,
    roleInTenant: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify({
          tenant_id: tenantId,
          role_in_tenant: roleInTenant,
        }),
      }
    );
    return handleResponse(response);
  },

  removeFromTenant: async (
    userId: string,
    tenantId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}`,
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

  getMyTenants: async (
    accessToken?: string,
    serviceId?: string
  ): Promise<{ data: TenantUserWithTenant[] }> => {
    let url = `${API_BASE_URL}/api/v1/users/me/tenants`;
    if (serviceId) {
      url += `?service_id=${encodeURIComponent(serviceId)}`;
    }
    const response = await fetch(url, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  getMe: async (accessToken?: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  updateMe: async (
    input: Partial<CreateUserInput>,
    accessToken?: string
  ): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  enableMfa: async (
    id: string,
    confirmPassword: string,
    accessToken?: string
  ): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}/mfa`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ confirm_password: confirmPassword }),
    });
    return handleResponse(response);
  },

  disableMfa: async (
    id: string,
    confirmPassword: string,
    accessToken?: string
  ): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}/mfa`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ confirm_password: confirmPassword }),
    });
    return handleResponse(response);
  },

  updateRoleInTenant: async (
    userId: string,
    tenantId: string,
    roleInTenant: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ role_in_tenant: roleInTenant }),
      }
    );
    return handleResponse(response);
  },
};
