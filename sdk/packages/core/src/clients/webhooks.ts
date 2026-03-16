import type { Auth9HttpClient } from "../http-client.js";
import type {
  Webhook,
  CreateWebhookInput,
  UpdateWebhookInput,
  WebhookTestResult,
} from "../types/webhook.js";

export class WebhooksClient {
  constructor(private http: Auth9HttpClient) {}

  async list(tenantId: string): Promise<Webhook[]> {
    const result = await this.http.get<{ data: Webhook[] }>(
      `/api/v1/tenants/${tenantId}/webhooks`
    );
    return result.data;
  }

  async get(tenantId: string, webhookId: string): Promise<Webhook> {
    const result = await this.http.get<{ data: Webhook }>(
      `/api/v1/tenants/${tenantId}/webhooks/${webhookId}`
    );
    return result.data;
  }

  async create(
    tenantId: string,
    input: CreateWebhookInput
  ): Promise<Webhook> {
    const result = await this.http.post<{ data: Webhook }>(
      `/api/v1/tenants/${tenantId}/webhooks`,
      input
    );
    return result.data;
  }

  async update(
    tenantId: string,
    webhookId: string,
    input: UpdateWebhookInput
  ): Promise<Webhook> {
    const result = await this.http.put<{ data: Webhook }>(
      `/api/v1/tenants/${tenantId}/webhooks/${webhookId}`,
      input
    );
    return result.data;
  }

  async delete(tenantId: string, webhookId: string): Promise<void> {
    await this.http.delete(
      `/api/v1/tenants/${tenantId}/webhooks/${webhookId}`
    );
  }

  async test(tenantId: string, webhookId: string): Promise<WebhookTestResult> {
    const result = await this.http.post<{ data: WebhookTestResult }>(
      `/api/v1/tenants/${tenantId}/webhooks/${webhookId}/test`
    );
    return result.data;
  }

  async regenerateSecret(
    tenantId: string,
    webhookId: string
  ): Promise<Webhook> {
    const result = await this.http.post<{ data: Webhook }>(
      `/api/v1/tenants/${tenantId}/webhooks/${webhookId}/regenerate-secret`
    );
    return result.data;
  }
}
