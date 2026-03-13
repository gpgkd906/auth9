import { API_BASE_URL, getHeaders, handleResponse } from "./client";

// Email Provider Configuration Types
export interface SmtpConfig {
  type: "smtp";
  host: string;
  port: number;
  username?: string;
  password?: string;
  use_tls: boolean;
  from_email: string;
  from_name?: string;
}

export interface SesConfig {
  type: "ses";
  region: string;
  access_key_id?: string;
  secret_access_key?: string;
  from_email: string;
  from_name?: string;
  configuration_set?: string;
}

export interface OracleEmailConfig {
  type: "oracle";
  smtp_endpoint: string;
  port: number;
  username: string;
  password: string;
  from_email: string;
  from_name?: string;
}

export interface NoneConfig {
  type: "none";
}

export type EmailProviderConfig =
  | NoneConfig
  | SmtpConfig
  | SesConfig
  | OracleEmailConfig;

export interface TestEmailResponse {
  success: boolean;
  message: string;
  message_id?: string;
}

export interface MaliciousIpBlacklistEntry {
  id: string;
  ip_address: string;
  reason?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
}

// System Setting Response from backend
export interface SystemSettingResponse {
  category: string;
  setting_key: string;
  value: EmailProviderConfig;
  description?: string;
  updated_at: string;
}

// System Settings API
export const systemApi = {
  getEmailSettings: async (
    accessToken?: string
  ): Promise<{ data: SystemSettingResponse }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  updateEmailSettings: async (
    config: EmailProviderConfig,
    accessToken?: string
  ): Promise<{ data: SystemSettingResponse }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ config }),
    });
    return handleResponse(response);
  },

  testEmailConnection: async (
    accessToken?: string
  ): Promise<TestEmailResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/email/test`, {
      method: "POST",
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  sendTestEmail: async (
    toEmail: string,
    accessToken?: string
  ): Promise<TestEmailResponse> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email/send-test`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ to_email: toEmail }),
      }
    );
    return handleResponse(response);
  },

  getMaliciousIpBlacklist: async (
    accessToken?: string
  ): Promise<{ data: MaliciousIpBlacklistEntry[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/security/malicious-ip-blacklist`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  updateMaliciousIpBlacklist: async (
    entries: Array<{ ip_address: string; reason?: string }>,
    accessToken?: string
  ): Promise<{ data: MaliciousIpBlacklistEntry[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/security/malicious-ip-blacklist`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ entries }),
      }
    );
    return handleResponse(response);
  },
};
