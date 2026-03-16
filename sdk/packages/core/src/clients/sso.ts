import type { Auth9HttpClient } from "../http-client.js";
import type {
  SSOConnector,
  CreateSSOConnectorInput,
  UpdateSSOConnectorInput,
  SSOTestResult,
} from "../types/sso.js";

export class SsoClient {
  constructor(private http: Auth9HttpClient) {}

  async listConnectors(tenantId: string): Promise<SSOConnector[]> {
    const result = await this.http.get<{ data: SSOConnector[] }>(
      `/api/v1/tenants/${tenantId}/sso/connectors`
    );
    return result.data;
  }

  async createConnector(
    tenantId: string,
    input: CreateSSOConnectorInput
  ): Promise<SSOConnector> {
    const result = await this.http.post<{ data: SSOConnector }>(
      `/api/v1/tenants/${tenantId}/sso/connectors`,
      input
    );
    return result.data;
  }

  async updateConnector(
    tenantId: string,
    connectorId: string,
    input: UpdateSSOConnectorInput
  ): Promise<SSOConnector> {
    const result = await this.http.put<{ data: SSOConnector }>(
      `/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
      input
    );
    return result.data;
  }

  async deleteConnector(
    tenantId: string,
    connectorId: string
  ): Promise<void> {
    await this.http.delete(
      `/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`
    );
  }

  async testConnector(
    tenantId: string,
    connectorId: string
  ): Promise<SSOTestResult> {
    const result = await this.http.post<{ data: SSOTestResult }>(
      `/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}/test`
    );
    return result.data;
  }
}
