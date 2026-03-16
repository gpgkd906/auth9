import type { Service } from "./service.js";

export interface TenantServiceInfo {
  serviceId: string;
  serviceName: string;
  enabled: boolean;
  enabledAt?: string;
}

export interface ToggleTenantServiceInput {
  serviceId: string;
  enabled: boolean;
}

export type { Service };
