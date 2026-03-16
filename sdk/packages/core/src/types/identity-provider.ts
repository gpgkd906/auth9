export interface IdentityProvider {
  alias: string;
  displayName: string;
  providerId: string;
  enabled: boolean;
  config: Record<string, string>;
  createdAt: string;
}

export interface CreateIdentityProviderInput {
  alias: string;
  displayName: string;
  providerId: string;
  enabled?: boolean;
  config: Record<string, string>;
}

export interface UpdateIdentityProviderInput {
  displayName?: string;
  enabled?: boolean;
  config?: Record<string, string>;
}

export interface IdentityProviderTemplate {
  id: string;
  name: string;
  providerId: string;
  config: Record<string, string>;
}

export interface LinkedIdentity {
  id: string;
  provider: string;
  providerUserId: string;
  email?: string;
  linkedAt: string;
}
