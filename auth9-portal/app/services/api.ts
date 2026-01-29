// API client for auth9-core

const API_BASE_URL = process.env.AUTH9_CORE_URL || "http://localhost:8080";

export interface ApiError {
  error: string;
  message: string;
  details?: unknown;
}

export interface PaginatedResponse<T> {
  data: T[];
  pagination: {
    page: number;
    per_page: number;
    total: number;
    total_pages: number;
  };
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: "unknown",
      message: response.statusText,
    }));
    throw new Error(error.message);
  }
  return response.json();
}

// Tenant API
export interface Tenant {
  id: string;
  name: string;
  slug: string;
  logo_url?: string;
  settings: Record<string, unknown>;
  status: "active" | "inactive" | "suspended";
  created_at: string;
  updated_at: string;
}

export interface CreateTenantInput {
  name: string;
  slug: string;
  logo_url?: string;
  settings?: Record<string, unknown>;
}

export const tenantApi = {
  list: async (page = 1, perPage = 20): Promise<PaginatedResponse<Tenant>> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants?page=${page}&per_page=${perPage}`
    );
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateTenantInput): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateTenantInput>): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },
};

// User API
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

export const userApi = {
  list: async (page = 1, perPage = 20): Promise<PaginatedResponse<User>> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users?page=${page}&per_page=${perPage}`
    );
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateUserInput & { password?: string }): Promise<{ data: User }> => {
    const { password, ...user } = input;
    const response = await fetch(`${API_BASE_URL}/api/v1/users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ user, password }),
    });
    return handleResponse(response);
  },
};

// Service API
export interface Service {
  id: string;
  tenant_id?: string;
  name: string;
  client_id: string;
  base_url?: string;
  redirect_uris: string[];
  logout_uris: string[];
  status: "active" | "inactive";
  created_at: string;
  updated_at: string;
}

export const serviceApi = {
  list: async (tenantId?: string, page = 1, perPage = 20): Promise<PaginatedResponse<Service>> => {
    let url = `${API_BASE_URL}/api/v1/services?page=${page}&per_page=${perPage}`;
    if (tenantId) url += `&tenant_id=${tenantId}`;
    const response = await fetch(url);
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateServiceInput): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateServiceInput>): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },
};

export interface CreateServiceInput {
  name: string;
  client_id?: string;
  base_url?: string;
  redirect_uris?: string[];
  logout_uris?: string[];
  tenant_id?: string;
}

export interface Role {
  id: string;
  service_id: string;
  name: string;
  description?: string;
  parent_role_id?: string;
  created_at: string;
  updated_at: string;
}

export interface Permission {
  id: string;
  service_id: string;
  code: string;
  name: string;
  description?: string;
}

export const rbacApi = {
  listRoles: async (serviceId: string): Promise<{ data: Role[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/roles`);
    return handleResponse(response);
  },

  createRole: async (serviceId: string, input: CreateRoleInput): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...input, service_id: serviceId }),
    });
    return handleResponse(response);
  },

  updateRole: async (serviceId: string, roleId: string, input: Partial<CreateRoleInput>): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deleteRole: async (serviceId: string, roleId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  listPermissions: async (serviceId: string): Promise<{ data: Permission[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/permissions`);
    return handleResponse(response);
  },
};

export interface CreateRoleInput {
  name: string;
  description?: string;
  parent_role_id?: string;
}

export interface AuditLog {
  id: number;
  actor_id?: string;
  action: string;
  resource_type: string;
  resource_id?: string;
  old_value?: unknown;
  new_value?: unknown;
  ip_address?: string;
  created_at: string;
}

export const auditApi = {
  list: async (page = 1, perPage = 50): Promise<PaginatedResponse<AuditLog>> => {
    const offset = (page - 1) * perPage;
    const response = await fetch(
      `${API_BASE_URL}/api/v1/audit-logs?limit=${perPage}&offset=${offset}`
    );
    return handleResponse(response);
  },
};
