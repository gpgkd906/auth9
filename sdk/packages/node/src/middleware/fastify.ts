import type { Auth9Claims, TokenType } from "@auth9/core";
import { TokenVerifier } from "../token-verifier.js";

export interface Auth9FastifyConfig {
  /** Auth9 Core URL */
  domain: string;
  /** Expected audience (service client_id) */
  audience?: string;
}

export interface Auth9FastifyAuth {
  userId: string;
  email: string;
  tokenType: TokenType;
  tenantId?: string;
  roles: string[];
  permissions: string[];
  raw: Auth9Claims;
  hasPermission(permission: string): boolean;
  hasRole(role: string): boolean;
  hasAnyPermission(permissions: string[]): boolean;
  hasAllPermissions(permissions: string[]): boolean;
}

/**
 * Fastify plugin for Auth9 token verification.
 *
 * Usage:
 * ```ts
 * import fastify from "fastify";
 * import { auth9Plugin } from "@auth9/node/middleware/fastify";
 *
 * const app = fastify();
 * app.register(auth9Plugin, {
 *   domain: process.env.AUTH9_DOMAIN!,
 *   audience: process.env.AUTH9_AUDIENCE!,
 * });
 * ```
 */
export async function auth9Plugin(
  fastify: {
    decorateRequest: (name: string, value: unknown) => void;
    addHook: (
      name: string,
      handler: (
        request: { headers: Record<string, string | undefined>; auth9?: Auth9FastifyAuth },
        reply: { code: (status: number) => { send: (body: unknown) => void } },
      ) => Promise<void>,
    ) => void;
  },
  options: Auth9FastifyConfig,
): Promise<void> {
  const verifier = new TokenVerifier({
    domain: options.domain,
    audience: options.audience,
  });

  fastify.decorateRequest("auth9", undefined);

  fastify.addHook("onRequest", async (request, reply) => {
    const authHeader = request.headers.authorization;
    if (!authHeader || !authHeader.startsWith("Bearer ")) {
      return; // No token - leave request.auth9 as undefined
    }

    const token = authHeader.slice(7);

    try {
      const { claims, tokenType } = await verifier.verify(token);
      const roles = "roles" in claims ? (claims.roles as string[]) : [];
      const permissions =
        "permissions" in claims ? (claims.permissions as string[]) : [];
      const tenantId =
        "tenantId" in claims ? (claims.tenantId as string) : undefined;

      request.auth9 = {
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
    } catch {
      // Token verification failed - leave request.auth9 as undefined
    }
  });
}
