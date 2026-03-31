import { createCookie, redirect } from "react-router";
import { authApi } from "~/services/api";
import { ApiResponseError } from "~/services/api/client";
import { getRedis } from "~/services/redis.server";

const isProduction = process.env.NODE_ENV === "production";

// Use SECURE_COOKIES env var when explicitly set to allow HTTP-based Docker dev deployments
// (NODE_ENV=production is used for Remix build optimizations even in HTTP-only dev environments)
const isSecureCookies =
  process.env.SECURE_COOKIES !== undefined
    ? process.env.SECURE_COOKIES === "true"
    : isProduction;
const SESSION_MAX_AGE = 8 * 60 * 60;
const DEFAULT_SESSION_SECRET = "default-secret-change-me"; // pragma: allowlist secret

const SESSION_PREFIX = "portal:session:";
const SESSION_TTL = SESSION_MAX_AGE;

function resolveSessionSecret(): string {
  const sessionSecret = process.env.SESSION_SECRET;
  if (isProduction && (!sessionSecret || sessionSecret === DEFAULT_SESSION_SECRET)) {
    throw new Error(
      "SESSION_SECRET must be set to a strong non-default value in production"
    );
  }
  return sessionSecret || DEFAULT_SESSION_SECRET;
}

const cookieSecret = resolveSessionSecret();

export const NO_STORE_HEADERS: HeadersInit = {
  "Cache-Control": "no-store, no-cache, must-revalidate, private",
  Pragma: "no-cache",
  Expires: "0",
};

// Cookie now only stores an opaque session ID — all session data lives in Redis
export const sessionCookie = createCookie("auth9_session", {
  secrets: [cookieSecret],
  path: "/",
  sameSite: "lax",
  httpOnly: true,
  secure: isSecureCookies,
  maxAge: SESSION_MAX_AGE,
});

export interface SessionData {
  /** @internal — not persisted to Redis, used to track session ID across spreads */
  _sid?: string;
  accessToken?: string;
  identityAccessToken?: string;
  tenantAccessToken?: string;
  refreshToken?: string;
  idToken?: string;
  expiresAt?: number;
  identityExpiresAt?: number;
  tenantExpiresAt?: number;
  activeTenantId?: string;
}

export const oauthStateCookie = createCookie("oauth_state", {
  secrets: [cookieSecret],
  path: "/",
  sameSite: "lax",
  httpOnly: true,
  secure: isSecureCookies,
  maxAge: 5 * 60,
});

export interface OAuthStateData {
  state: string;
  codeVerifier?: string;
}

export async function serializeOAuthState(
  state: string,
  codeVerifier?: string
): Promise<string> {
  return oauthStateCookie.serialize({ state, codeVerifier });
}

export async function getOAuthState(
  request: Request
): Promise<OAuthStateData | null> {
  const cookieHeader = request.headers.get("Cookie");
  const value = await oauthStateCookie.parse(cookieHeader);

  // Backward compatibility: old cookies may be plain strings
  if (typeof value === "string" && value.length > 0) {
    return { state: value };
  }
  if (value && typeof value === "object" && typeof value.state === "string") {
    return value as OAuthStateData;
  }
  return null;
}

export async function clearOAuthStateCookie(): Promise<string> {
  return oauthStateCookie.serialize("", { maxAge: 0 });
}

// ==================== Redis Session Helpers ====================

async function loadSession(sid: string): Promise<SessionData | null> {
  try {
    const raw = await getRedis().get(SESSION_PREFIX + sid);
    if (!raw) return null;
    return JSON.parse(raw);
  } catch (err) {
    console.error("[auth9-session] Failed to load session from Redis:", (err as Error)?.message || err);
    return null; // Redis unavailable → treat as no session
  }
}

async function saveSession(sid: string, data: SessionData): Promise<string> {
  try {
    // Strip internal _sid before persisting
    const { _sid, ...toStore } = data;
    void _sid;
    await getRedis().set(
      SESSION_PREFIX + sid,
      JSON.stringify(toStore),
      "EX",
      SESSION_TTL
    );
  } catch (err) {
    console.error("[auth9-session] Failed to save session to Redis — cookie set but data not persisted:", (err as Error)?.message || err);
    // Redis unavailable — cookie is set but data won't persist;
    // next request will see no session and redirect to login
  }
  return sessionCookie.serialize({ sid });
}

async function removeSession(sid: string): Promise<void> {
  try {
    await getRedis().del(SESSION_PREFIX + sid);
  } catch (err) {
    console.warn("[auth9-session] Failed to remove session from Redis (best-effort):", (err as Error)?.message || err);
  }
}

// ==================== Session Normalization ====================

