import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Request, Response, NextFunction } from "express";
import type { TenantAccessClaims } from "@auth9/core";

const mockVerify = vi.fn();

// Mock TokenVerifier before importing the module under test
vi.mock("../token-verifier.js", () => ({
  TokenVerifier: class {
    verify = mockVerify;
  },
}));

import { auth9Middleware, requirePermission, requireRole } from "./express.js";

beforeEach(() => {
  vi.clearAllMocks();
});

function createReq(headers: Record<string, string> = {}): Request {
  return { headers, auth: undefined } as unknown as Request;
}

function createRes(): Response {
  return {} as unknown as Response;
}

function createNext(): NextFunction & { calls: unknown[] } {
  const calls: unknown[] = [];
  const next = ((err?: unknown) => {
    calls.push(err ?? "called");
  }) as NextFunction & { calls: unknown[] };
  next.calls = calls;
  return next;
}

const testClaims: TenantAccessClaims = {
  sub: "user-1",
  email: "user@test.com",
  iss: "https://auth9.test",
  aud: "test-service",
  tenantId: "tenant-1",
  roles: ["admin", "editor"],
  permissions: ["user:read", "user:write", "post:read"],
  iat: Math.floor(Date.now() / 1000),
  exp: Math.floor(Date.now() / 1000) + 3600,
};

// ── auth9Middleware ──────────────────────────────────────────────────

describe("auth9Middleware", () => {
  it("injects req.auth on valid token", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const mw = auth9Middleware({ domain: "http://test" });
    const req = createReq({ authorization: "Bearer valid-token" });
    const next = createNext();

    await mw(req, createRes(), next);

    expect(next.calls).toEqual(["called"]);
    expect(req.auth).toBeDefined();
    expect(req.auth!.userId).toBe("user-1");
    expect(req.auth!.email).toBe("user@test.com");
    expect(req.auth!.tokenType).toBe("tenantAccess");
    expect(req.auth!.tenantId).toBe("tenant-1");
    expect(req.auth!.roles).toEqual(["admin", "editor"]);
    expect(req.auth!.permissions).toEqual(["user:read", "user:write", "post:read"]);
  });

  it("rejects request without token (optional=false)", async () => {
    const mw = auth9Middleware({ domain: "http://test", optional: false });
    const req = createReq();
    const next = createNext();

    await mw(req, createRes(), next);

    expect(next.calls.length).toBe(1);
    expect(next.calls[0]).toHaveProperty("statusCode", 401);
    expect(req.auth).toBeUndefined();
  });

  it("rejects invalid token (optional=false)", async () => {
    mockVerify.mockRejectedValue(new Error("invalid"));
    const mw = auth9Middleware({ domain: "http://test", optional: false });
    const req = createReq({ authorization: "Bearer bad-token" });
    const next = createNext();

    await mw(req, createRes(), next);

    expect(next.calls.length).toBe(1);
    expect(next.calls[0]).toHaveProperty("statusCode", 401);
  });

  describe("optional mode", () => {
    it("allows requests without token", async () => {
      const mw = auth9Middleware({ domain: "http://test", optional: true });
      const req = createReq();
      const next = createNext();

      await mw(req, createRes(), next);

      expect(next.calls).toEqual(["called"]);
      expect(req.auth).toBeUndefined();
    });

    it("injects auth for valid token", async () => {
      mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
      const mw = auth9Middleware({ domain: "http://test", optional: true });
      const req = createReq({ authorization: "Bearer valid" });
      const next = createNext();

      await mw(req, createRes(), next);

      expect(next.calls).toEqual(["called"]);
      expect(req.auth).toBeDefined();
      expect(req.auth!.userId).toBe("user-1");
    });

    it("does not reject invalid token", async () => {
      mockVerify.mockRejectedValue(new Error("invalid"));
      const mw = auth9Middleware({ domain: "http://test", optional: true });
      const req = createReq({ authorization: "Bearer bad" });
      const next = createNext();

      await mw(req, createRes(), next);

      expect(next.calls).toEqual(["called"]);
      expect(req.auth).toBeUndefined();
    });
  });
});

// ── requirePermission ───────────────────────────────────────────────

