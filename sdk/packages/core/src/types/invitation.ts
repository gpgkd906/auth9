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