function normalizeSession(session: SessionData | null): SessionData | null {
  if (!session) return null;
  const identityAccessToken = session.identityAccessToken || session.accessToken;
  const identityExpiresAt = session.identityExpiresAt || session.expiresAt;
  return {
    ...session,
    identityAccessToken,
    accessToken: identityAccessToken,
    identityExpiresAt,
    expiresAt: identityExpiresAt,
  };
}

// ==================== Public Session API ====================

export async function getSession(request: Request): Promise<SessionData | null> {
  const cookieHeader = request.headers.get("Cookie");
  const cookie = await sessionCookie.parse(cookieHeader);
  if (!cookie || !cookie.sid) return null;
  const raw = await loadSession(cookie.sid);
  const normalized = normalizeSession(raw);
  if (normalized) normalized._sid = cookie.sid;
  return normalized;
}

export async function commitSession(session: SessionData): Promise<string> {
  const sid = session._sid || crypto.randomUUID();
  const normalized = normalizeSession(session) || session;
  // Strip redundant alias fields (reconstructed by normalizeSession on read)
  const { _sid, accessToken, expiresAt, ...compact } = normalized;
  void _sid;
  void accessToken;
  void expiresAt;
  return saveSession(sid, compact as SessionData);
}

export async function destroySession(session: SessionData): Promise<string> {
  const sid = session._sid;
  if (sid) await removeSession(sid);
  return sessionCookie.serialize("", { maxAge: 0 });
}

// ==================== Token Lifecycle ====================

function isTokenExpired(expiresAt?: number): boolean {
  if (!expiresAt) return true;
  return Date.now() > expiresAt - 60000;
}

const refreshLocks = new Map<string, Promise<SessionData | null>>();

async function refreshIdentityToken(session: SessionData): Promise<SessionData | null> {
  if (!session.refreshToken) return null;
  const lockKey = session.refreshToken || session.identityAccessToken || session.accessToken || "";

  if (refreshLocks.has(lockKey)) {
    return await refreshLocks.get(lockKey)!;
  }

  const refreshPromise = (async () => {
    try {
      const tokenUrl = `${process.env.AUTH9_CORE_URL || "http://localhost:8080"}/api/v1/auth/token`;
      const response = await fetch(tokenUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          grant_type: "refresh_token",
          client_id: process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal",
          refresh_token: session.refreshToken,
        }),
      });

      if (!response.ok) return null;
      const data = await response.json();
      const identityAccessToken = data.access_token as string;
      const identityExpiresAt = Date.now() + (data.expires_in * 1000);

      return {
        ...session,
        accessToken: identityAccessToken,
        identityAccessToken,
        identityExpiresAt,
        expiresAt: identityExpiresAt,
        refreshToken: data.refresh_token || session.refreshToken,
        idToken: data.id_token || session.idToken,
      };
    } catch {
      return null;
    } finally {
      refreshLocks.delete(lockKey);
    }
  })();

  refreshLocks.set(lockKey, refreshPromise);
  return await refreshPromise;
}

async function exchangeTenantToken(
  session: SessionData,
  tenantId: string
): Promise<SessionData | null> {
  const identityToken = session.identityAccessToken || session.accessToken;
  if (!identityToken || !tenantId) return null;

  const serviceId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  try {
    const data = await authApi.exchangeTenantToken(tenantId, serviceId, identityToken);
    return {
      ...session,
      activeTenantId: tenantId,
      tenantAccessToken: data.access_token,
      tenantExpiresAt: Date.now() + (data.expires_in * 1000),
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`tenant_exchange_failed:${message}`, { cause: error });
  }
}

async function ensureIdentitySession(
  session: SessionData
): Promise<{ session: SessionData | null; updated: boolean }> {
  const normalized = normalizeSession(session);
  if (!normalized) return { session: null, updated: false };
  if (!normalized.identityAccessToken) return { session: null, updated: false };
  if (!isTokenExpired(normalized.identityExpiresAt)) {
    return { session: normalized, updated: false };
  }

  const refreshed = await refreshIdentityToken(normalized);
  return { session: refreshed, updated: !!refreshed };
}

