import type { Auth9HttpClient } from "../http-client.js";
import type {
  Tenant,
  CreateTenantInput,
  UpdateTenantInput,
  MaliciousIpBlacklistEntry,
  UpdateMaliciousIpBlacklistInput,
} from "../types/tenant.js";
import type { User } from "../types/user.js";

export class TenantsClient {
  constructor(private http: Auth9HttpClient) {}

  async list(): Promise<Tenant[]> {
    const result = await this.http.get<{ data: Tenant[] }>("/api/v1/tenants");
    return result.data;
  }

  async get(id: string): Promise<Tenant> {
    const result = await this.http.get<{ data: Tenant }>(
      `/api/v1/tenants/${id}`
    );
    return result.data;
  }

  async create(input: CreateTenantInput): Promise<Tenant> {
    const result = await this.http.post<{ data: Tenant }>(
      "/api/v1/tenants",
      input
    );
    return result.data;
  }

  async update(id: string, input: UpdateTenantInput): Promise<Tenant> {
    const result = await this.http.put<{ data: Tenant }>(
      `/api/v1/tenants/${id}`,
      input
    );
    return result.data;
  }

  async delete(id: string): Promise<void> {
    await this.http.delete(`/api/v1/tenants/${id}`);
  }

  async listUsers(tenantId: string): Promise<User[]> {
    const result = await this.http.get<{ data: User[] }>(
      `/api/v1/tenants/${tenantId}/users`
    );
    return result.data;
  }

  async getMaliciousIpBlacklist(
    tenantId: string
  ): Promise<MaliciousIpBlacklistEntry[]> {
    const result = await this.http.get<{ data: MaliciousIpBlacklistEntry[] }>(
      `/api/v1/tenants/${tenantId}/security/malicious-ip-blacklist`
    );
    return result.data;
  }

  async updateMaliciousIpBlacklist(
    tenantId: string,
    input: UpdateMaliciousIpBlacklistInput
  ): Promise<MaliciousIpBlacklistEntry[]> {
    const result = await this.http.put<{ data: MaliciousIpBlacklistEntry[] }>(
      `/api/v1/tenants/${tenantId}/security/malicious-ip-blacklist`,
      input
    );
    return result.data;
  }
}
