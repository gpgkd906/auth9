// API client for auth9-core

const API_BASE_URL = process.env.AUTH9_CORE_URL || "http://localhost:8080";

export interface ApiError {
  error: string;
  message: string;
  details?: unknown;
}

export interface PaginatedResponse<T> {
  data: T[];
  pagination: {
    page: number;
    per_page: number;
    total: number;
    total_pages: number;
  };
}

async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const error: ApiError = await response.json().catch(() => ({
      error: "unknown",
      message: response.statusText,
    }));
    throw new Error(error.message);
  }
  return response.json();
}

function getHeaders(accessToken?: string): HeadersInit {
  const headers: HeadersInit = { "Content-Type": "application/json" };
  if (accessToken) {
    headers["Authorization"] = `Bearer ${accessToken}`;
  }
  return headers;
}

// Tenant API
export interface Tenant {
  id: string;
  name: string;
  slug: string;
  logo_url?: string;
  settings: Record<string, unknown>;
  status: "active" | "inactive" | "suspended";
  created_at: string;
  updated_at: string;
}

export interface CreateTenantInput {
  name: string;
  slug: string;
  logo_url?: string;
  settings?: Record<string, unknown>;
}

export const tenantApi = {
  list: async (page = 1, perPage = 20, search?: string, accessToken?: string): Promise<PaginatedResponse<Tenant>> => {
    let url = `${API_BASE_URL}/api/v1/tenants?page=${page}&per_page=${perPage}`;
    if (search) url += `&search=${encodeURIComponent(search)}`;
    const response = await fetch(url, { headers: getHeaders(accessToken) });
    return handleResponse(response);
  },

  get: async (id: string, accessToken?: string): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  create: async (input: CreateTenantInput, accessToken?: string): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateTenantInput>, accessToken?: string): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      method: "DELETE",
      headers: { ...getHeaders(accessToken), "X-Confirm-Destructive": "true" },
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },
};

// User API
export interface User {
  id: string;
  email: string;
  display_name?: string;
  avatar_url?: string;
  mfa_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateUserInput {
  email: string;
  display_name?: string;
  avatar_url?: string;
}

export const userApi = {
  list: async (page = 1, perPage = 20, search?: string, accessToken?: string): Promise<PaginatedResponse<User>> => {
    let url = `${API_BASE_URL}/api/v1/users?page=${page}&per_page=${perPage}`;
    if (search) url += `&search=${encodeURIComponent(search)}`;
    const response = await fetch(
      url,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateUserInput & { password?: string; tenant_id?: string }, accessToken?: string): Promise<{ data: User }> => {
    const { password, tenant_id, ...user } = input;
    const response = await fetch(`${API_BASE_URL}/api/v1/users`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ ...user, password, tenant_id }),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateUserInput>, accessToken?: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  getTenants: async (userId: string, accessToken?: string): Promise<{ data: { id: string; tenant_id: string; user_id: string; role_in_tenant: string; joined_at: string; tenant: { id: string; name: string; slug: string; logo_url?: string; status: string } }[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  addToTenant: async (userId: string, tenantId: string, roleInTenant: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ tenant_id: tenantId, role_in_tenant: roleInTenant }),
    });
    return handleResponse(response);
  },

