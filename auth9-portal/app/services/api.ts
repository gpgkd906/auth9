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
  list: async (page = 1, perPage = 20): Promise<PaginatedResponse<Tenant>> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants?page=${page}&per_page=${perPage}`
    );
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateTenantInput): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateTenantInput>): Promise<{ data: Tenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${id}`, {
      method: "DELETE",
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
  list: async (page = 1, perPage = 20): Promise<PaginatedResponse<User>> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users?page=${page}&per_page=${perPage}`
    );
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateUserInput & { password?: string }): Promise<{ data: User }> => {
    const { password, ...user } = input;
    const response = await fetch(`${API_BASE_URL}/api/v1/users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...user, password }),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateUserInput>): Promise<{ data: User }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  getTenants: async (userId: string): Promise<{ data: { id: string; tenant_id: string; role_in_tenant: string; joined_at: string; tenant: Tenant }[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants`);
    return handleResponse(response);
  },

  addToTenant: async (userId: string, tenantId: string, roleInTenant: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tenant_id: tenantId, role_in_tenant: roleInTenant }),
    });
    return handleResponse(response);
  },

  removeFromTenant: async (userId: string, tenantId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
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

export interface ClientWithSecret {
  client: Client;
  client_secret: string;
}

export interface CreateClientInput {
  name?: string;
}

export const serviceApi = {
  list: async (tenantId?: string, page = 1, perPage = 20): Promise<PaginatedResponse<Service>> => {
    let url = `${API_BASE_URL}/api/v1/services?page=${page}&per_page=${perPage}`;
    if (tenantId) url += `&tenant_id=${tenantId}`;
    const response = await fetch(url);
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`);
    return handleResponse(response);
  },

  create: async (input: CreateServiceInput): Promise<{ data: { service: Service, client: ClientWithSecret } }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  update: async (id: string, input: Partial<CreateServiceInput>): Promise<{ data: Service }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${id}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  listClients: async (serviceId: string): Promise<{ data: Client[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients`);
    return handleResponse(response);
  },

  createClient: async (serviceId: string, input: CreateClientInput): Promise<{ data: ClientWithSecret }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deleteClient: async (serviceId: string, clientId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients/${clientId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  regenerateClientSecret: async (serviceId: string, clientId: string): Promise<{ data: { client_id: string; client_secret: string } }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/clients/${clientId}/regenerate-secret`, {
      method: "POST",
    });
    return handleResponse(response);
  },
};

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
  listRoles: async (serviceId: string): Promise<{ data: Role[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/roles`);
    return handleResponse(response);
  },

  createRole: async (serviceId: string, input: CreateRoleInput): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...input, service_id: serviceId }),
    });
    return handleResponse(response);
  },

  updateRole: async (serviceId: string, roleId: string, input: Partial<CreateRoleInput>): Promise<{ data: Role }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deleteRole: async (serviceId: string, roleId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  listPermissions: async (serviceId: string): Promise<{ data: Permission[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/services/${serviceId}/permissions`);
    return handleResponse(response);
  },

  createPermission: async (input: CreatePermissionInput): Promise<{ data: Permission }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/permissions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  deletePermission: async (permissionId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/permissions/${permissionId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  getRole: async (roleId: string): Promise<{ data: RoleWithPermissions }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}`);
    return handleResponse(response);
  },

  assignPermissionToRole: async (roleId: string, permissionId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}/permissions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ permission_id: permissionId }),
    });
    return handleResponse(response);
  },

  removePermissionFromRole: async (roleId: string, permissionId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/roles/${roleId}/permissions/${permissionId}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
  },

  assignRoles: async (input: AssignRolesInput): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/rbac/assign`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  getUserRoles: async (userId: string, tenantId: string): Promise<{ data: UserRolesInTenant }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/roles`);
    return handleResponse(response);
  },

  getUserAssignedRoles: async (userId: string, tenantId: string): Promise<{ data: Role[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/assigned-roles`);
    return handleResponse(response);
  },

  unassignRole: async (userId: string, tenantId: string, roleId: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/users/${userId}/tenants/${tenantId}/roles/${roleId}`, {
      method: "DELETE",
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
  roles: string[]; // Role IDs
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
  action: string;
  resource_type: string;
  resource_id?: string;
  old_value?: unknown;
  new_value?: unknown;
  ip_address?: string;
  created_at: string;
}

export const auditApi = {
  list: async (page = 1, perPage = 50): Promise<PaginatedResponse<AuditLog>> => {
    const offset = (page - 1) * perPage;
    const response = await fetch(
      `${API_BASE_URL}/api/v1/audit-logs?limit=${perPage}&offset=${offset}`
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
  getEmailSettings: async (): Promise<{ data: SystemSettingResponse }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email`);
    return handleResponse(response);
  },

  updateEmailSettings: async (config: EmailProviderConfig): Promise<{ data: SystemSettingResponse }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ config }),
    });
    return handleResponse(response);
  },

  testEmailConnection: async (): Promise<TestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email/test`, {
      method: "POST",
    });
    return handleResponse(response);
  },

  sendTestEmail: async (toEmail: string): Promise<TestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email/send-test`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
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
export const invitationApi = {
  list: async (tenantId: string, page = 1, perPage = 20): Promise<PaginatedResponse<Invitation>> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/invitations?page=${page}&per_page=${perPage}`
    );
    return handleResponse(response);
  },

  create: async (tenantId: string, input: CreateInvitationInput): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/tenants/${tenantId}/invitations`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(input),
    });
    return handleResponse(response);
  },

  get: async (id: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}`);
    return handleResponse(response);
  },

  revoke: async (id: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}/revoke`, {
      method: "POST",
    });
    return handleResponse(response);
  },

  resend: async (id: string): Promise<{ data: Invitation }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}/resend`, {
      method: "POST",
    });
    return handleResponse(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/invitations/${id}`, {
      method: "DELETE",
    });
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new Error(error.message);
    }
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

// Branding API
export const brandingApi = {
  get: async (): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/branding`);
    return handleResponse(response);
  },

  update: async (config: BrandingConfig): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/branding`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ config }),
    });
    return handleResponse(response);
  },
};

// Email Template API
export const emailTemplateApi = {
  list: async (): Promise<{ data: EmailTemplateWithContent[] }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates`);
    return handleResponse(response);
  },

  get: async (type: EmailTemplateType): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}`);
    return handleResponse(response);
  },

  update: async (
    type: EmailTemplateType,
    content: EmailTemplateContent
  ): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(content),
    });
    return handleResponse(response);
  },

  reset: async (type: EmailTemplateType): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}`, {
      method: "DELETE",
    });
    return handleResponse(response);
  },

  preview: async (
    type: EmailTemplateType,
    content: EmailTemplateContent
  ): Promise<{ data: RenderedEmailPreview }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}/preview`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(content),
    });
    return handleResponse(response);
  },

  sendTestEmail: async (
    type: EmailTemplateType,
    request: SendTemplateTestEmailRequest
  ): Promise<SendTemplateTestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email-templates/${type}/send-test`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    return handleResponse(response);
  },
};
