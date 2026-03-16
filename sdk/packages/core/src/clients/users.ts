import type { Auth9HttpClient } from "../http-client.js";
import type {
  User,
  CreateUserInput,
  UpdateUserInput,
  AddUserToTenantInput,
  UpdateUserRoleInput,
} from "../types/user.js";
import type { Tenant } from "../types/tenant.js";

export class UsersClient {
  constructor(private http: Auth9HttpClient) {}

  async list(): Promise<User[]> {
    const result = await this.http.get<{ data: User[] }>("/api/v1/users");
    return result.data;
  }

  async get(id: string): Promise<User> {
    const result = await this.http.get<{ data: User }>(
      `/api/v1/users/${id}`
    );
    return result.data;
  }

  async getMe(): Promise<User> {
    const result = await this.http.get<{ data: User }>("/api/v1/users/me");
    return result.data;
  }

  async updateMe(input: UpdateUserInput): Promise<User> {
    const result = await this.http.put<{ data: User }>(
      "/api/v1/users/me",
      input
    );
    return result.data;
  }

  async create(input: CreateUserInput): Promise<User> {
    const result = await this.http.post<{ data: User }>(
      "/api/v1/users",
      input
    );
    return result.data;
  }

  async update(id: string, input: UpdateUserInput): Promise<User> {
    const result = await this.http.put<{ data: User }>(
      `/api/v1/users/${id}`,
      input
    );
    return result.data;
  }

  async delete(id: string): Promise<void> {
    await this.http.delete(`/api/v1/users/${id}`);
  }

  async enableMfa(id: string): Promise<void> {
    await this.http.post(`/api/v1/users/${id}/mfa`);
  }

  async disableMfa(id: string): Promise<void> {
    await this.http.delete(`/api/v1/users/${id}/mfa`);
  }

  async getTenants(id: string): Promise<Tenant[]> {
    const result = await this.http.get<{ data: Tenant[] }>(
      `/api/v1/users/${id}/tenants`
    );
    return result.data;
  }

  async addToTenant(id: string, input: AddUserToTenantInput): Promise<void> {
    await this.http.post(`/api/v1/users/${id}/tenants`, input);
  }

  async removeFromTenant(userId: string, tenantId: string): Promise<void> {
    await this.http.delete(`/api/v1/users/${userId}/tenants/${tenantId}`);
  }

  async updateRoleInTenant(
    userId: string,
    tenantId: string,
    input: UpdateUserRoleInput
  ): Promise<void> {
    await this.http.put(
      `/api/v1/users/${userId}/tenants/${tenantId}`,
      input
    );
  }
}
