import type { LoaderFunctionArgs } from "react-router";
import { redirect, Outlet } from "react-router";
import { ThemeToggle } from "~/components/ThemeToggle";
import { requireAuthWithUpdate } from "~/services/session.server";
import { userApi } from "~/services/api";

export async function loader({ request }: LoaderFunctionArgs) {
  const { session } = await requireAuthWithUpdate(request);

  // If user already has active tenants, redirect to dashboard
  // (pending-only tenants should stay on the onboard flow)
  try {
    const res = await userApi.getMyTenants(session.accessToken);
    const hasActiveTenant = res.data?.some(
      (t) => t.tenant?.status === "active"
    );
    if (hasActiveTenant) {
      throw redirect("/dashboard");
    }
  } catch (e) {
    if (e instanceof Response) throw e;
    // API error, continue to show onboard page
  }

  // Get user email for domain suggestion
  let email = "";
  try {
    const userRes = await userApi.getMe(session.accessToken);
    email = userRes.data?.email || "";
  } catch {
    // fallback
  }

  return { email };
}

export default function OnboardLayout() {
  return (
    <>
      <div className="fixed top-6 right-6 z-20">
        <ThemeToggle />
      </div>

      <div className="min-h-screen flex items-center justify-center px-6 relative">
        <div className="page-backdrop" />
        <Outlet />
      </div>
    </>
  );
}