  removeFromTenant: async (userId: string, tenantId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  getMe: async (accessToken?: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  updateMe: async (input: Partial<CreateUserInput>, accessToken?: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  enableMfa: async (id: string, accessToken?: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}/mfa`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  disableMfa: async (id: string, accessToken?: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}/mfa`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// Tenant User API
export const tenantUserApi = {
  list: async (tenantId: string, accessToken?: string): Promise<{ data: User[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/users`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// Service API
export interface Service {
  id: string;
  tenant_id?: string;
  name: string;
  base_url?: string;
  redirect_uris: string[];
  logout_uris: string[];
  status: "active" | "inactive";
  created_at: string;
  updated_at: string;
}

export interface Client {
  id: string;
  service_id: string;
  client_id: string;
  name?: string;
  created_at: string;
}

// Note: Backend uses #[serde(flatten)] so Client fields are flattened
export interface ClientWithSecret extends Client {
  client_secret: string;
}

export interface CreateClientInput {
  name?: string;
}

export const serviceApi = {
  list: async (tenantId?: string, page = 1, perPage = 20, accessToken?: string): Promise<PaginatedResponse<Service>> => {
    let url = `${API_BASE_URL}/api/v1/services?page=${page}&per_page=${perPage}`;
    if (tenantId) url += `&tenant_id=${tenantId}`;
    const response = await fetch(url, { headers: getHeaders(accessToken) });
    return handleResponse(response);
  },

  get: async (id: string, accessToken?: string): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  // Note: Backend uses #[serde(flatten)] on ServiceWithClient, so Service fields are at root level.
  // Some code paths can return service data without client details.
  create: async (input: CreateServiceInput, accessToken?: string): Promise<{ data: Service & { client?: ClientWithSecret } }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateServiceInput>, accessToken?: string): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  listClients: async (serviceId: string, accessToken?: string): Promise<{ data: Client[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  createClient: async (serviceId: string, input: CreateClientInput, accessToken?: string): Promise<{ data: ClientWithSecret }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deleteClient: async (serviceId: string, clientId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients/${clientId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  regenerateClientSecret: async (serviceId: string, clientId: string, accessToken?: string): Promise<{ data: { client_id: string; client_secret: string } }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients/${clientId}/regenerate-secret`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  getIntegration: async (serviceId: string, accessToken?: string): Promise<{ data: ServiceIntegrationInfo }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/integration`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// Integration info types
export interface ServiceIntegrationInfo {
  service: {
    id: string;
    name: string;
    base_url?: string;
    redirect_uris: string[];
    logout_uris: string[];
  };
  clients: ClientIntegrationInfo[];
  endpoints: EndpointInfo;
  grpc: GrpcInfo;
  environment_variables: EnvVar[];
}

export interface ClientIntegrationInfo {
  client_id: string;
  name?: string;
  public_client: boolean;
  client_secret?: string;
  created_at: string;
}

export interface EndpointInfo {
  auth9_domain: string;
  auth9_public_url: string;
  authorize: string;
  token: string;
  callback: string;
  logout: string;
  userinfo: string;
  openid_configuration: string;
  jwks: string;
}

export interface GrpcInfo {
  address: string;
  auth_mode: string;
}

export interface EnvVar {
  key: string;
  value: string;
  description: string;
}

export interface CreateServiceInput {
  name: string;
  client_id?: string;
  base_url?: string;
  redirect_uris?: string[];
  logout_uris?: string[];
  tenant_id?: string;
}

export interface Role {
  id: string;
  service_id: string;
  name: string;
  description?: string;
  parent_role_id?: string;
  created_at: string;
  updated_at: string;
}

export interface Permission {
  id: string;
  service_id: string;
  code: string;
  name: string;
  description?: string;
  created_at?: string;
}

export interface CreatePermissionInput {
  service_id: string;
  code: string;
  name: string;
  description?: string;
}

export interface RoleWithPermissions extends Role {
  permissions: Permission[];
}

export const rbacApi = {
  listRoles: async (serviceId: string, accessToken?: string): Promise<{ data: Role[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/roles`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  createRole: async (serviceId: string, input: CreateRoleInput, accessToken?: string): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ ...input, service_id: serviceId }),
    });
    return handleResponse(response);
  },

  updateRole: async (serviceId: string, roleId: string, input: Partial<CreateRoleInput>, accessToken?: string): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deleteRole: async (serviceId: string, roleId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  listPermissions: async (serviceId: string, accessToken?: string): Promise<{ data: Permission[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/permissions`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  createPermission: async (input: CreatePermissionInput, accessToken?: string): Promise<{ data: Permission }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/permissions`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deletePermission: async (permissionId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/permissions/${permissionId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  getRole: async (roleId: string, accessToken?: string): Promise<{ data: RoleWithPermissions }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  assignPermissionToRole: async (roleId: string, permissionId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}/permissions`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ permission_id: permissionId }),
    });
    return handleResponse(response);
  },

  removePermissionFromRole: async (roleId: string, permissionId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}/permissions/${permissionId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  assignRoles: async (input: AssignRolesInput, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/rbac/assign`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  getUserRoles: async (userId: string, tenantId: string, accessToken?: string): Promise<{ data: UserRolesInTenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/roles`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  getUserAssignedRoles: async (userId: string, tenantId: string, accessToken?: string): Promise<{ data: Role[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/assigned-roles`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  unassignRole: async (userId: string, tenantId: string, roleId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/roles/${roleId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },
};

export interface CreateRoleInput {
  name: string;
  description?: string;
  parent_role_id?: string;
}

export interface AssignRolesInput {
  user_id: string;
  tenant_id: string;
  role_ids: string[];
}

export interface UserRolesInTenant {
  user_id: string;
  tenant_id: string;
  roles: string[]; // Role names
  permissions: string[];
}

export interface AuditLog {
  id: number;
  actor_id?: string;
  actor_email?: string;
  actor_display_name?: string;
  action: string;
  resource_type: string;
  resource_id?: string;
  old_value?: unknown;
  new_value?: unknown;
  ip_address?: string;
  created_at: string;
}

export const auditApi = {
  list: async (page = 1, perPage = 50, accessToken?: string): Promise<PaginatedResponse<AuditLog>> => {
    const offset = (page - 1) * perPage;
    const response = await fetch(
      `${API_BASE_URL}/api/v1/audit-logs?limit=${perPage}&offset=${offset}`,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },
};

// Email Provider Configuration Types
export interface SmtpConfig {
  type: "smtp";
  host: string;
  port: number;
  username?: string;
  password?: string;
  use_tls: boolean;
  from_email: string;
  from_name?: string;
}

export interface SesConfig {
  type: "ses";
  region: string;
  access_key_id?: string;
  secret_access_key?: string;
  from_email: string;
  from_name?: string;
  configuration_set?: string;
}

export interface OracleEmailConfig {
  type: "oracle";
  smtp_endpoint: string;
  port: number;
  username: string;
  password: string;
  from_email: string;
  from_name?: string;
}

export interface NoneConfig {
  type: "none";
}

export type EmailProviderConfig = NoneConfig | SmtpConfig | SesConfig | OracleEmailConfig;

export interface TestEmailResponse {
  success: boolean;
  message: string;
  message_id?: string;
}

// System Setting Response from backend
export interface SystemSettingResponse {
  category: string;
  setting_key: string;
  value: EmailProviderConfig;
  description?: string;
  updated_at: string;
}

// System Settings API
export const systemApi = {
  getEmailSettings: async (accessToken?: string): Promise<{ data: SystemSettingResponse }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  updateEmailSettings: async (config: EmailProviderConfig, accessToken?: string): Promise<{ data: SystemSettingResponse }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ config }),
    });
    return handleResponse(response);
  },

  testEmailConnection: async (accessToken?: string): Promise<TestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email/test`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  sendTestEmail: async (toEmail: string, accessToken?: string): Promise<TestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email/send-test`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ to_email: toEmail }),
    });
    return handleResponse(response);
  },
};

