import { redirect, type LoaderFunctionArgs } from "react-router";

export async function loader({ request }: LoaderFunctionArgs) {
  const AUTH9_CORE_URL = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const PORTAL_URL = process.env.AUTH9_PORTAL_URL || "http://localhost:3000";

  // Redirect to backend logout, which will redirect to Keycloak logout,
  // then back to the portal home page
  const logoutUrl = `${AUTH9_CORE_URL}/api/v1/auth/logout?post_logout_redirect_uri=${encodeURIComponent(PORTAL_URL)}`;

  return redirect(logoutUrl);
}

export default function Logout() {
  // This component never renders - the loader always redirects
  return null;
}
