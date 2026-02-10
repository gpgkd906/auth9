export interface Webhook {
  id: string;
  tenantId: string;
  name: string;
  url: string;
  secret?: string;
  events: string[];
  enabled: boolean;
  lastTriggeredAt?: string;
  failureCount: number;
  createdAt: string;
}

export interface CreateWebhookInput {
  name: string;
  url: string;
  secret?: string;
  events: string[];
  enabled?: boolean;
}

export interface WebhookTestResult {
  success: boolean;
  statusCode?: number;
  responseTimeMs?: number;
  error?: string;
}
