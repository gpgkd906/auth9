import { describe, it, expect, vi, beforeEach } from "vitest";
import type { TenantAccessClaims } from "@auth9/core";

const mockVerify = vi.fn();

vi.mock("../token-verifier.js", () => ({
  TokenVerifier: class {
    verify = mockVerify;
  },
}));

import { auth9Middleware } from "./next.js";

beforeEach(() => {
  vi.clearAllMocks();
});

const testClaims: TenantAccessClaims = {
  sub: "user-1",
  email: "user@test.com",
  iss: "https://auth9.test",
  aud: "test-service",
  tenantId: "tenant-1",
  roles: ["admin"],
  permissions: ["user:read"],
  iat: Math.floor(Date.now() / 1000),
  exp: Math.floor(Date.now() / 1000) + 3600,
};

function createRequest(path: string, token?: string): Request {
  const headers: Record<string, string> = {};
  if (token) headers["authorization"] = `Bearer ${token}`;
  return new Request(`http://localhost${path}`, { headers });
}

describe("Next.js auth9Middleware", () => {
  it("allows public paths without token", async () => {
    const mw = auth9Middleware({
      domain: "http://test",
      publicPaths: ["/", "/login", "/api/health"],
    });

    const res = await mw(createRequest("/login"));
    expect(res.status).toBe(200);

    const resRoot = await mw(createRequest("/"));
    expect(resRoot.status).toBe(200);

    const resHealth = await mw(createRequest("/api/health"));
    expect(resHealth.status).toBe(200);
  });

  it("allows unprotected paths when protectedPaths is specified", async () => {
    const mw = auth9Middleware({
      domain: "http://test",
      protectedPaths: ["/api/users", "/api/admin"],
    });

    const res = await mw(createRequest("/public-page"));
    expect(res.status).toBe(200);
  });

  it("rejects protected paths without token (401)", async () => {
    const mw = auth9Middleware({
      domain: "http://test",
      publicPaths: ["/login"],
    });

    const res = await mw(createRequest("/api/users"));
    expect(res.status).toBe(401);

    const body = await res.json();
    expect(body.error).toBe("unauthorized");
    expect(body.message).toContain("Missing authorization token");
  });

  it("rejects invalid token with 401", async () => {
    mockVerify.mockRejectedValue(new Error("invalid token"));
    const mw = auth9Middleware({ domain: "http://test" });

    const res = await mw(createRequest("/api/users", "bad-token"));
    expect(res.status).toBe(401);

    const body = await res.json();
    expect(body.error).toBe("unauthorized");
  });

  it("injects x-auth9-* headers for valid token", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const mw = auth9Middleware({ domain: "http://test" });

    const res = await mw(createRequest("/api/users", "valid-token"));
    expect(res.status).toBe(200);
    expect(res.headers.get("x-auth9-user-id")).toBe("user-1");
    expect(res.headers.get("x-auth9-email")).toBe("user@test.com");
    expect(res.headers.get("x-auth9-token-type")).toBe("tenantAccess");
    expect(res.headers.get("x-auth9-tenant-id")).toBe("tenant-1");
    expect(res.headers.get("x-auth9-roles")).toBe(JSON.stringify(["admin"]));
    expect(res.headers.get("x-auth9-permissions")).toBe(JSON.stringify(["user:read"]));
  });

  it("does not set x-auth9-tenant-id for identity tokens", async () => {
    const identityClaims = {
      sub: "user-1",
      email: "user@test.com",
      iss: "https://auth9.test",
      aud: "auth9",
      iat: Math.floor(Date.now() / 1000),
      exp: Math.floor(Date.now() / 1000) + 3600,
    };
    mockVerify.mockResolvedValue({ claims: identityClaims, tokenType: "identity" });
    const mw = auth9Middleware({ domain: "http://test" });

    const res = await mw(createRequest("/api/users", "identity-token"));
    expect(res.status).toBe(200);
    expect(res.headers.get("x-auth9-user-id")).toBe("user-1");
    expect(res.headers.get("x-auth9-tenant-id")).toBeNull();
  });

  it("public path check uses path-segment prefix matching", async () => {
    const mw = auth9Middleware({
      domain: "http://test",
      publicPaths: ["/api/health"],
    });

    // /api/health/deep should match (path-segment prefix)
    const res = await mw(createRequest("/api/health/deep"));
    expect(res.status).toBe(200);

    // /api/healthz should NOT match (not a path-segment boundary)
    const res2 = await mw(createRequest("/api/healthz"));
    expect(res2.status).toBe(401);
  });

  it("root path '/' in publicPaths does not match all paths", async () => {
    const mw = auth9Middleware({
      domain: "http://test",
      publicPaths: ["/", "/login"],
    });

    // Root path itself should be public
    const resRoot = await mw(createRequest("/"));
    expect(resRoot.status).toBe(200);

    // /login should be public
    const resLogin = await mw(createRequest("/login"));
    expect(resLogin.status).toBe(200);

    // /api/users should NOT be public (not matched by "/")
    const resApi = await mw(createRequest("/api/users"));
    expect(resApi.status).toBe(401);

    // /dashboard should NOT be public
    const resDash = await mw(createRequest("/dashboard"));
    expect(resDash.status).toBe(401);
  });
});
