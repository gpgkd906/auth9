import { beforeEach, describe, expect, it, vi } from "vitest";

const { mockCreateRemoteJWKSet, mockJwtVerify, mockJwks } = vi.hoisted(() => ({
  mockCreateRemoteJWKSet: vi.fn(),
  mockJwtVerify: vi.fn(),
  mockJwks: vi.fn(),
}));

vi.mock("jose", () => ({
  createRemoteJWKSet: mockCreateRemoteJWKSet,
  jwtVerify: mockJwtVerify,
}));

import { TokenVerifier } from "./token-verifier.js";

describe("TokenVerifier", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCreateRemoteJWKSet.mockReturnValue(mockJwks);
  });

  it("verifies identity tokens and normalizes domain", async () => {
    mockJwtVerify.mockResolvedValue({
      payload: {
        sub: "user-1",
        sid: "sid-1",
        email: "user@example.com",
        name: "User One",
        iss: "https://auth9.example.com",
        aud: "auth9",
        iat: 100,
        exp: 200,
      },
    });

    const verifier = new TokenVerifier({
      domain: "https://auth9.example.com///",
      audience: "service-a",
    });

    const result = await verifier.verify("token-1");

    expect(mockCreateRemoteJWKSet).toHaveBeenCalledTimes(1);
    const [jwksUrl, jwksOptions] = mockCreateRemoteJWKSet.mock.calls[0];
    expect((jwksUrl as URL).href).toBe(
      "https://auth9.example.com/.well-known/jwks.json",
    );
    expect(jwksOptions).toEqual({
      cooldownDuration: 3600 * 1000,
      cacheMaxAge: 3600 * 1000,
    });

    expect(mockJwtVerify).toHaveBeenCalledWith(
      "token-1",
      mockJwks,
      expect.objectContaining({
        algorithms: ["RS256"],
        issuer: "https://auth9.example.com",
        audience: "service-a",
      }),
    );
    expect(result.tokenType).toBe("identity");
    expect(result.claims).toMatchObject({
      sub: "user-1",
      aud: "auth9",
      email: "user@example.com",
    });
  });

  it("verifies service client tokens", async () => {
    mockJwtVerify.mockResolvedValue({
      payload: {
        sub: "service-client",
        email: "svc@example.com",
        iss: "https://auth9.example.com",
        aud: "auth9-service",
        tenant_id: "tenant-1",
        iat: 100,
        exp: 200,
      },
    });

    const verifier = new TokenVerifier({
      domain: "https://auth9.example.com",
    });
    const result = await verifier.verify("token-2");

    expect(result.tokenType).toBe("serviceClient");
    expect(result.claims).toMatchObject({
      sub: "service-client",
      aud: "auth9-service",
      tenantId: "tenant-1",
    });
  });

  it("verifies tenant access tokens from aud array and defaults roles/permissions", async () => {
    mockJwtVerify.mockResolvedValue({
      payload: {
        sub: "user-2",
        email: "u2@example.com",
        iss: "https://auth9.example.com",
        aud: ["service-b"],
        tenant_id: "tenant-2",
        iat: 200,
        exp: 300,
      },
    });

    const verifier = new TokenVerifier({
      domain: "https://auth9.example.com",
    });
    const result = await verifier.verify("token-3");

    expect(result.tokenType).toBe("tenantAccess");
    expect(result.claims).toMatchObject({
      sub: "user-2",
      aud: "service-b",
      tenantId: "tenant-2",
      roles: [],
      permissions: [],
    });
  });

  it("does not pass audience to jwtVerify when config does not define it", async () => {
    mockJwtVerify.mockResolvedValue({
      payload: {
        sub: "user-3",
        email: "u3@example.com",
        iss: "https://auth9.example.com",
        aud: "auth9",
        iat: 1,
        exp: 2,
      },
    });

    const verifier = new TokenVerifier({ domain: "https://auth9.example.com" });
    await verifier.verify("token-4");

    const verifyOptions = mockJwtVerify.mock.calls[0][2] as Record<
      string,
      unknown
    >;
    expect(verifyOptions.issuer).toBe("https://auth9.example.com");
    expect(verifyOptions.algorithms).toEqual(["RS256"]);
    expect(verifyOptions).not.toHaveProperty("audience");
  });

  it("reuses cached JWKS across multiple verify calls", async () => {
    mockJwtVerify.mockResolvedValue({
      payload: {
        sub: "user-4",
        email: "u4@example.com",
        iss: "https://auth9.example.com",
        aud: "auth9",
        iat: 10,
        exp: 20,
      },
    });

    const verifier = new TokenVerifier({ domain: "https://auth9.example.com" });
    await verifier.verify("token-5");
    await verifier.verify("token-6");

    expect(mockCreateRemoteJWKSet).toHaveBeenCalledTimes(1);
    expect(mockJwtVerify).toHaveBeenCalledTimes(2);
  });

  it("propagates jwt verification errors", async () => {
    const error = new Error("invalid signature");
    mockJwtVerify.mockRejectedValue(error);

    const verifier = new TokenVerifier({ domain: "https://auth9.example.com" });
    await expect(verifier.verify("bad-token")).rejects.toBe(error);
  });
});
