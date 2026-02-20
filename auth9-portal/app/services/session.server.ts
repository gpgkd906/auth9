import { createCookie, redirect } from "react-router";
import { authApi } from "~/services/api";

const isProduction = process.env.NODE_ENV === "production";
const SESSION_MAX_AGE = 8 * 60 * 60;

export const NO_STORE_HEADERS: HeadersInit = {
  "Cache-Control": "no-store, no-cache, must-revalidate, private",
  Pragma: "no-cache",
  Expires: "0",
};

export const sessionCookie = createCookie("auth9_session", {
  secrets: [process.env.SESSION_SECRET || "default-secret-change-me"],
  path: "/",
  sameSite: "lax",
  httpOnly: true,
  secure: isProduction,
  maxAge: SESSION_MAX_AGE,
});

export interface SessionData {
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
  secrets: [process.env.SESSION_SECRET || "default-secret-change-me"],
  path: "/",
  sameSite: "lax",
  httpOnly: true,
  secure: isProduction,
  maxAge: 5 * 60,
});

export async function serializeOAuthState(state: string): Promise<string> {
  return oauthStateCookie.serialize(state);
}

export async function getOAuthState(request: Request): Promise<string | null> {
  const cookieHeader = request.headers.get("Cookie");
  const value = await oauthStateCookie.parse(cookieHeader);
  return typeof value === "string" && value.length > 0 ? value : null;
}

export async function clearOAuthStateCookie(): Promise<string> {
  return oauthStateCookie.serialize("", { maxAge: 0 });
}

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

export async function getSession(request: Request): Promise<SessionData | null> {
  const cookieHeader = request.headers.get("Cookie");
  const raw = (await sessionCookie.parse(cookieHeader)) || null;
  return normalizeSession(raw);
}

export async function commitSession(session: SessionData) {
  const normalized = normalizeSession(session) || session;
  // Strip redundant and re-derivable fields to keep cookie under browser 4096-byte limit.
  // - accessToken / expiresAt are aliases for identityAccessToken / identityExpiresAt
  //   (normalizeSession reconstructs them on read).
  // - tenantAccessToken / tenantExpiresAt can be re-exchanged on the server from
  //   identityAccessToken + activeTenantId via ensureTenantSession().
  const compact = { ...normalized };
  delete compact.accessToken;
  delete compact.expiresAt;
  delete compact.tenantAccessToken;
  delete compact.tenantExpiresAt;
  return sessionCookie.serialize(compact);
}

export async function destroySession(session: SessionData) {
  void session;
  return sessionCookie.serialize("", { maxAge: 0 });
}

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
    console.error("[exchangeTenantToken] Failed for tenant", tenantId, ":",
      error instanceof Error ? error.message : error);
    return null;
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
): Promise<{ session: SessionData | null; updated: boolean }> {
  if (!session.activeTenantId) {
    return { session, updated: false };
  }
  if (session.tenantAccessToken && !isTokenExpired(session.tenantExpiresAt)) {
    return { session, updated: false };
  }
  const exchanged = await exchangeTenantToken(session, session.activeTenantId);
  return { session: exchanged, updated: !!exchanged };
}

export async function getAccessToken(request: Request): Promise<string | null> {
  const session = await getSession(request);
  if (!session) return null;

  const identityResult = await ensureIdentitySession(session);
  if (!identityResult.session) return null;
  const tenantResult = await ensureTenantSession(identityResult.session);
  const finalSession = tenantResult.session || identityResult.session;

  if (finalSession.activeTenantId) {
    return finalSession.tenantAccessToken || null;
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
  const token = finalSession.activeTenantId
    ? finalSession.tenantAccessToken || null
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
  if (!tenantResult.session || !tenantResult.session.tenantAccessToken) {
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