// Invitation Types
export type InvitationStatus = "pending" | "accepted" | "expired" | "revoked";

export interface Invitation {
  id: string;
  tenant_id: string;
  email: string;
  role_ids: string[];
  invited_by: string;
  status: InvitationStatus;
  expires_at: string;
  accepted_at?: string;
  created_at: string;
}

export interface CreateInvitationInput {
  email: string;
  role_ids: string[];
  expires_in_hours?: number;
}

// Invitation API
export type InvitationStatusFilter = "pending" | "accepted" | "expired" | "revoked";

export const invitationApi = {
  list: async (
    tenantId: string,
    page = 1,
    perPage = 20,
    status?: InvitationStatusFilter,
    accessToken?: string
  ): Promise<PaginatedResponse<Invitation>> => {
    const params = new URLSearchParams({
      page: page.toString(),
      per_page: perPage.toString(),
    });
    if (status) {
      params.set("status", status);
    }
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/invitations?${params.toString()}`,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },

  create: async (tenantId: string, input: CreateInvitationInput, accessToken?: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/invitations`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  get: async (id: string, accessToken?: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  revoke: async (id: string, accessToken?: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}/revoke`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  resend: async (id: string, accessToken?: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}/resend`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  delete: async (id: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  accept: async (input: { token: string; email?: string; password?: string; display_name?: string }): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/accept`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },
};

// Email Template Types
export type EmailTemplateType =
  | "invitation"
  | "password_reset"
  | "email_mfa"
  | "welcome"
  | "email_verification"
  | "password_changed"
  | "security_alert";

export interface TemplateVariable {
  name: string;
  description: string;
  example: string;
}

export interface EmailTemplateMetadata {
  template_type: EmailTemplateType;
  name: string;
  description: string;
  variables: TemplateVariable[];
}

export interface EmailTemplateContent {
  subject: string;
  html_body: string;
  text_body: string;
}

export interface EmailTemplateWithContent {
  metadata: EmailTemplateMetadata;
  content: EmailTemplateContent;
  is_customized: boolean;
  updated_at?: string;
}

export interface RenderedEmailPreview {
  subject: string;
  html_body: string;
  text_body: string;
}

export interface SendTemplateTestEmailRequest {
  to_email: string;
  subject: string;
  html_body: string;
  text_body: string;
  variables: Record<string, string>;
}

export interface SendTemplateTestEmailResponse {
  success: boolean;
  message: string;
  message_id?: string;
}

// Branding Configuration Types
export interface BrandingConfig {
  logo_url?: string;
  primary_color: string;
  secondary_color: string;
  background_color: string;
  text_color: string;
  custom_css?: string;
  company_name?: string;
  favicon_url?: string;
  allow_registration: boolean;
}

// Public Branding API (no authentication required)
export const publicBrandingApi = {
  get: async (): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/public/branding`);
    return handleResponse(response);
  },
};

// Branding API
export const brandingApi = {
  get: async (accessToken?: string): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/branding`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  update: async (config: BrandingConfig, accessToken?: string): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/branding`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ config }),
    });
    return handleResponse(response);
  },
};

