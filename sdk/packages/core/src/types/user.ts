export interface User {
  id: string;
  email: string;
  displayName?: string;
  avatarUrl?: string;
  mfaEnabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateUserInput {
  email: string;
  displayName?: string;
  avatarUrl?: string;
  password?: string;
  tenantId?: string;
}

export interface UpdateUserInput {
  displayName?: string;
  avatarUrl?: string;
}

export interface AddUserToTenantInput {
  tenantId: string;
  roleInTenant: string;
}

export interface UpdateUserRoleInput {
  roleInTenant: string;
}
