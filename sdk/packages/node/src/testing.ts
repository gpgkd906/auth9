import type { TenantAccessClaims, Auth9Claims, TokenType } from "@auth9/core";
import type { AuthInfo } from "./middleware/express.js";

const TEST_SIGNING_KEY = "auth9-test-signing-key-do-not-use-in-production";

/** Create a mock JWT token string for testing */
export function createMockToken(
  claims: Partial<TenantAccessClaims> = {},
): string {
  const now = Math.floor(Date.now() / 1000);
  const payload: TenantAccessClaims = {
    sub: claims.sub ?? "test-user-id",
    email: claims.email ?? "test@example.com",
    iss: claims.iss ?? "https://auth9.test",
    aud: claims.aud ?? "test-service",
    tenantId: claims.tenantId ?? "test-tenant-id",
    roles: claims.roles ?? ["user"],
    permissions: claims.permissions ?? [],
    iat: claims.iat ?? now,
    exp: claims.exp ?? now + 3600,
  };

  // Create a simple base64url-encoded JWT (not cryptographically signed - for testing only)
  const header = base64url(JSON.stringify({ alg: "HS256", typ: "JWT" }));
  const body = base64url(JSON.stringify(payload));
  const signature = base64url(TEST_SIGNING_KEY);

  return `${header}.${body}.${signature}`;
}

export interface MockAuth9Config {
  defaultUser?: Partial<TenantAccessClaims>;
}

type MockMiddlewareFn = (
  req: { headers: Record<string, string | undefined>; auth?: AuthInfo },
  res: unknown,
  next: () => void,
) => void;

export interface MockAuth9 {
  /** Express-compatible middleware that injects mock auth info without token verification */
  middleware(): MockMiddlewareFn;
  /** Parse a mock token and return claims (no signature verification) */
  verifyToken(token: string): Auth9Claims;
}

/** Create a mock Auth9 instance for testing */
export function createMockAuth9(config: MockAuth9Config = {}): MockAuth9 {
  const defaultClaims = buildDefaultClaims(config.defaultUser);

  return {
    middleware() {
      return (
        req: { headers: Record<string, string | undefined>; auth?: AuthInfo },
        _res: unknown,
        next: () => void,
      ) => {
        const authHeader = req.headers.authorization;
        let claims: Auth9Claims = defaultClaims;

        if (authHeader?.startsWith("Bearer ")) {
          const token = authHeader.slice(7);
          try {
            claims = parseMockToken(token);
          } catch {
            // Use default claims if token parsing fails
          }
        }

        const tokenType: TokenType =
          claims.aud === "auth9"
            ? "identity"
            : claims.aud === "auth9-service"
              ? "serviceClient"
              : "tenantAccess";

        const roles = "roles" in claims ? (claims as TenantAccessClaims).roles : [];
        const permissions =
          "permissions" in claims ? (claims as TenantAccessClaims).permissions : [];
        const tenantId =
          "tenantId" in claims
            ? (claims as TenantAccessClaims).tenantId
            : undefined;

        req.auth = {
          userId: claims.sub,
          email: claims.email,
          tokenType,
          tenantId,
          roles,
          permissions,
          raw: claims,
          hasPermission(p: string) {
            return this.permissions.includes(p);
          },
          hasRole(r: string) {
            return this.roles.includes(r);
          },
          hasAnyPermission(ps: string[]) {
            return ps.some((p) => this.permissions.includes(p));
          },
          hasAllPermissions(ps: string[]) {
            return ps.every((p) => this.permissions.includes(p));
          },
        };

        next();
      };
    },

    verifyToken(token: string): Auth9Claims {
      return parseMockToken(token);
    },
  };
}

function buildDefaultClaims(
  overrides?: Partial<TenantAccessClaims>,
): TenantAccessClaims {
  const now = Math.floor(Date.now() / 1000);
  return {
    sub: overrides?.sub ?? "test-user-id",
    email: overrides?.email ?? "test@example.com",
    iss: overrides?.iss ?? "https://auth9.test",
    aud: overrides?.aud ?? "test-service",
    tenantId: overrides?.tenantId ?? "test-tenant-id",
    roles: overrides?.roles ?? ["user"],
    permissions: overrides?.permissions ?? [],
    iat: overrides?.iat ?? now,
    exp: overrides?.exp ?? now + 3600,
  };
}

function parseMockToken(token: string): Auth9Claims {
  const parts = token.split(".");
  if (parts.length !== 3) throw new Error("Invalid mock token format");
  const payload = JSON.parse(
    Buffer.from(parts[1], "base64url").toString("utf-8"),
  );
  return payload as Auth9Claims;
}

function base64url(str: string): string {
  return Buffer.from(str)
    .toString("base64url");
}
