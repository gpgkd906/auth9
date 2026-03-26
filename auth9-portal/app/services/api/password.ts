import { API_BASE_URL, getHeaders, handleResponse } from "./client";

export interface PasswordPolicy {
  min_length: number;
  require_uppercase: boolean;
  require_lowercase: boolean;
  require_numbers: boolean;
  require_symbols: boolean;
  max_age_days: number;
  history_count: number;
  lockout_threshold: number;
  lockout_duration_mins: number;
  breach_check_mode: string;
  min_breach_count: number;
  breach_check_on_login: boolean;
}

export const passwordApi = {
  forgotPassword: async (email: string): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/auth/forgot-password`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      }
    );
    return handleResponse(response);
  },

  resetPassword: async (
    token: string,
    newPassword: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/auth/reset-password`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, new_password: newPassword }),
      }
    );
    return handleResponse(response);
  },

  changePassword: async (
    currentPassword: string,
    newPassword: string,
    accessToken: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/me/password`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({
          current_password: currentPassword,
          new_password: newPassword,
        }),
      }
    );
    return handleResponse(response);
  },

  forceChangePassword: async (
    newPassword: string,
    accessToken: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/me/force-update-password`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${accessToken}`,
        },
        body: JSON.stringify({ new_password: newPassword }),
      }
    );
    return handleResponse(response);
  },

  getPasswordPolicy: async (
    tenantId: string,
    accessToken?: string
  ): Promise<{ data: PasswordPolicy }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/password-policy`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  updatePasswordPolicy: async (
    tenantId: string,
    policy: Partial<PasswordPolicy>,
    accessToken?: string
  ): Promise<{ data: PasswordPolicy }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/tenants/${tenantId}/password-policy`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify(policy),
      }
    );
    return handleResponse(response);
  },
};
