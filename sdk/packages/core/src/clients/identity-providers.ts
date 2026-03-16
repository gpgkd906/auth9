import type { Auth9HttpClient } from "../http-client.js";
import type {
  IdentityProvider,
  CreateIdentityProviderInput,
  UpdateIdentityProviderInput,
  IdentityProviderTemplate,
  LinkedIdentity,
} from "../types/identity-provider.js";

export class IdentityProvidersClient {
  constructor(private http: Auth9HttpClient) {}

  async list(): Promise<IdentityProvider[]> {
    const result = await this.http.get<{ data: IdentityProvider[] }>(
      "/api/v1/identity-providers"
    );
    return result.data;
  }

  async get(alias: string): Promise<IdentityProvider> {
    const result = await this.http.get<{ data: IdentityProvider }>(
      `/api/v1/identity-providers/${alias}`
    );
    return result.data;
  }

  async create(input: CreateIdentityProviderInput): Promise<IdentityProvider> {
    const result = await this.http.post<{ data: IdentityProvider }>(
      "/api/v1/identity-providers",
      input
    );
    return result.data;
  }

  async update(
    alias: string,
    input: UpdateIdentityProviderInput
  ): Promise<IdentityProvider> {
    const result = await this.http.put<{ data: IdentityProvider }>(
      `/api/v1/identity-providers/${alias}`,
      input
    );
    return result.data;
  }

  async delete(alias: string): Promise<void> {
    await this.http.delete(`/api/v1/identity-providers/${alias}`);
  }

  async getTemplates(): Promise<IdentityProviderTemplate[]> {
    const result = await this.http.get<{ data: IdentityProviderTemplate[] }>(
      "/api/v1/identity-providers/templates"
    );
    return result.data;
  }

  async listMyLinkedIdentities(): Promise<LinkedIdentity[]> {
    const result = await this.http.get<{ data: LinkedIdentity[] }>(
      "/api/v1/users/me/linked-identities"
    );
    return result.data;
  }

  async unlinkIdentity(id: string): Promise<void> {
    await this.http.delete(`/api/v1/users/me/linked-identities/${id}`);
  }
}
