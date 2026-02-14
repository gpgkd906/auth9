import { TokenVerifier } from "../token-verifier.js";

export interface Auth9NextConfig {
  /** Auth9 Core URL */
  domain: string;
  /** Expected audience (service client_id) */
  audience?: string;
  /** Paths that require authentication (default: all paths) */
  protectedPaths?: string[];
  /** Paths that are always public */
  publicPaths?: string[];
}

/**
 * Next.js middleware factory for Auth9 token verification.
 *
 * Usage in middleware.ts:
 * ```ts
 * import { auth9Middleware } from "@auth9/node/middleware/next";
 *
 * export default auth9Middleware({
 *   domain: process.env.AUTH9_DOMAIN!,
 *   audience: process.env.AUTH9_AUDIENCE!,
 *   publicPaths: ["/", "/login", "/api/health"],
 * });
 * ```
 */
export function auth9Middleware(config: Auth9NextConfig) {
  const verifier = new TokenVerifier({
    domain: config.domain,
    audience: config.audience,
  });

  return async (req: Request): Promise<Response> => {
    const url = new URL(req.url);
    const path = url.pathname;

    // Skip public paths (exact match or path-segment prefix match)
    if (config.publicPaths?.some((p) => path === p || (p !== "/" && path.startsWith(p + "/")))) {
      return new Response(null, { status: 200 });
    }

    // Check protected paths (if specified, only protect those)
    if (
      config.protectedPaths &&
      !config.protectedPaths.some((p) => path === p || (p !== "/" && path.startsWith(p + "/")))
    ) {
      return new Response(null, { status: 200 });
    }

    const authHeader = req.headers.get("authorization");
    if (!authHeader || !authHeader.startsWith("Bearer ")) {
      return new Response(
        JSON.stringify({ error: "unauthorized", message: "Missing authorization token" }),
        { status: 401, headers: { "Content-Type": "application/json" } },
      );
    }

    const token = authHeader.slice(7);

    try {
      const { claims, tokenType } = await verifier.verify(token);

      // Forward auth info via headers to the downstream handler
      const headers = new Headers();
      headers.set("x-auth9-user-id", claims.sub);
      headers.set("x-auth9-email", claims.email);
      headers.set("x-auth9-token-type", tokenType);
      if ("tenantId" in claims && claims.tenantId) {
        headers.set("x-auth9-tenant-id", claims.tenantId as string);
      }
      if ("roles" in claims) {
        headers.set("x-auth9-roles", JSON.stringify(claims.roles));
      }
      if ("permissions" in claims) {
        headers.set("x-auth9-permissions", JSON.stringify(claims.permissions));
      }

      return new Response(null, { status: 200, headers });
    } catch {
      return new Response(
        JSON.stringify({ error: "unauthorized", message: "Invalid or expired token" }),
        { status: 401, headers: { "Content-Type": "application/json" } },
      );
    }
  };
}
