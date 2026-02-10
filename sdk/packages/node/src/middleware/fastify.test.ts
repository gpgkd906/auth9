import { describe, it, expect, vi, beforeEach } from "vitest";
import type { TenantAccessClaims } from "@auth9/core";

const mockVerify = vi.fn();

vi.mock("../token-verifier.js", () => ({
  TokenVerifier: class {
    verify = mockVerify;
  },
}));

import { auth9Plugin, type Auth9FastifyAuth } from "./fastify.js";

beforeEach(() => {
  vi.clearAllMocks();
});

const testClaims: TenantAccessClaims = {
  sub: "user-1",
  email: "user@test.com",
  iss: "https://auth9.test",
  aud: "test-service",
  tenantId: "tenant-1",
  roles: ["admin", "editor"],
  permissions: ["user:read", "user:write"],
  iat: Math.floor(Date.now() / 1000),
  exp: Math.floor(Date.now() / 1000) + 3600,
};

// Minimal Fastify mock that captures decorateRequest and addHook calls
function createMockFastify() {
  let hookHandler: ((
    request: { headers: Record<string, string | undefined>; auth9?: Auth9FastifyAuth },
    reply: { code: (s: number) => { send: (b: unknown) => void } },
  ) => Promise<void>) | null = null;

  const fastify = {
    decorateRequest: vi.fn(),
    addHook: vi.fn((name: string, handler: typeof hookHandler) => {
      if (name === "onRequest") hookHandler = handler;
    }),
  };

  function getHook() {
    return hookHandler!;
  }

  return { fastify, getHook };
}

function createRequest(token?: string) {
  const headers: Record<string, string | undefined> = {};
  if (token) headers.authorization = `Bearer ${token}`;
  return { headers, auth9: undefined as Auth9FastifyAuth | undefined };
}

function createReply() {
  return {
    code: (status: number) => ({
      send: (_body: unknown) => {},
    }),
  };
}

describe("Fastify auth9Plugin", () => {
  it("decorates request with auth9 property", async () => {
    const { fastify } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });
    expect(fastify.decorateRequest).toHaveBeenCalledWith("auth9", undefined);
  });

  it("registers onRequest hook", async () => {
    const { fastify } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });
    expect(fastify.addHook).toHaveBeenCalledWith("onRequest", expect.any(Function));
  });

  it("injects request.auth9 for valid token", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest("valid-token");
    await getHook()(request, createReply());

    expect(request.auth9).toBeDefined();
    expect(request.auth9!.userId).toBe("user-1");
    expect(request.auth9!.email).toBe("user@test.com");
    expect(request.auth9!.tokenType).toBe("tenantAccess");
    expect(request.auth9!.tenantId).toBe("tenant-1");
    expect(request.auth9!.roles).toEqual(["admin", "editor"]);
    expect(request.auth9!.permissions).toEqual(["user:read", "user:write"]);
  });

  it("leaves request.auth9 undefined without token", async () => {
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest();
    await getHook()(request, createReply());

    expect(request.auth9).toBeUndefined();
  });

  it("leaves request.auth9 undefined for invalid token", async () => {
    mockVerify.mockRejectedValue(new Error("invalid token"));
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest("bad-token");
    await getHook()(request, createReply());

    expect(request.auth9).toBeUndefined();
  });

  it("hasRole helper works correctly", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest("valid-token");
    await getHook()(request, createReply());

    expect(request.auth9!.hasRole("admin")).toBe(true);
    expect(request.auth9!.hasRole("superuser")).toBe(false);
  });

  it("hasPermission helper works correctly", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest("valid-token");
    await getHook()(request, createReply());

    expect(request.auth9!.hasPermission("user:read")).toBe(true);
    expect(request.auth9!.hasPermission("user:delete")).toBe(false);
  });

  it("hasAnyPermission helper works correctly", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest("valid-token");
    await getHook()(request, createReply());

    expect(request.auth9!.hasAnyPermission(["user:read", "user:delete"])).toBe(true);
    expect(request.auth9!.hasAnyPermission(["user:delete", "post:delete"])).toBe(false);
  });

  it("hasAllPermissions helper works correctly", async () => {
    mockVerify.mockResolvedValue({ claims: testClaims, tokenType: "tenantAccess" });
    const { fastify, getHook } = createMockFastify();
    await auth9Plugin(fastify, { domain: "http://test" });

    const request = createRequest("valid-token");
    await getHook()(request, createReply());

    expect(request.auth9!.hasAllPermissions(["user:read", "user:write"])).toBe(true);
    expect(request.auth9!.hasAllPermissions(["user:read", "user:delete"])).toBe(false);
  });
});
