export interface SamlApplication {
  id: string;
  tenantId: string;
  name: string;
  entityId: string;
  acsUrl: string;
  enabled: boolean;
  config: Record<string, string>;
  createdAt: string;
  updatedAt: string;
}

export interface CreateSamlApplicationInput {
  name: string;
  entityId: string;
  acsUrl: string;
  enabled?: boolean;
  config?: Record<string, string>;
}

export interface UpdateSamlApplicationInput {
  name?: string;
  entityId?: string;
  acsUrl?: string;
  enabled?: boolean;
  config?: Record<string, string>;
}

export interface SamlCertificateInfo {
  subject: string;
  issuer: string;
  validFrom: string;
  validTo: string;
  fingerprint: string;
  serialNumber: string;
}
