import type { AppLocale } from "~/i18n";
import { translate } from "~/i18n/translate";
import { ApiResponseError } from "~/services/api/client";

const API_ERROR_CODE_MAP: Record<string, string> = {
  not_found: "apiErrors.notFound",
  bad_request: "apiErrors.badRequest",
  unauthorized: "apiErrors.unauthorized",
  forbidden: "apiErrors.forbidden",
  conflict: "apiErrors.conflict",
  database_error: "apiErrors.serverError",
  cache_error: "apiErrors.serverError",
  jwt_error: "apiErrors.sessionExpired",
  identity_backend_error: "apiErrors.authServiceError",
  action_execution_failed: "apiErrors.serverError",
  internal_error: "apiErrors.serverError",
  method_not_allowed: "apiErrors.badRequest",
  rate_limited: "apiErrors.rateLimited",
  unsupported_media_type: "apiErrors.badRequest",
  client_error: "apiErrors.badRequest",
  validation_error: "apiErrors.badRequest",
  service_unavailable: "apiErrors.serviceUnavailable",
};

/**
 * Maps an API error (or any caught error) to a localized, user-friendly string.
 *
 * - ApiResponseError with a known code: translates via API_ERROR_CODE_MAP
 * - ApiResponseError with code "validation": delegates to formatErrorMessage
 * - Plain Error: delegates to formatErrorMessage for substring matching
 * - Unknown: returns generic "something went wrong"
 */
export function mapApiError(
  error: unknown,
  locale: AppLocale = "en-US"
): string {
  if (error instanceof ApiResponseError) {
    const specializedMessage = mapSpecialApiError(error, locale);
    if (specializedMessage) {
      return specializedMessage;
    }
    if (error.code === "validation") {
      return formatErrorMessage(error.message, locale);
    }
    const i18nKey = API_ERROR_CODE_MAP[error.code];
    if (i18nKey) {
      return translate(locale, i18nKey);
    }
  }
  if (error instanceof Error) {
    return formatErrorMessage(error.message, locale);
  }
  return translate(locale, "apiErrors.unknown");
}

function mapSpecialApiError(
  error: ApiResponseError,
  locale: AppLocale
): string | null {
  const message = error.message.toLowerCase();

  if (
    error.code === "bad_request" &&
    (message.includes("expired reset token") ||
      message.includes("invalid or expired reset token"))
  ) {
    return translate(locale, "auth.resetPassword.expiredToken");
  }

  // Credential-specific unauthorized errors should show the actual error,
  // not the generic "session expired" message.
  if (error.code === "unauthorized") {
    if (message.includes("invalid email or password")) {
      return translate(locale, "apiErrors.invalidCredentials");
    }
    if (message.includes("invalid totp code")) {
      return translate(locale, "apiErrors.invalidTotpCode");
    }
    if (message.includes("invalid or already used recovery code")) {
      return translate(locale, "apiErrors.invalidRecoveryCode");
    }
  }

  return null;
}

const ERROR_MESSAGES: Record<string, string> = {
  // Slug validation
  invalid_slug: "validation.slug",
  // Length validation
  length: "validation.length",
  // Common validation
  required: "validation.required",
  email: "validation.email",
  // Business errors
  "already exists": "validation.alreadyExists",
  "not found": "validation.notFound",
  conflict: "validation.conflict",
  // Database errors
  "duplicate entry": "validation.duplicateEntry",
  "1062": "validation.duplicateEntry",
  // SSRF protection
  ssrf_blocked: "validation.ssrfBlocked",
  internal_ip_blocked: "validation.internalIpBlocked",
};

// Field name translations
const FIELD_NAMES: Record<string, string> = {
  slug: "validation.fields.slug",
  name: "validation.fields.name",
  email: "validation.fields.email",
  logo_url: "validation.fields.logo_url",
};

/**
 * Formats a raw backend error message into a user-friendly message.
 *
 * @param rawMessage - The raw error message from the backend
 * @returns A user-friendly error message
 *
 * @example
 * formatErrorMessage("slug: Validation error: invalid_slug [{...}]")
 * // Returns: "Slug can only contain lowercase letters, numbers, and hyphens..."
 */
export function formatErrorMessage(
  rawMessage: string,
  locale: AppLocale = "en-US"
): string {
  // Check if it contains known error codes
  for (const [key, messageKey] of Object.entries(ERROR_MESSAGES)) {
    if (rawMessage.toLowerCase().includes(key.toLowerCase())) {
      // If field name is present, extract and format it
      const fieldMatch = rawMessage.match(/^(\w+):/);
      if (fieldMatch) {
        const fieldKey = fieldMatch[1].toLowerCase();
        const fieldNameKey = FIELD_NAMES[fieldKey];
        const fieldName = fieldNameKey
          ? translate(locale, fieldNameKey)
          : capitalize(fieldKey);
        return `${fieldName}: ${translate(locale, messageKey)}`;
      }
      return translate(locale, messageKey);
    }
  }

  // Handle "field: Validation error: ..." format
  const validationMatch = rawMessage.match(/^(\w+):\s*Validation error:\s*(.+)$/i);
  if (validationMatch) {
    const [, field, error] = validationMatch;
    const fieldName = FIELD_NAMES[field.toLowerCase()]
      ? translate(locale, FIELD_NAMES[field.toLowerCase()])
      : capitalize(field);
    // Remove the technical details in brackets
    const cleanError = error.replace(/\s*\[.*\]$/, "").trim();
    return `${fieldName}: ${cleanError}`;
  }

  // Handle simple "field: error" format
  const fieldMatch = rawMessage.match(/^(\w+):\s*(.+)$/);
  if (fieldMatch) {
    const [, field, error] = fieldMatch;
    const fieldName = FIELD_NAMES[field.toLowerCase()]
      ? translate(locale, FIELD_NAMES[field.toLowerCase()])
      : capitalize(field);
    return `${fieldName}: ${error}`;
  }

  return rawMessage;
}

const OAUTH_ERROR_MAP: Record<string, string> = {
  access_denied: "auth.login.oauthErrors.accessDenied",
  state_mismatch: "auth.login.oauthErrors.stateMismatch",
  token_exchange_failed: "auth.login.oauthErrors.tokenExchangeFailed",
  callback_exception: "auth.login.oauthErrors.callbackException",
  invalid_grant: "auth.login.oauthErrors.invalidGrant",
};

/**
 * Maps an OAuth/OIDC callback error code to a localized, user-friendly string.
 */
export function mapOAuthError(
  errorCode: string,
  locale: AppLocale = "en-US"
): string {
  const i18nKey = OAUTH_ERROR_MAP[errorCode];
  if (i18nKey) {
    return translate(locale, i18nKey);
  }
  return translate(locale, "auth.login.oauthErrors.unknown");
}

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
