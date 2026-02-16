import { redirect } from "react-router";
import type { LoaderFunctionArgs } from "react-router";
import { getAccessToken, getSession, destroySession } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  const AUTH9_CORE_URL = process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const AUTH9_CORE_PUBLIC_URL = process.env.AUTH9_CORE_PUBLIC_URL || AUTH9_CORE_URL;
  const PORTAL_URL = process.env.AUTH9_PORTAL_URL || "http://localhost:3000";
  const CLIENT_ID = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";

  // Get the access token to send with logout request
  const accessToken = await getAccessToken(request);

  // Call backend logout API with token to revoke session in database
  // Use internal URL for server-to-server communication
  if (accessToken) {
    try {
      const response = await fetch(`${AUTH9_CORE_URL}/api/v1/auth/logout?post_logout_redirect_uri=${encodeURIComponent(PORTAL_URL)}&client_id=${encodeURIComponent(CLIENT_ID)}`, {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${accessToken}`,
        },
        redirect: "manual", // Don't follow redirects, we'll handle it ourselves
      });
      // Log for debugging - session revocation happens on this call
      if (!response.ok && response.status !== 302) {
        console.error("[logout] Backend logout API returned non-redirect status:", response.status);
      }
    } catch (error) {
      // Log but proceed with logout anyway
      console.error("[logout] Backend logout API error:", error);
    }
  }

  // Destroy the portal session cookie so the user can't access dashboard after logout
  const session = await getSession(request);
  const headers = new Headers();
  headers.set("Cache-Control", "no-store, no-cache, must-revalidate, private");
  headers.set("Pragma", "no-cache");
  headers.set("Expires", "0");
  if (session) {
    headers.append("Set-Cookie", await destroySession(session));
  }

  // Redirect to backend logout (public URL for browser redirect)
  // This will redirect to Keycloak logout, then back to portal
  const logoutUrl = new URL(`${AUTH9_CORE_PUBLIC_URL}/api/v1/auth/logout`);
  logoutUrl.searchParams.set("post_logout_redirect_uri", PORTAL_URL);
  logoutUrl.searchParams.set("client_id", CLIENT_ID);

  // Pass id_token_hint so Keycloak skips the logout confirmation page
  if (session?.idToken) {
    logoutUrl.searchParams.set("id_token_hint", session.idToken);
  }

  return redirect(logoutUrl.toString(), { headers });
}

export default function Logout() {
  // This component never renders - the loader always redirects
  return null;
}
