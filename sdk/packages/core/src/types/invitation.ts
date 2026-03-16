export type InvitationStatus = "pending" | "accepted" | "expired" | "revoked";

export interface Invitation {
  id: string;
  tenantId: string;
  email: string;
  roleIds: string[];
  invitedBy: string;
  status: InvitationStatus;
  expiresAt: string;
  acceptedAt?: string;
  createdAt: string;
}

export interface CreateInvitationInput {
  email: string;
  roleIds: string[];
  expiresInHours?: number;
}

export interface InvitationValidation {
  valid: boolean;
  invitation?: Invitation;
  tenantName?: string;
}

export interface AcceptInvitationInput {
  token: string;
  email?: string;
  displayName?: string;
  password?: string;
}
