import { describe, it, expect } from "vitest";
import {
  Auth9Error,
  NotFoundError,
  UnauthorizedError,
  ForbiddenError,
  ValidationError,
  ConflictError,
  RateLimitError,
  BadRequestError,
  createErrorFromStatus,
} from "./errors.js";

describe("Auth9Error", () => {
  it("creates base error with all properties", () => {
    const err = new Auth9Error("test_code", "Test message", 500, {
      detail: "info",
    });
    expect(err.message).toBe("Test message");
    expect(err.code).toBe("test_code");
    expect(err.statusCode).toBe(500);
    expect(err.details).toEqual({ detail: "info" });
    expect(err.name).toBe("Auth9Error");
    expect(err instanceof Error).toBe(true);
  });
});

describe("typed error classes", () => {
  it("NotFoundError has correct status", () => {
    const err = new NotFoundError("User not found");
    expect(err.statusCode).toBe(404);
    expect(err.code).toBe("not_found");
    expect(err.name).toBe("NotFoundError");
    expect(err instanceof Auth9Error).toBe(true);
  });

  it("UnauthorizedError has correct status", () => {
    const err = new UnauthorizedError();
    expect(err.statusCode).toBe(401);
    expect(err.message).toBe("Unauthorized");
  });

  it("ForbiddenError has correct status", () => {
    const err = new ForbiddenError();
    expect(err.statusCode).toBe(403);
    expect(err.message).toBe("Forbidden");
  });

  it("ValidationError has correct status", () => {
    const err = new ValidationError("Email is required");
    expect(err.statusCode).toBe(422);
    expect(err.code).toBe("validation");
  });

  it("ConflictError has correct status", () => {
    const err = new ConflictError("Already exists");
    expect(err.statusCode).toBe(409);
    expect(err.code).toBe("conflict");
  });

  it("RateLimitError has correct status", () => {
    const err = new RateLimitError();
    expect(err.statusCode).toBe(429);
    expect(err.message).toBe("Rate limit exceeded");
  });

  it("BadRequestError has correct status", () => {
    const err = new BadRequestError("Invalid input");
    expect(err.statusCode).toBe(400);
    expect(err.code).toBe("bad_request");
  });
});

describe("createErrorFromStatus", () => {
  it("maps 400 to BadRequestError", () => {
    const err = createErrorFromStatus(400, { message: "Bad input" });
    expect(err).toBeInstanceOf(BadRequestError);
    expect(err.message).toBe("Bad input");
  });

  it("maps 401 to UnauthorizedError", () => {
    const err = createErrorFromStatus(401, { message: "No token" });
    expect(err).toBeInstanceOf(UnauthorizedError);
  });

  it("maps 403 to ForbiddenError", () => {
    const err = createErrorFromStatus(403, { message: "No access" });
    expect(err).toBeInstanceOf(ForbiddenError);
  });

  it("maps 404 to NotFoundError", () => {
    const err = createErrorFromStatus(404, { message: "Not found" });
    expect(err).toBeInstanceOf(NotFoundError);
  });

  it("maps 409 to ConflictError", () => {
    const err = createErrorFromStatus(409, { message: "Conflict" });
    expect(err).toBeInstanceOf(ConflictError);
  });

  it("maps 422 to ValidationError", () => {
    const err = createErrorFromStatus(422, { message: "Validation failed" });
    expect(err).toBeInstanceOf(ValidationError);
  });

  it("maps 429 to RateLimitError", () => {
    const err = createErrorFromStatus(429, { message: "Too many" });
    expect(err).toBeInstanceOf(RateLimitError);
  });

  it("maps unknown status to base Auth9Error", () => {
    const err = createErrorFromStatus(502, { error: "bad_gateway", message: "Server error" });
    expect(err).toBeInstanceOf(Auth9Error);
    expect(err.statusCode).toBe(502);
  });

  it("falls back to error field when message is missing", () => {
    const err = createErrorFromStatus(404, { error: "not_found" });
    expect(err.message).toBe("not_found");
  });
});
