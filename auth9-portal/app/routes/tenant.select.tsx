import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { requireIdentityAuthWithUpdate, setActiveTenant } from "~/services/session.server";
import { type TenantUserWithTenant, userApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Select Tenant - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const { session, headers } = await requireIdentityAuthWithUpdate(request);
  const identityToken = session.identityAccessToken || session.accessToken;
  if (!identityToken) throw redirect("/login");

  const serviceId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  const tenantsRes = await userApi.getMyTenants(identityToken, serviceId);
  const tenants = tenantsRes.data || [];

  if (tenants.length === 0) {
    throw redirect("/onboard", { headers: headers || undefined });
  }

  if (tenants.length === 1) {
    const tenantCookie = await setActiveTenant(request, tenants[0].tenant_id);
    const responseHeaders: [string, string][] = [["Set-Cookie", tenantCookie]];
    if (headers) {
      const refreshed = (headers as Record<string, string>)["Set-Cookie"];
      if (refreshed) responseHeaders.push(["Set-Cookie", refreshed]);
    }
    return redirect("/dashboard", { headers: responseHeaders });
  }

  return {
    tenants,
    activeTenantId: session.activeTenantId,
    error: new URL(request.url).searchParams.get("error"),
  };
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const tenantId = String(formData.get("tenantId") || "").trim();
  if (!tenantId) {
    return { error: "Please select a tenant" };
  }

  const cookie = await setActiveTenant(request, tenantId);
  return redirect("/dashboard", {
    headers: { "Set-Cookie": cookie },
  });
}

export default function TenantSelectPage() {
  const { tenants, activeTenantId, error } = useLoaderData<typeof loader>() as {
    tenants: TenantUserWithTenant[];
    activeTenantId?: string;
    error?: string | null;
  };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="min-h-screen flex items-center justify-center px-6 relative">
      <div className="page-backdrop" />
      <Card className="w-full max-w-lg relative z-10 animate-fade-in-up">
        <CardHeader>
          <CardTitle>Select your tenant</CardTitle>
          <CardDescription>Choose which organization context to enter.</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-3">
            {tenants.map((tenant) => (
              <label
                key={tenant.tenant_id}
                className="flex items-center gap-3 border border-[var(--glass-border-subtle)] rounded-lg px-3 py-2 cursor-pointer"
              >
                <input
                  type="radio"
                  name="tenantId"
                  value={tenant.tenant_id}
                  defaultChecked={tenant.tenant_id === activeTenantId}
                  required
                />
                <div>
                  <div className="font-medium">{tenant.tenant.name}</div>
                  <div className="text-xs text-[var(--text-tertiary)]">{tenant.tenant.slug}</div>
                </div>
              </label>
            ))}
            {(error || actionData?.error) && (
              <p className="text-sm text-[var(--accent-red)]">
                {error === "tenant_exchange_failed"
                  ? "Failed to access this tenant. Please try again or contact support."
                  : String(error || actionData?.error)}
              </p>
            )}
            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? "Switching..." : "Continue"}
            </Button>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
