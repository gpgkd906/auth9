/** Identity Token claims (issued after initial authentication) */
export interface IdentityClaims {
  /** Subject (user ID) */
  sub: string;
  /** Session ID (for session management) */
  sid?: string;
  /** Email */
  email: string;
  /** Display name */
  name?: string;
  /** Issuer */
  iss: string;
  /** Audience (always "auth9" for identity tokens) */
  aud: "auth9";
  /** Issued at (Unix timestamp) */
  iat: number;
  /** Expiration (Unix timestamp) */
  exp: number;
}

/** Tenant Access Token claims (issued after token exchange) */
export interface TenantAccessClaims {
  /** Subject (user ID) */
  sub: string;
  /** Email */
  email: string;
  /** Issuer */
  iss: string;
  /** Audience (service client_id) */
  aud: string;
  /** Tenant ID */
  tenantId: string;
  /** Roles in this tenant */
  roles: string[];
  /** Permissions (derived from roles) */
  permissions: string[];
  /** Issued at (Unix timestamp) */
  iat: number;
  /** Expiration (Unix timestamp) */
  exp: number;
}

/** Service Client Token claims (issued via client_credentials grant) */
export interface ServiceClientClaims {
  /** Subject (service ID) */
  sub: string;
  /** Service email */
  email: string;
  /** Issuer */
  iss: string;
  /** Audience (always "auth9-service") */
  aud: "auth9-service";
  /** The tenant_id this service belongs to (if any) */
  tenantId?: string;
  /** Issued at (Unix timestamp) */
  iat: number;
  /** Expiration (Unix timestamp) */
  exp: number;
}

/** Union type of all Auth9 JWT claims */
export type Auth9Claims = IdentityClaims | TenantAccessClaims | ServiceClientClaims;

/** Token type discriminator */
export type TokenType = "identity" | "tenantAccess" | "serviceClient";

/** Determine the token type from claims */
export function getTokenType(claims: Auth9Claims): TokenType {
  if (claims.aud === "auth9") return "identity";
  if (claims.aud === "auth9-service") return "serviceClient";
  return "tenantAccess";
}
