import { API_BASE_URL, getHeaders, handleResponse } from "./client";

export interface MfaStatusResponse {
  totp_enabled: boolean;
  webauthn_enabled: boolean;
  recovery_codes_remaining: number;
}

export const mfaApi = {
  status: async (accessToken: string): Promise<MfaStatusResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/mfa/status`, {
      headers: getHeaders(accessToken),
    });
    const result = await handleResponse<{ data: MfaStatusResponse }>(response);
    return result.data;
  },

  totpRemove: async (accessToken: string): Promise<void> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/mfa/totp`, {
      method: "DELETE",
      headers: getHeaders(accessToken),
    });
    await handleResponse(response);
  },

  recoveryCodesGenerate: async (
    accessToken: string
  ): Promise<string[]> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/mfa/recovery-codes/generate`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
      }
    );
    const result = await handleResponse<{ data: string[] }>(response);
    return result.data;
  },

  recoveryCodesRemaining: async (
    accessToken: string
  ): Promise<number> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/mfa/recovery-codes/remaining`,
      {
        headers: getHeaders(accessToken),
      }
    );
    const result = await handleResponse<{ data: number }>(response);
    return result.data;
  },
};
