export interface Tenant {
  id: string;
  name: string;
  slug: string;
  logoUrl?: string;
  settings: Record<string, unknown>;
  status: "active" | "inactive" | "suspended";
  createdAt: string;
  updatedAt: string;
}

export interface CreateTenantInput {
  name: string;
  slug: string;
  logoUrl?: string;
  settings?: Record<string, unknown>;
}

export interface UpdateTenantInput {
  name?: string;
  slug?: string;
  logoUrl?: string;
  settings?: Record<string, unknown>;
  status?: "active" | "inactive" | "suspended";
}

export interface MaliciousIpBlacklistEntry {
  id: string;
  tenantId: string;
  ipAddress: string;
  reason?: string;
  createdBy?: string;
  createdAt: string;
  updatedAt: string;
}

export interface UpdateMaliciousIpBlacklistInput {
  entries: { ipAddress: string; reason?: string }[];
}

export interface TenantUser {
  id: string;
  tenantId: string;
  userId: string;
  roleInTenant: string;
  joinedAt: string;
  tenant: {
    id: string;
    name: string;
    slug: string;
    logoUrl?: string;
    status: string;
  };
}
