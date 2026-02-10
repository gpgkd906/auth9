import { describe, it, expect } from "vitest";
import { getTokenType } from "./types/claims.js";
import type {
  IdentityClaims,
  TenantAccessClaims,
  ServiceClientClaims,
} from "./types/claims.js";

describe("getTokenType", () => {
  it("returns identity for auth9 audience", () => {
    const claims: IdentityClaims = {
      sub: "user-1",
      email: "test@example.com",
      iss: "https://auth9.test",
      aud: "auth9",
      iat: 1000000,
      exp: 1003600,
    };
    expect(getTokenType(claims)).toBe("identity");
  });

  it("returns serviceClient for auth9-service audience", () => {
    const claims: ServiceClientClaims = {
      sub: "service-1",
      email: "service@auth9.local",
      iss: "https://auth9.test",
      aud: "auth9-service",
      iat: 1000000,
      exp: 1003600,
    };
    expect(getTokenType(claims)).toBe("serviceClient");
  });

  it("returns tenantAccess for other audiences", () => {
    const claims: TenantAccessClaims = {
      sub: "user-1",
      email: "test@example.com",
      iss: "https://auth9.test",
      aud: "my-service",
      tenantId: "tenant-1",
      roles: ["admin"],
      permissions: ["user:read"],
      iat: 1000000,
      exp: 1003600,
    };
    expect(getTokenType(claims)).toBe("tenantAccess");
  });
});
