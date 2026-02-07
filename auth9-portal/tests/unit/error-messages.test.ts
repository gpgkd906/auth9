import { describe, it, expect } from "vitest";
import { formatErrorMessage } from "~/lib/error-messages";

describe("formatErrorMessage", () => {
  // ============================================================================
  // Known error code mapping tests
  // ============================================================================

  it("maps invalid_slug error code", () => {
    const result = formatErrorMessage("invalid_slug");
    expect(result).toBe(
      "Slug can only contain lowercase letters, numbers, and hyphens. It cannot start or end with a hyphen."
    );
  });

  it("maps length error code", () => {
    const result = formatErrorMessage("length");
    expect(result).toBe("Length must be between 1-63 characters");
  });

  it("maps required error code", () => {
    const result = formatErrorMessage("required");
    expect(result).toBe("This field is required");
  });

  it("maps email error code", () => {
    const result = formatErrorMessage("email");
    expect(result).toBe("Please enter a valid email address");
  });

  it("maps 'already exists' error", () => {
    const result = formatErrorMessage("Tenant already exists");
    expect(result).toBe(
      "This value already exists. Please use a different one."
    );
  });

  it("maps 'not found' error", () => {
    const result = formatErrorMessage("Resource not found");
    expect(result).toBe("The requested resource was not found");
  });

  it("maps conflict error", () => {
    const result = formatErrorMessage("conflict");
    expect(result).toBe(
      "A resource with this identifier already exists"
    );
  });

  it("maps 'duplicate entry' database error", () => {
    const result = formatErrorMessage("duplicate entry for key 'name'");
    expect(result).toBe(
      "This name already exists. Please use a different one."
    );
  });

  it("maps MySQL error code 1062", () => {
    const result = formatErrorMessage("Error 1062: Duplicate entry");
    expect(result).toBe(
      "This name already exists. Please use a different one."
    );
  });

  // ============================================================================
  // Field name extraction with known error codes
  // ============================================================================

  it("extracts known field name with error code", () => {
    const result = formatErrorMessage("slug: invalid_slug");
    expect(result).toBe(
      "Slug: Slug can only contain lowercase letters, numbers, and hyphens. It cannot start or end with a hyphen."
    );
  });

  it("extracts email field name with error code", () => {
    const result = formatErrorMessage("email: required");
    expect(result).toBe("Email: This field is required");
  });

  it("extracts name field name with error code", () => {
    const result = formatErrorMessage("name: already exists");
    expect(result).toBe(
      "Name: This value already exists. Please use a different one."
    );
  });

  it("capitalizes unknown field names with error code", () => {
    const result = formatErrorMessage("username: required");
    expect(result).toBe("Username: This field is required");
  });

  // ============================================================================
  // Validation error format
  // ============================================================================

  it("handles 'field: Validation error: ...' format", () => {
    const result = formatErrorMessage(
      "slug: Validation error: must be alphanumeric [detail info]"
    );
    expect(result).toBe("Slug: must be alphanumeric");
  });

  it("strips bracket details from validation errors", () => {
    const result = formatErrorMessage(
      "name: Validation error: some_error [{min: 1, max: 63}]"
    );
    expect(result).toBe("Name: some_error");
  });

  it("handles validation error with unknown field name", () => {
    const result = formatErrorMessage(
      "description: Validation error: too long"
    );
    expect(result).toBe("Description: too long");
  });

  // ============================================================================
  // Simple "field: error" format
  // ============================================================================

  it("handles simple 'field: error' format with known field", () => {
    const result = formatErrorMessage("logo_url: must be a valid URL");
    expect(result).toBe("Logo URL: must be a valid URL");
  });

  it("handles simple 'field: error' format with unknown field", () => {
    const result = formatErrorMessage("hostname: invalid format");
    expect(result).toBe("Hostname: invalid format");
  });

  // ============================================================================
  // Fallback
  // ============================================================================

  it("returns raw message when no pattern matches", () => {
    const result = formatErrorMessage("Something went wrong");
    expect(result).toBe("Something went wrong");
  });

  it("returns empty string unchanged", () => {
    const result = formatErrorMessage("");
    expect(result).toBe("");
  });

  // ============================================================================
  // Case insensitivity
  // ============================================================================

  it("matches error codes case-insensitively", () => {
    const result = formatErrorMessage("ALREADY EXISTS in database");
    expect(result).toBe(
      "This value already exists. Please use a different one."
    );
  });

  it("matches NOT FOUND case-insensitively", () => {
    const result = formatErrorMessage("NOT FOUND");
    expect(result).toBe("The requested resource was not found");
  });
});
