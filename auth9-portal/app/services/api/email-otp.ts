import { API_BASE_URL, handleResponse } from "./client";

export interface SendEmailOtpResponse {
  message: string;
  expires_in_seconds: number;
}

export interface EmailOtpTokenResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
}

export const emailOtpApi = {
  send: async (email: string): Promise<SendEmailOtpResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/email-otp/send`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email }),
    });
    return handleResponse(response);
  },

  verify: async (email: string, code: string): Promise<EmailOtpTokenResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/email-otp/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, code }),
    });
    return handleResponse(response);
  },
};
