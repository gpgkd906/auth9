import type { Auth9HttpClient } from "../http-client.js";
import type {
  Role,
  RoleWithPermissions,
  CreateRoleInput,
  UpdateRoleInput,
} from "../types/rbac.js";

export class RolesClient {
  constructor(private http: Auth9HttpClient) {}

  async list(serviceId: string): Promise<Role[]> {
    const result = await this.http.get<{ data: Role[] }>(
      `/api/v1/services/${serviceId}/roles`
    );
    return result.data;
  }

  async get(id: string): Promise<RoleWithPermissions> {
    const result = await this.http.get<{ data: RoleWithPermissions }>(
      `/api/v1/roles/${id}`
    );
    return result.data;
  }

  async create(input: CreateRoleInput): Promise<Role> {
    const result = await this.http.post<{ data: Role }>(
      "/api/v1/roles",
      input
    );
    return result.data;
  }

  async update(id: string, input: UpdateRoleInput): Promise<Role> {
    const result = await this.http.put<{ data: Role }>(
      `/api/v1/roles/${id}`,
      input
    );
    return result.data;
  }

  async delete(id: string): Promise<void> {
    await this.http.delete(`/api/v1/roles/${id}`);
  }

  async assignPermission(
    roleId: string,
    permissionId: string
  ): Promise<void> {
    await this.http.post(`/api/v1/roles/${roleId}/permissions`, {
      permissionId,
    });
  }

  async removePermission(
    roleId: string,
    permissionId: string
  ): Promise<void> {
    await this.http.delete(
      `/api/v1/roles/${roleId}/permissions/${permissionId}`
    );
  }
}
