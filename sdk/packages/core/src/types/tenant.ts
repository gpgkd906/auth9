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
