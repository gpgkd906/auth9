export interface Service {
  id: string;
  tenantId?: string;
  name: string;
  baseUrl?: string;
  redirectUris: string[];
  logoutUris: string[];
  status: "active" | "inactive";
  createdAt: string;
  updatedAt: string;
}

export interface CreateServiceInput {
  name: string;
  clientId?: string;
  baseUrl?: string;
  redirectUris?: string[];
  logoutUris?: string[];
  tenantId?: string;
}

export interface Client {
  id: string;
  serviceId: string;
  clientId: string;
  name?: string;
  createdAt: string;
}

export interface ClientWithSecret extends Client {
  clientSecret: string;
}

export interface CreateClientInput {
  name?: string;
}

export interface UpdateServiceInput {
  name?: string;
  baseUrl?: string;
  redirectUris?: string[];
  logoutUris?: string[];
  status?: "active" | "inactive";
}

export interface ServiceIntegration {
  serviceId: string;
  clientId: string;
  issuerUrl: string;
  authorizationEndpoint: string;
  tokenEndpoint: string;
  userinfoEndpoint: string;
  jwksUri: string;
}

export interface ServiceWithStatus {
  id: string;
  name: string;
  baseUrl?: string;
  status: string;
  enabled: boolean;
}
