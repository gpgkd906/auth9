import { API_BASE_URL, ApiResponseError, getHeaders, handleResponse, type ApiError } from "./client";

// Email Template Types
export type EmailTemplateType =
  | "invitation"
  | "password_reset"
  | "email_mfa"
  | "welcome"
  | "email_verification"
  | "password_changed"
  | "security_alert";

export interface TemplateVariable {
  name: string;
  description: string;
  example: string;
}

export interface EmailTemplateMetadata {
  template_type: EmailTemplateType;
  name: string;
  description: string;
  variables: TemplateVariable[];
}

export interface EmailTemplateContent {
  subject: string;
  html_body: string;
  text_body: string;
}

export interface EmailTemplateWithContent {
  metadata: EmailTemplateMetadata;
  content: EmailTemplateContent;
  is_customized: boolean;
  updated_at?: string;
}

export interface RenderedEmailPreview {
  subject: string;
  html_body: string;
  text_body: string;
}

export interface SendTemplateTestEmailRequest {
  to_email: string;
  subject: string;
  html_body: string;
  text_body: string;
  variables: Record<string, string>;
}

export interface SendTemplateTestEmailResponse {
  success: boolean;
  message: string;
  message_id?: string;
}

// Branding Configuration Types
export interface BrandingConfig {
  logo_url?: string;
  primary_color: string;
  secondary_color: string;
  background_color: string;
  text_color: string;
  custom_css?: string;
  company_name?: string;
  favicon_url?: string;
  allow_registration: boolean;
  email_otp_enabled?: boolean;
}

// Public Branding API (no authentication required)
export const publicBrandingApi = {
  get: async (clientId?: string): Promise<{ data: BrandingConfig }> => {
    let url = `${API_BASE_URL}/api/v1/public/branding`;
    if (clientId) url += `?client_id=${encodeURIComponent(clientId)}`;
    const response = await fetch(url);
    return handleResponse(response);
  },
};

// Branding API
export const brandingApi = {
  get: async (
    accessToken?: string
  ): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/branding`, {
      headers: getHeaders(accessToken),
    });
    return handleResponse(response);
  },

  update: async (
    config: BrandingConfig,
    accessToken?: string
  ): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(`${API_BASE_URL}/api/v1/system/branding`, {
      method: "PUT",
      headers: getHeaders(accessToken),
      body: JSON.stringify({ config }),
    });
    return handleResponse(response);
  },
};

// Email Template API
export const emailTemplateApi = {
  list: async (
    accessToken?: string
  ): Promise<{ data: EmailTemplateWithContent[] }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email-templates`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  get: async (
    type: EmailTemplateType,
    accessToken?: string
  ): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email-templates/${type}`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  update: async (
    type: EmailTemplateType,
    content: EmailTemplateContent,
    accessToken?: string
  ): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email-templates/${type}`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify(content),
      }
    );
    return handleResponse(response);
  },

  reset: async (
    type: EmailTemplateType,
    accessToken?: string
  ): Promise<{ data: EmailTemplateWithContent }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email-templates/${type}`,
      {
        method: "DELETE",
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  preview: async (
    type: EmailTemplateType,
    content: EmailTemplateContent,
    accessToken?: string
  ): Promise<{ data: RenderedEmailPreview }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email-templates/${type}/preview`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify(content),
      }
    );
    return handleResponse(response);
  },

  sendTestEmail: async (
    type: EmailTemplateType,
    request: SendTemplateTestEmailRequest,
    accessToken?: string
  ): Promise<SendTemplateTestEmailResponse> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/system/email-templates/${type}/send-test`,
      {
        method: "POST",
        headers: getHeaders(accessToken),
        body: JSON.stringify(request),
      }
    );
    return handleResponse(response);
  },
};

// Service Branding API
export interface ServiceBranding {
  id: string;
  service_id: string;
  config: BrandingConfig;
  created_at: string;
  updated_at: string;
}

export const serviceBrandingApi = {
  get: async (
    serviceId: string,
    accessToken?: string
  ): Promise<{ data: BrandingConfig }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/services/${serviceId}/branding`,
      {
        headers: getHeaders(accessToken),
      }
    );
    return handleResponse(response);
  },

  update: async (
    serviceId: string,
    config: BrandingConfig,
    accessToken?: string
  ): Promise<{ data: ServiceBranding }> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/services/${serviceId}/branding`,
      {
        method: "PUT",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ config }),
      }
    );
    return handleResponse(response);
  },

  delete: async (
    serviceId: string,
    accessToken?: string
  ): Promise<void> => {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/services/${serviceId}/branding`,
      {
        method: "DELETE",
        headers: getHeaders(accessToken),
      }
    );
    if (!response.ok) {
      const error: ApiError = await response.json();
      throw new ApiResponseError(error, response.status);
    }
  },
};
