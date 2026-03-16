export interface SSOConnector {
  id: string;
  tenantId: string;
  name: string;
  protocol: "saml" | "oidc";
  domains: string[];
  config: Record<string, string>;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateSSOConnectorInput {
  name: string;
  protocol: "saml" | "oidc";
  domains: string[];
  config: Record<string, string>;
  enabled?: boolean;
}

export interface UpdateSSOConnectorInput {
  name?: string;
  domains?: string[];
  config?: Record<string, string>;
  enabled?: boolean;
}

export interface SSOTestResult {
  success: boolean;
  message?: string;
  error?: string;
}
