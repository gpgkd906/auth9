import type { Auth9HttpClient } from "../http-client.js";
import type {
  TenantServiceInfo,
  ToggleTenantServiceInput,
} from "../types/tenant-service.js";
import type { Service } from "../types/service.js";

export class TenantServicesClient {
  constructor(private http: Auth9HttpClient) {}

  async list(tenantId: string): Promise<TenantServiceInfo[]> {
    const result = await this.http.get<{ data: TenantServiceInfo[] }>(
      `/api/v1/tenants/${tenantId}/services`
    );
    return result.data;
  }

  async toggle(
    tenantId: string,
    input: ToggleTenantServiceInput
  ): Promise<void> {
    await this.http.post(
      `/api/v1/tenants/${tenantId}/services`,
      input
    );
  }

  async getEnabled(tenantId: string): Promise<Service[]> {
    const result = await this.http.get<{ data: Service[] }>(
      `/api/v1/tenants/${tenantId}/services/enabled`
    );
    return result.data;
  }
}
