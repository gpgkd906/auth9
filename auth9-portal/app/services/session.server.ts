import { createCookie, redirect } from "react-router";

// Helper to determine if we are in production
const isProduction = process.env.NODE_ENV === "production";

// Create the session cookie
export const sessionCookie = createCookie("auth9_session", {
  secrets: [process.env.SESSION_SECRET || "default-secret-change-me"],
  path: "/",
  sameSite: "lax",
  httpOnly: true,
  secure: isProduction,
  maxAge: 60 * 60 * 24 * 7, // 7 days
});

export interface SessionData {
  accessToken: string;
  refreshToken?: string;
  idToken?: string;
  expiresAt?: number;
}

export async function getSession(request: Request): Promise<SessionData | null> {
  const cookieHeader = request.headers.get("Cookie");
  return (await sessionCookie.parse(cookieHeader)) || null;
}

export async function commitSession(session: SessionData) {
  return sessionCookie.serialize(session);
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export async function destroySession(session: SessionData) {
  return sessionCookie.serialize("", { maxAge: 0 });
}

// Check if token is expired or expiring soon (within 60 seconds)
function isTokenExpired(session: SessionData): boolean {
  if (!session.expiresAt) return true;
  return Date.now() > (session.expiresAt - 60000);
}

// In-memory refresh lock to prevent concurrent refresh requests (per Node.js instance)
const refreshLocks = new Map<string, Promise<SessionData | null>>();

async function refreshAccessToken(session: SessionData): Promise<SessionData | null> {
  if (!session.refreshToken) return null;

  const lockKey = session.refreshToken || session.accessToken;

  // Check if refresh already in progress for this token
  if (refreshLocks.has(lockKey)) {
    console.log("Refresh already in progress, awaiting...");
    return await refreshLocks.get(lockKey)!;
  }

  const refreshPromise = (async () => {
    try {
      const tokenUrl = `${process.env.AUTH9_CORE_URL || "http://localhost:8080"}/api/v1/auth/token`;

      // Refresh Token Exchange
      const response = await fetch(tokenUrl, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          grant_type: "refresh_token",
          client_id: process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal",
          refresh_token: session.refreshToken,
        }),
      });

      if (!response.ok) {
        console.error("Failed to refresh token:", response.status);
        return null;
      }

      const data = await response.json();

      // Return updated session
      return {
        accessToken: data.access_token,
        refreshToken: data.refresh_token || session.refreshToken, // Use new refresh token if provided, else keep old
        idToken: data.id_token || session.idToken,
        expiresAt: Date.now() + (data.expires_in * 1000),
      };
    } catch (err) {
      console.error("Refresh exception:", err);
      return null;
    } finally {
      refreshLocks.delete(lockKey);
    }
  })();

  refreshLocks.set(lockKey, refreshPromise);
  return await refreshPromise;
}

export async function getAccessToken(request: Request): Promise<string | null> {
  const session = await getSession(request);
  if (!session || !session.accessToken) return null;

  // Check if token is expired and refresh if needed
  if (isTokenExpired(session)) {
    const newSession = await refreshAccessToken(session);
    if (newSession) {
      // Return the new token so the current request succeeds
      // Note: This does NOT update the cookie in the browser. 
      // The cookie will remain stale until a write occurs.
      // Ideally, callers should handle session updates.
      return newSession.accessToken;
    } else {
      // Refresh failed, token is invalid
      return null;
    }
  }

  return session.accessToken;
}

/**
 * @deprecated Use requireAuthWithUpdate to properly handle token refresh.
 * This function refreshes tokens but does not update the cookie in the browser.
 */
export async function requireAuth(request: Request) {
  console.warn("requireAuth is deprecated, use requireAuthWithUpdate instead");
  const { session } = await requireAuthWithUpdate(request);
  return session;
}

export interface SessionUpdateResult {
  session: SessionData;
  headers?: HeadersInit;
}

/**
 * Get access token and return Set-Cookie header if refreshed.
 * Callers must apply returned headers to response.
 */
export async function getAccessTokenWithUpdate(
  request: Request
): Promise<{ token: string | null; headers?: HeadersInit }> {
  const session = await getSession(request);
  if (!session || !session.accessToken) {
    return { token: null };
  }

  if (isTokenExpired(session)) {
    const newSession = await refreshAccessToken(session);
    if (newSession) {
      return {
        token: newSession.accessToken,
        headers: {
          "Set-Cookie": await commitSession(newSession),
        },
      };
    } else {
      return { token: null };
    }
  }

  return { token: session.accessToken };
}

/**
 * Require authentication and return Set-Cookie header if refreshed.
 * Throws redirect to /login if session invalid.
 */
export async function requireAuthWithUpdate(
  request: Request
): Promise<SessionUpdateResult> {
  const session = await getSession(request);
  if (!session || !session.accessToken) {
    throw redirect("/login");
  }

  if (isTokenExpired(session)) {
    const newSession = await refreshAccessToken(session);
    if (!newSession) {
      throw redirect("/login");
    }
    return {
      session: newSession,
      headers: {
        "Set-Cookie": await commitSession(newSession),
      },
    };
  }

  return { session };
}
