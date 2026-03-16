import type { Auth9HttpClient } from "../http-client.js";
import type {
  AssignRolesInput,
  UserRolesInTenant,
  Role,
} from "../types/rbac.js";

export class RbacClient {
  constructor(private http: Auth9HttpClient) {}

  async assignRoles(input: AssignRolesInput): Promise<void> {
    await this.http.post("/api/v1/rbac/assign", input);
  }

  async getUserRoles(
    userId: string,
    tenantId: string
  ): Promise<UserRolesInTenant> {
    const result = await this.http.get<{ data: UserRolesInTenant }>(
      `/api/v1/users/${userId}/tenants/${tenantId}/roles`
    );
    return result.data;
  }

  async getUserAssignedRoles(
    userId: string,
    tenantId: string
  ): Promise<Role[]> {
    const result = await this.http.get<{ data: Role[] }>(
      `/api/v1/users/${userId}/tenants/${tenantId}/assigned-roles`
    );
    return result.data;
  }

  async unassignRole(
    userId: string,
    tenantId: string,
    roleId: string
  ): Promise<void> {
    await this.http.delete(
      `/api/v1/users/${userId}/tenants/${tenantId}/roles/${roleId}`
    );
  }
}
