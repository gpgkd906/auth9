import { redirect } from "react-router";
import type { LoaderFunctionArgs } from "react-router";
import { getSession, destroySession } from "~/services/session.server";
import { hostedLoginApi } from "~/services/api";

export async function loader({ request }: LoaderFunctionArgs) {
  // Get the identity token (not tenant token) to send with logout request
  // The logout endpoint validates against aud:"auth9" which is the identity token audience
  const session = await getSession(request);
  const identityToken = session?.identityAccessToken || session?.accessToken;

  // Call hosted-login logout API to revoke session in database + identity engine
  // Returns JSON (no Keycloak redirect), so we handle the redirect ourselves
  if (identityToken) {
    try {
      await hostedLoginApi.logout(identityToken);
    } catch (error) {
      // Log but proceed with logout anyway
      console.error("[logout] Hosted login logout API error:", error);
    }
  }

  // Destroy the portal session cookie so the user can't access dashboard after logout
  const headers = new Headers();
  headers.set("Cache-Control", "no-store, no-cache, must-revalidate, private");
  headers.set("Pragma", "no-cache");
  headers.set("Expires", "0");
  if (session) {
    headers.append("Set-Cookie", await destroySession(session));
  }

  // Redirect directly to login page (no Keycloak redirect needed)
  return redirect("/login", { headers });
}

export default function Logout() {
  // This component never renders - the loader always redirects
  return null;
}