async function ensureTenantSession(
  session: SessionData
): Promise<{ session: SessionData | null; updated: boolean; degraded?: boolean }> {
  if (!session.activeTenantId) {
    return { session, updated: false };
  }
  if (session.tenantAccessToken && !isTokenExpired(session.tenantExpiresAt)) {
    return { session, updated: false };
  }
  try {
    const exchanged = await exchangeTenantToken(session, session.activeTenantId);
    return { session: exchanged, updated: true };
  } catch (error) {
    const cause = error instanceof Error ? error.cause : undefined;
    const isRecoverableNetworkFailure =
      cause instanceof TypeError ||
      (cause instanceof ApiResponseError && cause.status >= 500);

    if (isRecoverableNetworkFailure) {
      // Keep tenant selection on transient failures and degrade to identity token.
      return { session, updated: false, degraded: true };
    }

    // Clear stale tenant from session when exchange fails permanently
    // (e.g. tenant removed, membership revoked, or explicit forbidden).
    return {
      session: {
        ...session,
        activeTenantId: undefined,
        tenantAccessToken: undefined,
        tenantExpiresAt: undefined,
      },
      updated: true,
    };
  }
}

export async function getAccessToken(request: Request): Promise<string | null> {
  const session = await getSession(request);
  if (!session) return null;

  const identityResult = await ensureIdentitySession(session);
  if (!identityResult.session) return null;
  const tenantResult = await ensureTenantSession(identityResult.session);
  const finalSession = tenantResult.session || identityResult.session;

  if (finalSession.activeTenantId && finalSession.tenantAccessToken) {
    return finalSession.tenantAccessToken;
  }
  return finalSession.identityAccessToken || null;
}

export async function requireAuth(request: Request) {
  const { session } = await requireAuthWithUpdate(request);
  return session;
}

export interface SessionUpdateResult {
  session: SessionData;
  headers?: HeadersInit;
}

export async function getAccessTokenWithUpdate(
  request: Request
): Promise<{ token: string | null; headers?: HeadersInit }> {
  const session = await getSession(request);
  if (!session) return { token: null };

  const identityResult = await ensureIdentitySession(session);
  if (!identityResult.session) return { token: null };

  const tenantResult = await ensureTenantSession(identityResult.session);
  const finalSession = tenantResult.session || identityResult.session;
  const token = finalSession.activeTenantId && finalSession.tenantAccessToken
    ? finalSession.tenantAccessToken
    : finalSession.identityAccessToken || null;

  if (identityResult.updated || tenantResult.updated) {
    return {
      token,
      headers: { "Set-Cookie": await commitSession(finalSession) },
    };
  }

  return { token };
}

export async function requireIdentityAuthWithUpdate(
  request: Request
): Promise<SessionUpdateResult> {
  const session = await getSession(request);
  if (!session || !(session.identityAccessToken || session.accessToken)) {
    throw redirect("/login", { headers: NO_STORE_HEADERS });
  }

  const identityResult = await ensureIdentitySession(session);
  if (!identityResult.session) {
    throw redirect("/login", { headers: NO_STORE_HEADERS });
  }

  if (identityResult.updated) {
    return {
      session: identityResult.session,
      headers: { "Set-Cookie": await commitSession(identityResult.session) },
    };
  }

  return { session: identityResult.session };
}

export async function requireTenantAuthWithUpdate(
  request: Request
): Promise<SessionUpdateResult> {
  const identity = await requireIdentityAuthWithUpdate(request);
  const session = identity.session;
  if (!session.activeTenantId) {
    throw redirect("/tenant/select", { headers: NO_STORE_HEADERS });
  }

  const tenantResult = await ensureTenantSession(session);
  if (!tenantResult.session) {
    throw redirect("/tenant/select?error=tenant_exchange_failed", {
      headers: NO_STORE_HEADERS,
    });
  }
  if (!tenantResult.session.tenantAccessToken && !tenantResult.degraded) {
    throw redirect("/tenant/select?error=tenant_exchange_failed", {
      headers: NO_STORE_HEADERS,
    });
  }

  if (tenantResult.updated) {
    return {
      session: tenantResult.session,
      headers: { "Set-Cookie": await commitSession(tenantResult.session) },
    };
  }

  return identity;
}

export async function requireAuthWithUpdate(
  request: Request
): Promise<SessionUpdateResult> {
  return requireIdentityAuthWithUpdate(request);
}

export async function setActiveTenant(
  request: Request,
  tenantId: string
): Promise<string> {
  const { session } = await requireIdentityAuthWithUpdate(request);
  const exchanged = await exchangeTenantToken(session, tenantId);
  if (!exchanged) {
    throw redirect("/tenant/select?error=tenant_exchange_failed", {
      headers: NO_STORE_HEADERS,
    });
  }
  return commitSession(exchanged);
}

export async function trySetActiveTenant(
  request: Request,
  tenantId: string
): Promise<{ cookie: string } | { error: string }> {
  const { session } = await requireIdentityAuthWithUpdate(request);
  const exchanged = await exchangeTenantToken(session, tenantId);
  if (!exchanged) {
    return { error: "tenant_exchange_failed" };
  }
  return { cookie: await commitSession(exchanged) };
}
