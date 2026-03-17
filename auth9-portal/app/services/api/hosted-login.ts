import { API_BASE_URL, handleResponse } from "./client";

export interface HostedLoginTokenResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
}

export const hostedLoginApi = {
  passwordLogin: async (
    email: string,
    password: string
  ): Promise<HostedLoginTokenResponse> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/password`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      }
    );
    return handleResponse(response);
  },

  logout: async (accessToken: string): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/logout`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({}),
      }
    );
    return handleResponse(response);
  },

  startPasswordReset: async (
    email: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/start-password-reset`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      }
    );
    return handleResponse(response);
  },

  completePasswordReset: async (
    token: string,
    newPassword: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/complete-password-reset`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, new_password: newPassword }),
      }
    );
    return handleResponse(response);
  },
};
