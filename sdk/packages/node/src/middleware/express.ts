import type { Request, Response, NextFunction, RequestHandler } from "express";
import type { Auth9Claims, TokenType } from "@auth9/core";
import { UnauthorizedError, ForbiddenError } from "@auth9/core";
import { TokenVerifier } from "../token-verifier.js";

/** Authenticated request information attached to req.auth */
export interface AuthInfo {
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

declare global {
  namespace Express {
    interface Request {
      auth?: AuthInfo;
    }
  }
}

export interface Auth9MiddlewareConfig {
  /** Auth9 Core URL */
  domain: string;
  /** Expected audience (service client_id) */
  audience?: string;
  /** If true, requests without tokens proceed with req.auth = undefined */
  optional?: boolean;
}

function createAuthInfo(claims: Auth9Claims, tokenType: TokenType): AuthInfo {
  const roles =
    "roles" in claims ? (claims.roles as string[]) : [];
  const permissions =
    "permissions" in claims ? (claims.permissions as string[]) : [];
  const tenantId = "tenantId" in claims ? (claims.tenantId as string) : undefined;

  return {
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
}

/** Express middleware that verifies Auth9 JWT tokens */
export function auth9Middleware(config: Auth9MiddlewareConfig): RequestHandler {
  const verifier = new TokenVerifier({
    domain: config.domain,
    audience: config.audience,
  });

  return async (req: Request, _res: Response, next: NextFunction) => {
    const authHeader = req.headers.authorization;

    if (!authHeader || !authHeader.startsWith("Bearer ")) {
      if (config.optional) return next();
      return next(new UnauthorizedError("Missing authorization token"));
    }

    const token = authHeader.slice(7);

    try {
      const { claims, tokenType } = await verifier.verify(token);
      req.auth = createAuthInfo(claims, tokenType);
      next();
    } catch (err) {
      if (config.optional) return next();
      next(new UnauthorizedError("Invalid or expired token"));
    }
  };
}

/** Require specific permissions */
export function requirePermission(
  permissions: string | string[],
  options: { mode: "all" | "any" } = { mode: "all" },
): RequestHandler {
  const permList = Array.isArray(permissions) ? permissions : [permissions];

  return (req: Request, _res: Response, next: NextFunction) => {
    if (!req.auth) {
      return next(new UnauthorizedError("Authentication required"));
    }

    const hasAccess =
      options.mode === "any"
        ? req.auth.hasAnyPermission(permList)
        : req.auth.hasAllPermissions(permList);

    if (!hasAccess) {
      return next(
        new ForbiddenError(
          `Missing required permission(s): ${permList.join(", ")}`,
        ),
      );
    }

    next();
  };
}

/** Require specific roles */
export function requireRole(
  roles: string | string[],
  options: { mode: "all" | "any" } = { mode: "all" },
): RequestHandler {
  const roleList = Array.isArray(roles) ? roles : [roles];

  return (req: Request, _res: Response, next: NextFunction) => {
    if (!req.auth) {
      return next(new UnauthorizedError("Authentication required"));
    }

    const hasAccess =
      options.mode === "any"
        ? roleList.some((r) => req.auth!.hasRole(r))
        : roleList.every((r) => req.auth!.hasRole(r));

    if (!hasAccess) {
      return next(
        new ForbiddenError(`Missing required role(s): ${roleList.join(", ")}`),
      );
    }

    next();
  };
}
