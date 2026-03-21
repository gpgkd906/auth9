import { describe, it, expect } from "vitest";
import { formatErrorMessage, mapApiError } from "~/lib/error-messages";
import { ApiResponseError } from "~/services/api/client";

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
    expect(result).toBe("Length must be between 1 and 63 characters.");
  });

  it("maps required error code", () => {
    const result = formatErrorMessage("required");
    expect(result).toBe("This field is required.");
  });

  it("maps email error code", () => {
    const result = formatErrorMessage("email");
    expect(result).toBe("Please enter a valid email address.");
  });

  it("maps 'already exists' error", () => {
    const result = formatErrorMessage("Tenant already exists");
    expect(result).toBe(
      "This value already exists. Please use a different one."
    );
  });

  it("maps 'not found' error", () => {
    const result = formatErrorMessage("Resource not found");
    expect(result).toBe("The requested resource was not found.");
  });

  it("maps conflict error", () => {
    const result = formatErrorMessage("conflict");
    expect(result).toBe("A resource with this identifier already exists.");
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
    expect(result).toBe("Email: This field is required.");
  });

  it("extracts name field name with error code", () => {
    const result = formatErrorMessage("name: already exists");
    expect(result).toBe(
      "Name: This value already exists. Please use a different one."
    );
  });

  it("capitalizes unknown field names with error code", () => {
    const result = formatErrorMessage("username: required");
    expect(result).toBe("Username: This field is required.");
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
    expect(result).toBe("The requested resource was not found.");
  });
});

// =============================================================================
// mapApiError
// =============================================================================

function makeApiError(code: string, message: string, status = 400): ApiResponseError {
  return new ApiResponseError({ error: code, message }, status);
}

