import type { Auth9HttpClient } from "../http-client.js";
import type { SessionInfo } from "../types/session.js";

export class SessionsClient {
  constructor(private http: Auth9HttpClient) {}

  async listMy(): Promise<SessionInfo[]> {
    const result = await this.http.get<{ data: SessionInfo[] }>(
      "/api/v1/users/me/sessions"
    );
    return result.data;
  }

  async revoke(id: string): Promise<void> {
    await this.http.delete(`/api/v1/users/me/sessions/${id}`);
  }

  async revokeAllOther(): Promise<void> {
    await this.http.delete("/api/v1/users/me/sessions");
  }

  async forceLogout(userId: string): Promise<void> {
    await this.http.post(`/api/v1/admin/users/${userId}/logout`);
  }
}
