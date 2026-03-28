import {
  API_BASE_URL,
  ApiResponseError,
  getHeaders,
  handleResponse,
  type ApiError,
} from "./client";

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
  created_at?: string;
}

export interface CreatePermissionInput {
  service_id: string;
  code: string;
  name: string;
  description?: string;
}

export interface RoleWithPermissions extends Role {
  permissions: Permission[];
}

export interface CreateRoleInput {
  name: string;
  description?: string;
  parent_role_id?: string;
}

export interface AssignRolesInput {
  user_id: string;
  tenant_id: string;
  role_ids: string[];
  service_id?: string;
}

export interface UserRolesInTenant {
  user_id: string;
  tenant_id: string;
  roles: string[]; // Role names
  permissions: string[];
}

export const rbacApi = {
  listRoles: async (
    serviceId: string,
    accessToken?: string
  ): Promise<{ data: Role[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/services/${serviceId}/roles`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  createRole: async (
    serviceId: string,
    input: CreateRoleInput,
    accessToken?: string
  ): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ ...input, service_id: serviceId }),
    });
    return handleResponse(response);
  },

  updateRole: async (
    serviceId: string,
    roleId: string,
    input: Partial<CreateRoleInput>,
    accessToken?: string
  ): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deleteRole: async (
    serviceId: string,
    roleId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new ApiResponseError(error, response.status);
    }
  },

  listPermissions: async (
    serviceId: string,
    accessToken?: string
  ): Promise<{ data: Permission[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/services/${serviceId}/permissions`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  createPermission: async (
    input: CreatePermissionInput,
    accessToken?: string
  ): Promise<{ data: Permission }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/permissions`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deletePermission: async (
    permissionId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/permissions/${permissionId}`,
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

  getRole: async (
    roleId: string,
    accessToken?: string
  ): Promise<{ data: RoleWithPermissions }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  assignPermissionToRole: async (
    roleId: string,
    permissionId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/roles/${roleId}/permissions`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ permission_id: permissionId }),
      }
    );
    return handleResponse(response);
  },

  removePermissionFromRole: async (
    roleId: string,
    permissionId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/roles/${roleId}/permissions/${permissionId}`,
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

  assignRoles: async (
    input: AssignRolesInput,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/rbac/assign`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  getUserRoles: async (
    userId: string,
    tenantId: string,
    accessToken?: string
  ): Promise<{ data: UserRolesInTenant }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/roles`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  getUserAssignedRoles: async (
    userId: string,
    tenantId: string,
    accessToken?: string
  ): Promise<{ data: Role[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/assigned-roles`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  unassignRole: async (
    userId: string,
    tenantId: string,
    roleId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/roles/${roleId}`,
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
