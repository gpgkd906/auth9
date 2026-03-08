export interface TenantInfo {
  id: string;
  name: string;
  slug: string;
  logo_url?: string;
  status: string;
}

export interface UserTenant {
  id: string;
  tenant_id: string;
  user_id: string;
  role_in_tenant: string;
  joined_at: string;
  tenant: TenantInfo;
}
