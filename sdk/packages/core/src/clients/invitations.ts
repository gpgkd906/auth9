import type { Auth9HttpClient } from "../http-client.js";
import type {
  Invitation,
  CreateInvitationInput,
  InvitationValidation,
  AcceptInvitationInput,
} from "../types/invitation.js";

export class InvitationsClient {
  constructor(private http: Auth9HttpClient) {}

  async list(tenantId: string): Promise<Invitation[]> {
    const result = await this.http.get<{ data: Invitation[] }>(
      `/api/v1/tenants/${tenantId}/invitations`
    );
    return result.data;
  }

  async get(id: string): Promise<Invitation> {
    const result = await this.http.get<{ data: Invitation }>(
      `/api/v1/invitations/${id}`
    );
    return result.data;
  }

  async create(
    tenantId: string,
    input: CreateInvitationInput
  ): Promise<Invitation> {
    const result = await this.http.post<{ data: Invitation }>(
      `/api/v1/tenants/${tenantId}/invitations`,
      input
    );
    return result.data;
  }

  async delete(id: string): Promise<void> {
    await this.http.delete(`/api/v1/invitations/${id}`);
  }

  async revoke(id: string): Promise<void> {
    await this.http.post(`/api/v1/invitations/${id}/revoke`);
  }

  async resend(id: string): Promise<void> {
    await this.http.post(`/api/v1/invitations/${id}/resend`);
  }

  async validate(token: string): Promise<InvitationValidation> {
    const result = await this.http.get<{ data: InvitationValidation }>(
      "/api/v1/invitations/validate",
      { token }
    );
    return result.data;
  }

  async accept(input: AcceptInvitationInput): Promise<void> {
    await this.http.post("/api/v1/invitations/accept", input);
  }
}
