import type { Auth9HttpClient } from "../http-client.js";
import type {
  Service,
  CreateServiceInput,
  UpdateServiceInput,
  ServiceIntegration,
  Client,
  ClientWithSecret,
  CreateClientInput,
} from "../types/service.js";

export class ServicesClient {
  constructor(private http: Auth9HttpClient) {}

  async list(): Promise<Service[]> {
    const result = await this.http.get<{ data: Service[] }>(
      "/api/v1/services"
    );
    return result.data;
  }

  async get(id: string): Promise<Service> {
    const result = await this.http.get<{ data: Service }>(
      `/api/v1/services/${id}`
    );
    return result.data;
  }

  async create(input: CreateServiceInput): Promise<Service> {
    const result = await this.http.post<{ data: Service }>(
      "/api/v1/services",
      input
    );
    return result.data;
  }

  async update(id: string, input: UpdateServiceInput): Promise<Service> {
    const result = await this.http.put<{ data: Service }>(
      `/api/v1/services/${id}`,
      input
    );
    return result.data;
  }

  async delete(id: string): Promise<void> {
    await this.http.delete(`/api/v1/services/${id}`);
  }

  async getIntegrationInfo(id: string): Promise<ServiceIntegration> {
    const result = await this.http.get<{ data: ServiceIntegration }>(
      `/api/v1/services/${id}/integration`
    );
    return result.data;
  }

  async listClients(serviceId: string): Promise<Client[]> {
    const result = await this.http.get<{ data: Client[] }>(
      `/api/v1/services/${serviceId}/clients`
    );
    return result.data;
  }

  async createClient(
    serviceId: string,
    input: CreateClientInput
  ): Promise<ClientWithSecret> {
    const result = await this.http.post<{ data: ClientWithSecret }>(
      `/api/v1/services/${serviceId}/clients`,
      input
    );
    return result.data;
  }

  async deleteClient(serviceId: string, clientId: string): Promise<void> {
    await this.http.delete(
      `/api/v1/services/${serviceId}/clients/${clientId}`
    );
  }

  async regenerateClientSecret(
    serviceId: string,
    clientId: string
  ): Promise<ClientWithSecret> {
    const result = await this.http.post<{ data: ClientWithSecret }>(
      `/api/v1/services/${serviceId}/clients/${clientId}/regenerate-secret`
    );
    return result.data;
  }
}