describe("requirePermission", () => {
  function reqWithAuth(permissions: string[]): Request {
    const req = createReq();
    req.auth = {
      userId: "u1",
      email: "u@test.com",
      tokenType: "tenantAccess",
      roles: ["admin"],
      permissions,
      raw: testClaims,
      hasPermission(p) { return this.permissions.includes(p); },
      hasRole(r) { return this.roles.includes(r); },
      hasAnyPermission(ps) { return ps.some((p) => this.permissions.includes(p)); },
      hasAllPermissions(ps) { return ps.every((p) => this.permissions.includes(p)); },
    };
    return req;
  }

  it("allows single permission", () => {
    const mw = requirePermission("user:read");
    const next = createNext();
    mw(reqWithAuth(["user:read"]), createRes(), next);
    expect(next.calls).toEqual(["called"]);
  });

  it("requires all permissions (default mode)", () => {
    const mw = requirePermission(["user:read", "user:write"]);
    const next = createNext();
    mw(reqWithAuth(["user:read"]), createRes(), next);
    expect(next.calls.length).toBe(1);
    expect(next.calls[0]).toHaveProperty("statusCode", 403);
  });

  it("allows all permissions when present", () => {
    const mw = requirePermission(["user:read", "user:write"]);
    const next = createNext();
    mw(reqWithAuth(["user:read", "user:write", "post:read"]), createRes(), next);
    expect(next.calls).toEqual(["called"]);
  });

  it("requires any permission (any mode)", () => {
    const mw = requirePermission(["user:write", "user:admin"], { mode: "any" });
    const next = createNext();
    mw(reqWithAuth(["user:write"]), createRes(), next);
    expect(next.calls).toEqual(["called"]);
  });

  it("rejects when no matching permission (any mode)", () => {
    const mw = requirePermission(["user:write", "user:admin"], { mode: "any" });
    const next = createNext();
    mw(reqWithAuth(["post:read"]), createRes(), next);
    expect(next.calls[0]).toHaveProperty("statusCode", 403);
  });

  it("rejects unauthenticated requests with 401", () => {
    const mw = requirePermission("user:read");
    const req = createReq(); // no auth
    const next = createNext();
    mw(req, createRes(), next);
    expect(next.calls[0]).toHaveProperty("statusCode", 401);
  });
});

// ── requireRole ─────────────────────────────────────────────────────

describe("requireRole", () => {
  function reqWithAuth(roles: string[]): Request {
    const req = createReq();
    req.auth = {
      userId: "u1",
      email: "u@test.com",
      tokenType: "tenantAccess",
      roles,
      permissions: [],
      raw: testClaims,
      hasPermission(p) { return this.permissions.includes(p); },
      hasRole(r) { return this.roles.includes(r); },
      hasAnyPermission(ps) { return ps.some((p) => this.permissions.includes(p)); },
      hasAllPermissions(ps) { return ps.every((p) => this.permissions.includes(p)); },
    };
    return req;
  }

  it("allows single role", () => {
    const mw = requireRole("admin");
    const next = createNext();
    mw(reqWithAuth(["admin"]), createRes(), next);
    expect(next.calls).toEqual(["called"]);
  });

  it("requires all roles (default mode)", () => {
    const mw = requireRole(["admin", "editor"]);
    const next = createNext();
    mw(reqWithAuth(["admin"]), createRes(), next);
    expect(next.calls[0]).toHaveProperty("statusCode", 403);
  });

  it("allows all roles when present", () => {
    const mw = requireRole(["admin", "editor"]);
    const next = createNext();
    mw(reqWithAuth(["admin", "editor"]), createRes(), next);
    expect(next.calls).toEqual(["called"]);
  });

  it("requires any role (any mode)", () => {
    const mw = requireRole(["admin", "superuser"], { mode: "any" });
    const next = createNext();
    mw(reqWithAuth(["admin"]), createRes(), next);
    expect(next.calls).toEqual(["called"]);
  });

  it("rejects when no matching role (any mode)", () => {
    const mw = requireRole(["admin", "superuser"], { mode: "any" });
    const next = createNext();
    mw(reqWithAuth(["editor"]), createRes(), next);
    expect(next.calls[0]).toHaveProperty("statusCode", 403);
  });

  it("rejects unauthenticated requests with 401", () => {
    const mw = requireRole("admin");
    const req = createReq();
    const next = createNext();
    mw(req, createRes(), next);
    expect(next.calls[0]).toHaveProperty("statusCode", 401);
  });
});

// ── AuthInfo helpers ────────────────────────────────────────────────

describe("AuthInfo helpers", () => {
  it("hasPermission checks single permission", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const mw = auth9Middleware({ domain: "http://test" });
    const req = createReq({ authorization: "Bearer t" });
    const next = createNext();
    await mw(req, createRes(), next);

    expect(req.auth!.hasPermission("user:read")).toBe(true);
    expect(req.auth!.hasPermission("user:delete")).toBe(false);
  });

  it("hasRole checks single role", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const mw = auth9Middleware({ domain: "http://test" });
    const req = createReq({ authorization: "Bearer t" });
    const next = createNext();
    await mw(req, createRes(), next);

    expect(req.auth!.hasRole("admin")).toBe(true);
    expect(req.auth!.hasRole("superuser")).toBe(false);
  });

  it("hasAnyPermission checks any match", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const mw = auth9Middleware({ domain: "http://test" });
    const req = createReq({ authorization: "Bearer t" });
    const next = createNext();
    await mw(req, createRes(), next);

    expect(req.auth!.hasAnyPermission(["user:read", "user:delete"])).toBe(true);
    expect(req.auth!.hasAnyPermission(["user:delete", "post:delete"])).toBe(false);
  });

  it("hasAllPermissions checks all match", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const mw = auth9Middleware({ domain: "http://test" });
    const req = createReq({ authorization: "Bearer t" });
    const next = createNext();
    await mw(req, createRes(), next);

    expect(req.auth!.hasAllPermissions(["user:read", "user:write"])).toBe(true);
    expect(req.auth!.hasAllPermissions(["user:read", "user:delete"])).toBe(false);
  });
});
