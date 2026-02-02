/**
 * Error message mapping for user-friendly validation error display.
 *
 * Maps backend validation error codes and patterns to human-readable messages.
 */

const ERROR_MESSAGES: Record<string, string> = {
  // Slug validation
  invalid_slug:
    "Slug can only contain lowercase letters, numbers, and hyphens. It cannot start or end with a hyphen.",
  // Length validation
  length: "Length must be between 1-63 characters",
  // Common validation
  required: "This field is required",
  email: "Please enter a valid email address",
  // Business errors
  "already exists": "This value already exists. Please use a different one.",
  "not found": "The requested resource was not found",
  conflict: "A resource with this identifier already exists",
  // Database errors
  "duplicate entry": "This name already exists. Please use a different one.",
  "1062": "This name already exists. Please use a different one.",
};

// Field name translations
const FIELD_NAMES: Record<string, string> = {
  slug: "Slug",
  name: "Name",
  email: "Email",
  logo_url: "Logo URL",
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
export function formatErrorMessage(rawMessage: string): string {
  // Check if it contains known error codes
  for (const [key, friendlyMessage] of Object.entries(ERROR_MESSAGES)) {
    if (rawMessage.toLowerCase().includes(key.toLowerCase())) {
      // If field name is present, extract and format it
      const fieldMatch = rawMessage.match(/^(\w+):/);
      if (fieldMatch) {
        const fieldKey = fieldMatch[1].toLowerCase();
        const fieldName = FIELD_NAMES[fieldKey] || capitalize(fieldKey);
        return `${fieldName}: ${friendlyMessage}`;
      }
      return friendlyMessage;
    }
  }

  // Handle "field: Validation error: ..." format
  const validationMatch = rawMessage.match(/^(\w+):\s*Validation error:\s*(.+)$/i);
  if (validationMatch) {
    const [, field, error] = validationMatch;
    const fieldName = FIELD_NAMES[field.toLowerCase()] || capitalize(field);
    // Remove the technical details in brackets
    const cleanError = error.replace(/\s*\[.*\]$/, "").trim();
    return `${fieldName}: ${cleanError}`;
  }

  // Handle simple "field: error" format
  const fieldMatch = rawMessage.match(/^(\w+):\s*(.+)$/);
  if (fieldMatch) {
    const [, field, error] = fieldMatch;
    const fieldName = FIELD_NAMES[field.toLowerCase()] || capitalize(field);
    return `${fieldName}: ${error}`;
  }

  return rawMessage;
}

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