// Email Template API
export const emailTemplateApi = {
  list: async (accessToken?: string): Promise<{ data: EmailTemplateWithContent[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  get: async (type: EmailTemplateType, accessToken?: string): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  update: async (
    type: EmailTemplateType,
    content: EmailTemplateContent,
    accessToken?: string
  ): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(content),
    });
    return handleResponse(response);
  },

  reset: async (type: EmailTemplateType, accessToken?: string): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  preview: async (
    type: EmailTemplateType,
    content: EmailTemplateContent,
    accessToken?: string
  ): Promise<{ data: RenderedEmailPreview }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}/preview`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(content),
    });
    return handleResponse(response);
  },

  sendTestEmail: async (
    type: EmailTemplateType,
    request: SendTemplateTestEmailRequest,
    accessToken?: string
  ): Promise<SendTemplateTestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}/send-test`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(request),
    });
    return handleResponse(response);
  },
};

// ==================== Password Management API ====================

export interface PasswordPolicy {
  min_length: number;
  require_uppercase: boolean;
  require_lowercase: boolean;
  require_numbers: boolean;
  require_symbols: boolean;
  max_age_days: number;
  history_count: number;
  lockout_threshold: number;
  lockout_duration_mins: number;
}

export const passwordApi = {
  forgotPassword: async (email: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/forgot-password`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
    });
    return handleResponse(response);
  },

  resetPassword: async (token: string, newPassword: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/reset-password`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ token, new_password: newPassword }),
    });
    return handleResponse(response);
  },

  changePassword: async (
    currentPassword: string,
    newPassword: string,
    accessToken: string
  ): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/password`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify({
        current_password: currentPassword,
        new_password: newPassword,
      }),
    });
    return handleResponse(response);
  },

  getPasswordPolicy: async (tenantId: string, accessToken?: string): Promise<{ data: PasswordPolicy }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/password-policy`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  updatePasswordPolicy: async (
    tenantId: string,
    policy: Partial<PasswordPolicy>,
    accessToken?: string
  ): Promise<{ data: PasswordPolicy }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/password-policy`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(policy),
    });
    return handleResponse(response);
  },
};

// ==================== Session Management API ====================

export interface SessionInfo {
  id: string;
  device_type?: string;
  device_name?: string;
  ip_address?: string;
  location?: string;
  last_active_at: string;
  created_at: string;
  is_current: boolean;
}

export const sessionApi = {
  listMySessions: async (accessToken: string): Promise<{ data: SessionInfo[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/sessions`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  revokeSession: async (sessionId: string, accessToken: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/sessions/${sessionId}`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  revokeOtherSessions: async (accessToken: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/sessions`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  forceLogoutUser: async (userId: string, accessToken?: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/admin/users/${userId}/logout`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// ==================== WebAuthn/Passkey API ====================

export interface WebAuthnCredential {
  id: string;
  credential_type: string;
  user_label?: string;
  created_at: string;
}

export const webauthnApi = {
  listPasskeys: async (accessToken: string): Promise<{ data: WebAuthnCredential[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/passkeys`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  deletePasskey: async (credentialId: string, accessToken: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/passkeys/${credentialId}`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  startRegistration: async (accessToken: string, label?: string): Promise<{ data: unknown }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/passkeys/register/start`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify({ label }),
    });
    return handleResponse(response);
  },

  completeRegistration: async (
    credential: Credential,
    accessToken: string,
    label?: string
  ): Promise<{ message: string }> => {
    const pkCred = credential as PublicKeyCredential;
    const attestation = pkCred.response as AuthenticatorAttestationResponse;
    const body = {
      credential: {
        id: pkCred.id,
        rawId: arrayBufferToBase64url(pkCred.rawId),
        type: pkCred.type,
        response: {
          attestationObject: arrayBufferToBase64url(attestation.attestationObject),
          clientDataJSON: arrayBufferToBase64url(attestation.clientDataJSON),
        },
      },
      label,
    };
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/passkeys/register/complete`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify(body),
    });
    return handleResponse(response);
  },

  startAuthentication: async (): Promise<{ data: { challenge_id: string; publicKey: unknown } }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/webauthn/authenticate/start`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
    return handleResponse(response);
  },

  completeAuthentication: async (
    challengeId: string,
    credential: Credential
  ): Promise<{ access_token: string; token_type: string; expires_in: number }> => {
    const pkCred = credential as PublicKeyCredential;
    const assertion = pkCred.response as AuthenticatorAssertionResponse;
    const body = {
      challenge_id: challengeId,
      credential: {
        id: pkCred.id,
        rawId: arrayBufferToBase64url(pkCred.rawId),
        type: pkCred.type,
        response: {
          authenticatorData: arrayBufferToBase64url(assertion.authenticatorData),
          clientDataJSON: arrayBufferToBase64url(assertion.clientDataJSON),
          signature: arrayBufferToBase64url(assertion.signature),
          userHandle: assertion.userHandle
            ? arrayBufferToBase64url(assertion.userHandle)
            : undefined,
        },
      },
    };
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/webauthn/authenticate/complete`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    return handleResponse(response);
  },
};

// ==================== WebAuthn Base64URL Helpers ====================

function arrayBufferToBase64url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function base64urlToArrayBuffer(base64url: string): ArrayBuffer {
  const base64 = base64url.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

// ==================== Identity Provider API ====================

export interface IdentityProvider {
  alias: string;
  provider_id: string;
  display_name?: string;
  enabled: boolean;
  config: Record<string, string>;
}

export interface CreateIdentityProviderInput {
  alias: string;
  provider_id: string;
  display_name?: string;
  enabled?: boolean;
  config: Record<string, string>;
}

export interface LinkedIdentity {
  id: string;
  provider_type: string;
  provider_alias: string;
  external_user_id: string;
  external_email?: string;
  linked_at: string;
}

export interface IdpTemplate {
  provider_id: string;
  name: string;
  description?: string;
  required_fields: string[];
  optional_fields: string[];
}

export const identityProviderApi = {
  list: async (accessToken?: string): Promise<{ data: IdentityProvider[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/identity-providers`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  get: async (alias: string, accessToken?: string): Promise<{ data: IdentityProvider }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/identity-providers/${alias}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  create: async (input: CreateIdentityProviderInput, accessToken?: string): Promise<{ data: IdentityProvider }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/identity-providers`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (
    alias: string,
    input: Partial<CreateIdentityProviderInput>,
    accessToken?: string
  ): Promise<{ data: IdentityProvider }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/identity-providers/${alias}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (alias: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/identity-providers/${alias}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  listMyLinkedIdentities: async (accessToken: string): Promise<{ data: LinkedIdentity[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/linked-identities`, {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  unlinkIdentity: async (id: string, accessToken: string): Promise<{ message: string }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/me/linked-identities/${id}`, {
      method: "DELETE",
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    return handleResponse(response);
  },

  templates: async (accessToken?: string): Promise<{ data: IdpTemplate[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/identity-providers/templates`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

export interface EnterpriseSsoDiscoveryInput {
  email: string;
}

export interface EnterpriseSsoDiscoveryResponse {
  tenant_id: string;
  tenant_slug: string;
  connector_alias: string;
  authorize_url: string;
}

export const enterpriseSsoApi = {
  discover: async (
    input: EnterpriseSsoDiscoveryInput,
    query: {
      response_type: string;
      client_id: string;
      redirect_uri: string;
      scope: string;
      state: string;
      nonce?: string;
    }
  ): Promise<{ data: EnterpriseSsoDiscoveryResponse }> => {
    const url = new URL(`${API_BASE_URL}/api/v1/enterprise-sso/discovery`);
    url.searchParams.set("response_type", query.response_type);
    url.searchParams.set("client_id", query.client_id);
    url.searchParams.set("redirect_uri", query.redirect_uri);
    url.searchParams.set("scope", query.scope);
    url.searchParams.set("state", query.state);
    if (query.nonce) url.searchParams.set("nonce", query.nonce);

    const response = await fetch(url.toString(), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },
};

export interface TenantSsoConnector {
  id: string;
  tenant_id: string;
  alias: string;
  display_name?: string;
  provider_type: "saml" | "oidc";
  enabled: boolean;
  priority: number;
  keycloak_alias: string;
  config: Record<string, string>;
  domains: string[];
  created_at: string;
  updated_at: string;
}

export interface CreateTenantSsoConnectorInput {
  alias: string;
  display_name?: string;
  provider_type: "saml" | "oidc";
  enabled?: boolean;
  priority?: number;
  config: Record<string, string>;
  domains: string[];
}

export interface UpdateTenantSsoConnectorInput {
  display_name?: string;
  enabled?: boolean;
  priority?: number;
  config?: Record<string, string>;
  domains?: string[];
}

export const tenantSsoApi = {
  list: async (tenantId: string, accessToken?: string): Promise<{ data: TenantSsoConnector[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  create: async (
    tenantId: string,
    input: CreateTenantSsoConnectorInput,
    accessToken?: string
  ): Promise<{ data: TenantSsoConnector }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (
    tenantId: string,
    connectorId: string,
    input: UpdateTenantSsoConnectorInput,
    accessToken?: string
  ): Promise<{ data: TenantSsoConnector }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify(input),
      }
    );
    return handleResponse(response);
  },

  delete: async (tenantId: string, connectorId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}`,
      {
        method: "DELETE",
        headers: getHeaders(accessToken),
      }
    );
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  test: async (
    tenantId: string,
    connectorId: string,
    accessToken?: string
  ): Promise<{ data: { ok: boolean; message: string } }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/sso/connectors/${connectorId}/test`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },
};

// ==================== Analytics API ====================

export interface LoginStats {
  total_logins: number;
  successful_logins: number;
  failed_logins: number;
  unique_users: number;
  by_event_type: Record<string, number>;
  by_device_type: Record<string, number>;
  period_start: string;
  period_end: string;
}

export interface DailyTrendPoint {
  date: string;
  total: number;
  successful: number;
  failed: number;
}

export interface LoginEvent {
  id: number;
  user_id?: string;
  email?: string;
  tenant_id?: string;
  event_type: string;
  ip_address?: string;
  user_agent?: string;
  device_type?: string;
  location?: string;
  session_id?: string;
  failure_reason?: string;
  created_at: string;
}

export const analyticsApi = {
  getStats: async (
    startDate?: string,
    endDate?: string,
    accessToken?: string
  ): Promise<{ data: LoginStats }> => {
    let url = `${API_BASE_URL}/api/v1/analytics/login-stats`;
    const params = new URLSearchParams();
    if (startDate) params.set("start", startDate);
    if (endDate) params.set("end", endDate);
    if (params.toString()) url += `?${params}`;
    const response = await fetch(url, { headers: getHeaders(accessToken) });
    return handleResponse(response);
  },

  getDailyTrend: async (
    days = 7,
    accessToken?: string,
    startDate?: string,
    endDate?: string
  ): Promise<{ data: DailyTrendPoint[] }> => {
    const params = new URLSearchParams();
    if (startDate && endDate) {
      params.set("start", startDate);
      params.set("end", endDate);
    } else {
      params.set("days", String(days));
    }
    const url = `${API_BASE_URL}/api/v1/analytics/daily-trend?${params}`;
    const response = await fetch(url, { headers: getHeaders(accessToken) });
    return handleResponse(response);
  },

  listEvents: async (
    page = 1,
    perPage = 50,
    email?: string,
    accessToken?: string
  ): Promise<PaginatedResponse<LoginEvent>> => {
    let url = `${API_BASE_URL}/api/v1/analytics/login-events?page=${page}&per_page=${perPage}`;
    if (email) url += `&email=${encodeURIComponent(email)}`;
    const response = await fetch(
      url,
      { headers: getHeaders(accessToken) }
    );
    return handleResponse(response);
  },
};

// ==================== Webhook API ====================

export interface Webhook {
  id: string;
  tenant_id: string;
  name: string;
  url: string;
  secret?: string;
  events: string[];
  enabled: boolean;
  last_triggered_at?: string;
  failure_count: number;
  created_at: string;
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
  status_code?: number;
  response_time_ms?: number;
  error?: string;
}

export const webhookApi = {
  list: async (tenantId: string, accessToken?: string): Promise<{ data: Webhook[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  get: async (tenantId: string, id: string, accessToken?: string): Promise<{ data: Webhook }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks/${id}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  create: async (tenantId: string, input: CreateWebhookInput, accessToken?: string): Promise<{ data: Webhook }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (
    tenantId: string,
    id: string,
    input: Partial<CreateWebhookInput>,
    accessToken?: string
  ): Promise<{ data: Webhook }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks/${id}`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (tenantId: string, id: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks/${id}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  test: async (tenantId: string, id: string, accessToken?: string): Promise<{ data: WebhookTestResult }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks/${id}/test`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  regenerateSecret: async (tenantId: string, id: string, accessToken?: string): Promise<{ data: Webhook }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/webhooks/${id}/regenerate-secret`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// ==================== Tenant-Service Toggle API ====================

export interface ServiceWithStatus {
  id: string;
  name: string;
  base_url?: string;
  status: string;
  enabled: boolean;
}

export const tenantServiceApi = {
  listServices: async (tenantId: string, accessToken?: string): Promise<{ data: ServiceWithStatus[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/services`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  toggleService: async (
    tenantId: string,
    serviceId: string,
    enabled: boolean,
    accessToken?: string
  ): Promise<{ data: ServiceWithStatus[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/services`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ service_id: serviceId, enabled }),
    });
    return handleResponse(response);
  },

  getEnabledServices: async (tenantId: string, accessToken?: string): Promise<{ data: ServiceWithStatus[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/services/enabled`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// ==================== Security Alert API ====================

export type AlertSeverity = "low" | "medium" | "high" | "critical";
export type SecurityAlertType = "brute_force" | "new_device" | "impossible_travel" | "suspicious_ip";

export interface SecurityAlert {
  id: string;
  user_id?: string;
  tenant_id?: string;
  alert_type: SecurityAlertType;
  severity: AlertSeverity;
  details?: Record<string, unknown>;
  resolved_at?: string;
  resolved_by?: string;
  created_at: string;
}

export const securityAlertApi = {
  list: async (
    page = 1,
    perPage = 50,
    unresolvedOnly = false,
    accessToken?: string,
    severity?: AlertSeverity,
    alertType?: SecurityAlertType,
  ): Promise<PaginatedResponse<SecurityAlert>> => {
    let url = `${API_BASE_URL}/api/v1/security/alerts?page=${page}&per_page=${perPage}`;
    if (unresolvedOnly) url += "&unresolved_only=true";
    if (severity) url += `&severity=${severity}`;
    if (alertType) url += `&alert_type=${alertType}`;
    const response = await fetch(url, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  resolve: async (id: string, accessToken?: string): Promise<{ data: SecurityAlert }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/security/alerts/${id}/resolve`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};

// Actions API
export type ActionTrigger =
  | "post-login"
  | "pre-user-registration"
  | "post-user-registration"
  | "post-change-password"
  | "post-email-verification"
  | "pre-token-refresh";

export interface Action {
  id: string;
  tenant_id: string;
  name: string;
  description?: string;
  trigger_id: ActionTrigger;
  script: string;
  enabled: boolean;
  execution_order: number;
  timeout_ms: number;
  last_executed_at?: string;
  execution_count: number;
  error_count: number;
  last_error?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateActionInput {
  name: string;
  description?: string;
  trigger_id: ActionTrigger;
  script: string;
  enabled?: boolean;
  execution_order?: number;
  timeout_ms?: number;
}

export interface UpdateActionInput {
  name?: string;
  description?: string;
  script?: string;
  enabled?: boolean;
  execution_order?: number;
  timeout_ms?: number;
}

export interface ActionExecution {
  id: string;
  action_id: string;
  tenant_id: string;
  trigger_id: ActionTrigger;
  user_id?: string;
  success: boolean;
  duration_ms: number;
  error_message?: string;
  executed_at: string;
}

export interface ActionStats {
  execution_count: number;
  error_count: number;
  success_rate: number;
  avg_duration_ms: number;
  last_24h_count: number;
  last_executed_at?: string;
}

export const actionApi = {
  list: async (
    tenantId: string,
    trigger?: ActionTrigger,
    accessToken?: string
  ): Promise<{ data: Action[] }> => {
    let url = `${API_BASE_URL}/api/v1/tenants/${tenantId}/actions`;
    if (trigger) url += `?trigger_id=${trigger}`;
    const response = await fetch(url, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  get: async (tenantId: string, actionId: string, accessToken?: string): Promise<{ data: Action }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/actions/${actionId}`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  create: async (
    tenantId: string,
    input: CreateActionInput,
    accessToken?: string
  ): Promise<{ data: Action }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/actions`, {
      method: "POST",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (
    tenantId: string,
    actionId: string,
    input: UpdateActionInput,
    accessToken?: string
  ): Promise<{ data: Action }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/actions/${actionId}`, {
      method: "PATCH",
      headers: getHeaders(accessToken),
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (tenantId: string, actionId: string, accessToken?: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/actions/${actionId}`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  logs: async (
    tenantId: string,
    actionId?: string,
    success?: boolean,
    limit = 50,
    accessToken?: string
  ): Promise<{ data: ActionExecution[]; pagination: { page: number; per_page: number; total: number; total_pages: number } }> => {
    let url = `${API_BASE_URL}/api/v1/tenants/${tenantId}/actions/logs?limit=${limit}`;
    if (actionId) url += `&action_id=${actionId}`;
    if (success !== undefined) url += `&success=${success}`;
    const response = await fetch(url, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  stats: async (
    tenantId: string,
    actionId: string,
    accessToken?: string
  ): Promise<{ data: ActionStats }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/actions/${actionId}/stats`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  triggers: async (accessToken?: string): Promise<{ data: ActionTrigger[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/actions/triggers`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },
};
