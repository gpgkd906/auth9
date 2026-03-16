import type { Auth9HttpClient } from "../http-client.js";
import type {
  AbacPolicy,
  CreateAbacPolicyInput,
  UpdateAbacPolicyInput,
  SimulateAbacInput,
  AbacSimulationResult,
} from "../types/abac.js";

export class AbacClient {
  constructor(private http: Auth9HttpClient) {}

  async listPolicies(tenantId: string): Promise<AbacPolicy[]> {
    const result = await this.http.get<{ data: AbacPolicy[] }>(
      `/api/v1/tenants/${tenantId}/abac/policies`
    );
    return result.data;
  }

  async createPolicy(
    tenantId: string,
    input: CreateAbacPolicyInput
  ): Promise<AbacPolicy> {
    const result = await this.http.post<{ data: AbacPolicy }>(
      `/api/v1/tenants/${tenantId}/abac/policies`,
      input
    );
    return result.data;
  }

  async updatePolicy(
    tenantId: string,
    versionId: string,
    input: UpdateAbacPolicyInput
  ): Promise<AbacPolicy> {
    const result = await this.http.put<{ data: AbacPolicy }>(
      `/api/v1/tenants/${tenantId}/abac/policies/${versionId}`,
      input
    );
    return result.data;
  }

  async publishPolicy(
    tenantId: string,
    versionId: string
  ): Promise<AbacPolicy> {
    const result = await this.http.post<{ data: AbacPolicy }>(
      `/api/v1/tenants/${tenantId}/abac/policies/${versionId}/publish`
    );
    return result.data;
  }

  async rollbackPolicy(
    tenantId: string,
    versionId: string
  ): Promise<AbacPolicy> {
    const result = await this.http.post<{ data: AbacPolicy }>(
      `/api/v1/tenants/${tenantId}/abac/policies/${versionId}/rollback`
    );
    return result.data;
  }

  async simulate(
    tenantId: string,
    input: SimulateAbacInput
  ): Promise<AbacSimulationResult> {
    const result = await this.http.post<{ data: AbacSimulationResult }>(
      `/api/v1/tenants/${tenantId}/abac/simulate`,
      input
    );
    return result.data;
  }
}
