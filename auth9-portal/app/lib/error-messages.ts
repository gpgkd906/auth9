import type { AppLocale } from "~/i18n";
import { translate } from "~/i18n/translate";

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

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
