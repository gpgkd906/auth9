import { API_BASE_URL, getHeaders, handleResponse } from "./client";

export interface PendingAction {
  id: string;
  action_type: string;
  redirect_url: string;
}

export interface HostedLoginTokenResponse {
  access_token: string;
  token_type: string;
  expires_in: number;
  pending_actions?: PendingAction[];
}

export interface MfaChallengeResponse {
  mfa_required: true;
  mfa_session_token: string;
  mfa_methods: string[];
  expires_in: number;
}

export type PasswordLoginResponse =
  | HostedLoginTokenResponse
  | MfaChallengeResponse;

export function isMfaChallenge(
  res: PasswordLoginResponse
): res is MfaChallengeResponse {
  return "mfa_required" in res && res.mfa_required === true;
}

export interface TotpEnrollmentResponse {
  setup_token: string;
  otpauth_uri: string;
  secret: string;
}

export const hostedLoginApi = {
  passwordLogin: async (
    email: string,
    password: string
  ): Promise<PasswordLoginResponse> => {
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

  sendVerification: async (
    email: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/send-verification`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      }
    );
    return handleResponse(response);
  },

  verifyEmail: async (
    token: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/verify-email`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token }),
      }
    );
    return handleResponse(response);
  },

  getPendingActions: async (
    accessToken: string
  ): Promise<PendingAction[]> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/pending-actions`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  completeAction: async (
    actionId: string,
    accessToken: string
  ): Promise<{ message: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/hosted-login/complete-action`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ action_id: actionId }),
      }
    );
    return handleResponse(response);
  },

  authorizeComplete: async (
    loginChallengeId: string,
    accessToken: string
  ): Promise<{ redirect_url: string }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/auth/authorize/complete`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ login_challenge_id: loginChallengeId }),
      }
    );
    const result: { data?: { redirect_url: string }; redirect_url?: string } =
      await handleResponse(response);
    return result.data ?? (result as { redirect_url: string });
  },

  challengeTotp: async (
    mfaSessionToken: string,
    code: string
  ): Promise<HostedLoginTokenResponse> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/mfa/challenge/totp`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mfa_session_token: mfaSessionToken,
          code,
        }),
      }
    );
    return handleResponse(response);
  },

  challengeRecoveryCode: async (
    mfaSessionToken: string,
    code: string
  ): Promise<HostedLoginTokenResponse> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/mfa/challenge/recovery-code`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mfa_session_token: mfaSessionToken,
          code,
        }),
      }
    );
    return handleResponse(response);
  },

  totpEnrollStart: async (
    accessToken: string
  ): Promise<TotpEnrollmentResponse> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/mfa/totp/enroll`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
      }
    );
    const result = await handleResponse<{ data: TotpEnrollmentResponse }>(
      response
    );
    return result.data;
  },

  totpEnrollVerify: async (
    setupToken: string,
    code: string,
    accessToken: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/mfa/totp/enroll/verify`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ setup_token: setupToken, code }),
      }
    );
    await handleResponse(response);
  },
};
