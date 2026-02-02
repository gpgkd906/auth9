import { redirect } from "react-router";
import type { LoaderFunctionArgs } from "react-router";
import { getAccessToken } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  const AUTH9_CORE_URL = process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const AUTH9_CORE_PUBLIC_URL = process.env.AUTH9_CORE_PUBLIC_URL || AUTH9_CORE_URL;
  const PORTAL_URL = process.env.AUTH9_PORTAL_URL || "http://localhost:3000";

  // Get the access token to send with logout request
  const accessToken = await getAccessToken(request);

  // Call backend logout API with token to revoke session in database
  // Use internal URL for server-to-server communication
  if (accessToken) {
    try {
      await fetch(`${AUTH9_CORE_URL}/api/v1/auth/logout?post_logout_redirect_uri=${encodeURIComponent(PORTAL_URL)}`, {
        method: "GET",
        headers: {
          "Authorization": `Bearer ${accessToken}`,
        },
        redirect: "manual", // Don't follow redirects, we'll handle it ourselves
      });
    } catch {
      // Ignore errors - proceed with logout anyway
    }
  }

  // Redirect to backend logout (public URL for browser redirect)
  // This will redirect to Keycloak logout, then back to portal
  const logoutUrl = `${AUTH9_CORE_PUBLIC_URL}/api/v1/auth/logout?post_logout_redirect_uri=${encodeURIComponent(PORTAL_URL)}`;

  return redirect(logoutUrl);
}

export default function Logout() {
  // This component never renders - the loader always redirects
  return null;
}
