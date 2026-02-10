import * as jose from "jose";
import type {
  Auth9Claims,
  IdentityClaims,
  TenantAccessClaims,
  ServiceClientClaims,
  TokenType,
} from "@auth9/core";

export interface TokenVerifierConfig {
  /** Auth9 Core URL (e.g., "https://auth9.example.com") */
  domain: string;
  /** Expected audience (service client_id). If not set, audience is not validated. */
  audience?: string;
  /** JWKS cache TTL in seconds (default: 3600) */
  jwksCacheTtl?: number;
  /** Allowed algorithms (default: ["RS256"]) */
  algorithms?: string[];
}

export interface VerifyResult {
  /** Decoded claims */
  claims: Auth9Claims;
  /** Token type */
  tokenType: TokenType;
}

export class TokenVerifier {
  private jwks: ReturnType<typeof jose.createRemoteJWKSet> | null = null;
  private config: Required<
    Pick<TokenVerifierConfig, "domain" | "jwksCacheTtl" | "algorithms">
  > &
    Pick<TokenVerifierConfig, "audience">;

  constructor(config: TokenVerifierConfig) {
    this.config = {
      domain: config.domain.replace(/\/+$/, ""),
      audience: config.audience,
      jwksCacheTtl: config.jwksCacheTtl ?? 3600,
      algorithms: config.algorithms ?? ["RS256"],
    };
  }

  private getJwks(): ReturnType<typeof jose.createRemoteJWKSet> {
    if (!this.jwks) {
      const url = new URL(
        "/.well-known/jwks.json",
        this.config.domain,
      );
      this.jwks = jose.createRemoteJWKSet(url, {
        cooldownDuration: this.config.jwksCacheTtl * 1000,
        cacheMaxAge: this.config.jwksCacheTtl * 1000,
      });
    }
    return this.jwks!;
  }

  /** Verify a JWT token and return typed claims */
  async verify(token: string): Promise<VerifyResult> {
    const jwks = this.getJwks();

    const { payload } = await jose.jwtVerify(token, jwks, {
      algorithms: this.config.algorithms,
      issuer: this.config.domain,
      ...(this.config.audience ? { audience: this.config.audience } : {}),
    });

    const aud = payload.aud;
    const audStr = Array.isArray(aud) ? aud[0] : aud;

    if (audStr === "auth9") {
      const claims: IdentityClaims = {
        sub: payload.sub!,
        sid: payload.sid as string | undefined,
        email: payload.email as string,
        name: payload.name as string | undefined,
        iss: payload.iss!,
        aud: "auth9",
        iat: payload.iat!,
        exp: payload.exp!,
      };
      return { claims, tokenType: "identity" };
    }

    if (audStr === "auth9-service") {
      const claims: ServiceClientClaims = {
        sub: payload.sub!,
        email: payload.email as string,
        iss: payload.iss!,
        aud: "auth9-service",
        tenantId: payload.tenant_id as string | undefined,
        iat: payload.iat!,
        exp: payload.exp!,
      };
      return { claims, tokenType: "serviceClient" };
    }

    // Tenant Access Token
    const claims: TenantAccessClaims = {
      sub: payload.sub!,
      email: payload.email as string,
      iss: payload.iss!,
      aud: audStr!,
      tenantId: payload.tenant_id as string,
      roles: (payload.roles as string[]) ?? [],
      permissions: (payload.permissions as string[]) ?? [],
      iat: payload.iat!,
      exp: payload.exp!,
    };
    return { claims, tokenType: "tenantAccess" };
  }
}
