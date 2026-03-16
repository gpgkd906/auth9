import type { Auth9HttpClient } from "../http-client.js";
import type {
  SamlApplication,
  CreateSamlApplicationInput,
  UpdateSamlApplicationInput,
  SamlCertificateInfo,
} from "../types/saml.js";

export class SamlClient {
  constructor(private http: Auth9HttpClient) {}

  async list(tenantId: string): Promise<SamlApplication[]> {
    const result = await this.http.get<{ data: SamlApplication[] }>(
      `/api/v1/tenants/${tenantId}/saml-apps`
    );
    return result.data;
  }

  async get(tenantId: string, appId: string): Promise<SamlApplication> {
    const result = await this.http.get<{ data: SamlApplication }>(
      `/api/v1/tenants/${tenantId}/saml-apps/${appId}`
    );
    return result.data;
  }

  async create(
    tenantId: string,
    input: CreateSamlApplicationInput
  ): Promise<SamlApplication> {
    const result = await this.http.post<{ data: SamlApplication }>(
      `/api/v1/tenants/${tenantId}/saml-apps`,
      input
    );
    return result.data;
  }

  async update(
    tenantId: string,
    appId: string,
    input: UpdateSamlApplicationInput
  ): Promise<SamlApplication> {
    const result = await this.http.put<{ data: SamlApplication }>(
      `/api/v1/tenants/${tenantId}/saml-apps/${appId}`,
      input
    );
    return result.data;
  }

  async delete(tenantId: string, appId: string): Promise<void> {
    await this.http.delete(
      `/api/v1/tenants/${tenantId}/saml-apps/${appId}`
    );
  }

  async getMetadata(tenantId: string, appId: string): Promise<string> {
    const result = await this.http.get<{ data: string }>(
      `/api/v1/tenants/${tenantId}/saml-apps/${appId}/metadata`
    );
    return result.data;
  }

  async getCertificate(tenantId: string, appId: string): Promise<string> {
    const result = await this.http.get<{ data: string }>(
      `/api/v1/tenants/${tenantId}/saml-apps/${appId}/certificate`
    );
    return result.data;
  }

  async getCertificateInfo(
    tenantId: string,
    appId: string
  ): Promise<SamlCertificateInfo> {
    const result = await this.http.get<{ data: SamlCertificateInfo }>(
      `/api/v1/tenants/${tenantId}/saml-apps/${appId}/certificate-info`
    );
    return result.data;
  }
}