describe("mapApiError", () => {
  // ===========================================================================
  // Known error code mapping (ApiResponseError)
  // ===========================================================================

  it("maps not_found to localized message", () => {
    const err = makeApiError("not_found", "Tenant not found", 404);
    expect(mapApiError(err)).toBe("The requested resource was not found.");
  });

  it("maps bad_request to localized message", () => {
    const err = makeApiError("bad_request", "Invalid input", 400);
    expect(mapApiError(err)).toBe("The request is invalid. Please check your input.");
  });

  it("maps expired password reset token to a specific localized message", () => {
    const err = makeApiError("bad_request", "Invalid or expired reset token", 400);
    expect(mapApiError(err)).toBe(
      "This reset link has expired. Please request a new one."
    );
  });

  it("maps unauthorized to localized message", () => {
    const err = makeApiError("unauthorized", "Token expired", 401);
    expect(mapApiError(err)).toBe("Your session has expired. Please sign in again.");
  });

  it("maps forbidden to localized message", () => {
    const err = makeApiError("forbidden", "Access denied", 403);
    expect(mapApiError(err)).toBe("You do not have permission to perform this action.");
  });

  it("maps conflict to localized message", () => {
    const err = makeApiError("conflict", "Resource already exists", 409);
    expect(mapApiError(err)).toBe("A resource with this identifier already exists.");
  });

  it("maps database_error to server error", () => {
    const err = makeApiError("database_error", "A database error occurred", 500);
    expect(mapApiError(err)).toBe("A server error occurred. Please try again later.");
  });

  it("maps cache_error to server error", () => {
    const err = makeApiError("cache_error", "A cache error occurred", 500);
    expect(mapApiError(err)).toBe("A server error occurred. Please try again later.");
  });

  it("maps jwt_error to session expired", () => {
    const err = makeApiError("jwt_error", "Invalid or expired token", 401);
    expect(mapApiError(err)).toBe("Your session has expired. Please sign in again.");
  });

  it("maps identity_backend_error to auth service error", () => {
    const err = makeApiError("identity_backend_error", "Authentication service error", 502);
    expect(mapApiError(err)).toBe(
      "The authentication service is temporarily unavailable. Please try again later."
    );
  });

  it("maps internal_error to server error", () => {
    const err = makeApiError("internal_error", "An internal error occurred", 500);
    expect(mapApiError(err)).toBe("A server error occurred. Please try again later.");
  });

  it("maps action_execution_failed to server error", () => {
    const err = makeApiError("action_execution_failed", "Script failed", 500);
    expect(mapApiError(err)).toBe("A server error occurred. Please try again later.");
  });

  it("maps rate_limited to rate limit message", () => {
    const err = makeApiError("rate_limited", "Too many requests", 429);
    expect(mapApiError(err)).toBe("Too many requests. Please wait a moment and try again.");
  });

  it("maps method_not_allowed to bad request", () => {
    const err = makeApiError("method_not_allowed", "Method not allowed", 405);
    expect(mapApiError(err)).toBe("The request is invalid. Please check your input.");
  });

  it("maps validation_error to bad request", () => {
    const err = makeApiError("validation_error", "Validation failed", 422);
    expect(mapApiError(err)).toBe("The request is invalid. Please check your input.");
  });

  // ===========================================================================
  // Validation code delegates to formatErrorMessage
  // ===========================================================================

  it("delegates validation code to formatErrorMessage", () => {
    const err = makeApiError("validation", "slug: invalid_slug", 422);
    expect(mapApiError(err)).toBe(
      "Slug: Slug can only contain lowercase letters, numbers, and hyphens. It cannot start or end with a hyphen."
    );
  });

  it("delegates validation code with simple field error", () => {
    const err = makeApiError("validation", "email: required", 422);
    expect(mapApiError(err)).toBe("Email: This field is required.");
  });

  // ===========================================================================
  // Unknown ApiResponseError code
  // ===========================================================================

  it("falls back to formatErrorMessage for unknown ApiResponseError code", () => {
    const err = makeApiError("some_new_code", "already exists", 400);
    expect(mapApiError(err)).toBe(
      "This value already exists. Please use a different one."
    );
  });

  // ===========================================================================
  // Plain Error fallback
  // ===========================================================================

  it("delegates plain Error to formatErrorMessage", () => {
    const err = new Error("Resource not found");
    expect(mapApiError(err)).toBe("The requested resource was not found.");
  });

  it("delegates plain Error with unmatched message", () => {
    const err = new Error("Something unexpected happened");
    expect(mapApiError(err)).toBe("Something unexpected happened");
  });

  // ===========================================================================
  // Non-Error fallback
  // ===========================================================================

  it("returns generic unknown error for non-Error values", () => {
    expect(mapApiError("string error")).toBe("Something went wrong. Please try again.");
    expect(mapApiError(42)).toBe("Something went wrong. Please try again.");
    expect(mapApiError(null)).toBe("Something went wrong. Please try again.");
    expect(mapApiError(undefined)).toBe("Something went wrong. Please try again.");
  });

  // ===========================================================================
  // Locale support
  // ===========================================================================

  it("maps error to zh-CN locale", () => {
    const err = makeApiError("forbidden", "Access denied", 403);
    expect(mapApiError(err, "zh-CN")).toBe("您没有权限执行此操作。");
  });

  it("maps error to ja locale", () => {
    const err = makeApiError("forbidden", "Access denied", 403);
    expect(mapApiError(err, "ja")).toBe("この操作を実行する権限がありません。");
  });

  it("maps not_found to zh-CN locale", () => {
    const err = makeApiError("not_found", "User not found", 404);
    expect(mapApiError(err, "zh-CN")).toBe("请求的资源不存在。");
  });

  it("maps rate_limited to ja locale", () => {
    const err = makeApiError("rate_limited", "Too many requests", 429);
    expect(mapApiError(err, "ja")).toBe(
      "リクエストが多すぎます。しばらく待ってから再度お試しください。"
    );
  });

  it("maps unknown error to zh-CN locale", () => {
    expect(mapApiError(null, "zh-CN")).toBe("发生未知错误，请重试。");
  });

  it("maps unknown error to ja locale", () => {
    expect(mapApiError(null, "ja")).toBe("エラーが発生しました。再度お試しください。");
  });

  it("delegates validation code to formatErrorMessage with zh-CN", () => {
    const err = makeApiError("validation", "email: required", 422);
    expect(mapApiError(err, "zh-CN")).toBe("邮箱: 此字段为必填项。");
  });
});
