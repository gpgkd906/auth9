import type { Auth9HttpClient } from "../http-client.js";
import type {
  ScimToken,
  ScimTokenWithValue,
  CreateScimTokenInput,
  ScimLog,
  ScimLogQuery,
  ScimGroupMapping,
} from "../types/scim.js";

export class ScimClient {
  constructor(private http: Auth9HttpClient) {}

  private basePath(tenantId: string, connectorId: string): string {
    return `/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}/scim`;
  }

  async listTokens(
    tenantId: string,
    connectorId: string
  ): Promise<ScimToken[]> {
    const result = await this.http.get<{ data: ScimToken[] }>(
      `${this.basePath(tenantId, connectorId)}/tokens`
    );
    return result.data;
  }

  async createToken(
    tenantId: string,
    connectorId: string,
    input: CreateScimTokenInput
  ): Promise<ScimTokenWithValue> {
    const result = await this.http.post<{ data: ScimTokenWithValue }>(
      `${this.basePath(tenantId, connectorId)}/tokens`,
      input
    );
    return result.data;
  }

  async revokeToken(
    tenantId: string,
    connectorId: string,
    tokenId: string
  ): Promise<void> {
    await this.http.delete(
      `${this.basePath(tenantId, connectorId)}/tokens/${tokenId}`
    );
  }

  async listLogs(
    tenantId: string,
    connectorId: string,
    options?: ScimLogQuery
  ): Promise<ScimLog[]> {
    const params: Record<string, string> = {};
    if (options?.operation) params.operation = options.operation;
    if (options?.resourceType) params.resource_type = options.resourceType;
    if (options?.status) params.status = options.status;
    if (options?.limit) params.limit = String(options.limit);

    const result = await this.http.get<{ data: ScimLog[] }>(
      `${this.basePath(tenantId, connectorId)}/logs`,
      params
    );
    return result.data;
  }

  async listGroupMappings(
    tenantId: string,
    connectorId: string
  ): Promise<ScimGroupMapping[]> {
    const result = await this.http.get<{ data: ScimGroupMapping[] }>(
      `${this.basePath(tenantId, connectorId)}/group-mappings`
    );
    return result.data;
  }

  async updateGroupMappings(
    tenantId: string,
    connectorId: string,
    mappings: ScimGroupMapping[]
  ): Promise<ScimGroupMapping[]> {
    const result = await this.http.put<{ data: ScimGroupMapping[] }>(
      `${this.basePath(tenantId, connectorId)}/group-mappings`,
      { mappings }
    );
    return result.data;
  }
}
