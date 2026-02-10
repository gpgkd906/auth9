export interface Role {
  id: string;
  serviceId: string;
  name: string;
  description?: string;
  parentRoleId?: string;
  createdAt: string;
  updatedAt: string;
}

export interface Permission {
  id: string;
  serviceId: string;
  code: string;
  name: string;
  description?: string;
  createdAt?: string;
}

export interface CreateRoleInput {
  name: string;
  description?: string;
  parentRoleId?: string;
}

export interface CreatePermissionInput {
  serviceId: string;
  code: string;
  name: string;
  description?: string;
}

export interface RoleWithPermissions extends Role {
  permissions: Permission[];
}

export interface AssignRolesInput {
  userId: string;
  tenantId: string;
  roles: string[];
}

export interface UserRolesInTenant {
  userId: string;
  tenantId: string;
  roles: string[];
  permissions: string[];
}
