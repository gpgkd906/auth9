import type { Auth9HttpClient } from "../http-client.js";
import type { Permission, CreatePermissionInput } from "../types/rbac.js";

export class PermissionsClient {
  constructor(private http: Auth9HttpClient) {}

  async list(serviceId: string): Promise<Permission[]> {
    const result = await this.http.get<{ data: Permission[] }>(
      `/api/v1/services/${serviceId}/permissions`
    );
    return result.data;
  }

  async create(input: CreatePermissionInput): Promise<Permission> {
    const result = await this.http.post<{ data: Permission }>(
      "/api/v1/permissions",
      input
    );
    return result.data;
  }

  async delete(id: string): Promise<void> {
    await this.http.delete(`/api/v1/permissions/${id}`);
  }
}
