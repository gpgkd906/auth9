import { describe, it, expect } from "vitest";
import { createMockToken, createMockAuth9 } from "./testing.js";

describe("createMockToken", () => {
  it("creates a valid JWT-format token with default claims", () => {
    const token = createMockToken();
    const parts = token.split(".");
    expect(parts.length).toBe(3);

    // Decode payload
    const payload = JSON.parse(
      Buffer.from(parts[1], "base64url").toString("utf-8"),
    );
    expect(payload.sub).toBe("test-user-id");
    expect(payload.email).toBe("test@example.com");
    expect(payload.tenantId).toBe("test-tenant-id");
    expect(payload.roles).toEqual(["user"]);
  });

  it("allows overriding claims", () => {
    const token = createMockToken({
      sub: "custom-user",
      email: "custom@example.com",
      roles: ["admin", "user"],
      permissions: ["user:read", "user:write"],
    });

    const parts = token.split(".");
    const payload = JSON.parse(
      Buffer.from(parts[1], "base64url").toString("utf-8"),
    );
    expect(payload.sub).toBe("custom-user");
    expect(payload.email).toBe("custom@example.com");
    expect(payload.roles).toEqual(["admin", "user"]);
    expect(payload.permissions).toEqual(["user:read", "user:write"]);
  });
});

describe("createMockAuth9", () => {
  it("verifyToken parses a mock token", () => {
    const mock = createMockAuth9();
    const token = createMockToken({ sub: "user-123" });
    const claims = mock.verifyToken(token);
    expect(claims.sub).toBe("user-123");
  });

  it("middleware injects auth info from token", () => {
    const mock = createMockAuth9();
    const token = createMockToken({
      sub: "user-abc",
      roles: ["admin"],
      permissions: ["user:read"],
    });

    const req = {
      headers: { authorization: `Bearer ${token}` },
      auth: undefined as unknown,
    };
    const res = {};
    let nextCalled = false;

    mock.middleware()(req as never, res as never, (() => {
      nextCalled = true;
    }) as never);

    expect(nextCalled).toBe(true);
    const auth = req.auth as {
      userId: string;
      roles: string[];
      permissions: string[];
      hasPermission: (p: string) => boolean;
      hasRole: (r: string) => boolean;
    };
    expect(auth.userId).toBe("user-abc");
    expect(auth.roles).toEqual(["admin"]);
    expect(auth.hasPermission("user:read")).toBe(true);
    expect(auth.hasPermission("user:write")).toBe(false);
    expect(auth.hasRole("admin")).toBe(true);
  });

  it("middleware uses default claims when no token provided", () => {
    const mock = createMockAuth9({
      defaultUser: { sub: "default-user", email: "default@test.com" },
    });

    const req = { headers: {}, auth: undefined as unknown };
    const res = {};
    let nextCalled = false;

    mock.middleware()(req as never, res as never, (() => {
      nextCalled = true;
    }) as never);

    expect(nextCalled).toBe(true);
    const auth = req.auth as { userId: string; email: string };
    expect(auth.userId).toBe("default-user");
    expect(auth.email).toBe("default@test.com");
  });

  // ============================================================================
  // Edge Case Tests
  // ============================================================================

  it("handles empty Bearer token gracefully", () => {
    const mock = createMockAuth9();
    const req = {
      headers: { authorization: "Bearer " },
      auth: undefined as unknown,
    };
    const res = {};

    mock.middleware()(req as never, res as never, (() => {}) as never);

    // Should handle empty token
    expect((req.auth as { userId: string }).userId).toBeDefined();
  });

  it("handles array audiences in token", () => {
    const token = createMockToken({
      aud: ["service1", "service2"] as unknown as string,
    });

    const parts = token.split(".");
    const payload = JSON.parse(
      Buffer.from(parts[1], "base64url").toString("utf-8"),
    );
    expect(payload.aud).toEqual(["service1", "service2"]);
  });

  it("handles missing optional claims fields", () => {
    const token = createMockToken({
      sub: "user-1",
      email: "test@example.com",
      iss: "https://auth9.test",
      aud: "auth9",
      iat: Date.now() / 1000,
      exp: (Date.now() / 1000) + 3600,
      // No displayName, roles, permissions
    } as Parameters<typeof createMockToken>[0]);

    const parts = token.split(".");
    const payload = JSON.parse(
      Buffer.from(parts[1], "base64url").toString("utf-8"),
    );
    expect(payload.sub).toBe("user-1");
    // Optional fields should be handled gracefully
  });
});
