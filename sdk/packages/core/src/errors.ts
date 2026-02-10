/** Base error class for all Auth9 SDK errors */
export class Auth9Error extends Error {
  constructor(
    public code: string,
    message: string,
    public statusCode: number,
    public details?: unknown,
  ) {
    super(message);
    this.name = "Auth9Error";
  }
}

/** Resource not found (HTTP 404) */
export class NotFoundError extends Auth9Error {
  constructor(message: string, details?: unknown) {
    super("not_found", message, 404, details);
    this.name = "NotFoundError";
  }
}

/** Authentication required (HTTP 401) */
export class UnauthorizedError extends Auth9Error {
  constructor(message = "Unauthorized", details?: unknown) {
    super("unauthorized", message, 401, details);
    this.name = "UnauthorizedError";
  }
}

/** Insufficient permissions (HTTP 403) */
export class ForbiddenError extends Auth9Error {
  constructor(message = "Forbidden", details?: unknown) {
    super("forbidden", message, 403, details);
    this.name = "ForbiddenError";
  }
}

/** Validation error (HTTP 422) */
export class ValidationError extends Auth9Error {
  constructor(message: string, details?: unknown) {
    super("validation", message, 422, details);
    this.name = "ValidationError";
  }
}

/** Resource conflict (HTTP 409) */
export class ConflictError extends Auth9Error {
  constructor(message: string, details?: unknown) {
    super("conflict", message, 409, details);
    this.name = "ConflictError";
  }
}

/** Rate limit exceeded (HTTP 429) */
export class RateLimitError extends Auth9Error {
  constructor(message = "Rate limit exceeded", details?: unknown) {
    super("rate_limit", message, 429, details);
    this.name = "RateLimitError";
  }
}

/** Bad request (HTTP 400) */
export class BadRequestError extends Auth9Error {
  constructor(message: string, details?: unknown) {
    super("bad_request", message, 400, details);
    this.name = "BadRequestError";
  }
}

/** Map HTTP status code to typed Auth9 error */
export function createErrorFromStatus(
  statusCode: number,
  errorBody: { error?: string; message?: string; details?: unknown },
): Auth9Error {
  const message = errorBody.message || errorBody.error || "Unknown error";
  const details = errorBody.details;

  switch (statusCode) {
    case 400:
      return new BadRequestError(message, details);
    case 401:
      return new UnauthorizedError(message, details);
    case 403:
      return new ForbiddenError(message, details);
    case 404:
      return new NotFoundError(message, details);
    case 409:
      return new ConflictError(message, details);
    case 422:
      return new ValidationError(message, details);
    case 429:
      return new RateLimitError(message, details);
    default:
      return new Auth9Error(
        errorBody.error || "unknown",
        message,
        statusCode,
        details,
      );
  }
}
