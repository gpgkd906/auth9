export interface ScimToken {
  id: string;
  connectorId: string;
  name: string;
  lastUsedAt?: string;
  expiresAt?: string;
  createdAt: string;
}

export interface ScimTokenWithValue {
  id: string;
  connectorId: string;
  name: string;
  token: string;
  expiresAt?: string;
  createdAt: string;
}

export interface CreateScimTokenInput {
  name: string;
  expiresAt?: string;
}

export interface ScimLog {
  id: string;
  connectorId: string;
  operation: string;
  resourceType: string;
  resourceId?: string;
  status: "success" | "error";
  detail?: string;
  createdAt: string;
}

export interface ScimLogQuery {
  operation?: string;
  resourceType?: string;
  status?: string;
  limit?: number;
}

export interface ScimGroupMapping {
  id: string;
  connectorId: string;
  scimGroupId: string;
  scimGroupName: string;
  roleId: string;
  roleName?: string;
}
