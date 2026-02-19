import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { redirect, useFetcher, useLoaderData, useNavigation } from "react-router";
import { useMemo, useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
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
  const [searchQuery, setSearchQuery] = useState("");
  const fetcher = useFetcher();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting" || fetcher.state !== "idle";

  const filteredTenants = useMemo(() => {
    if (!searchQuery.trim()) return tenants;
    const query = searchQuery.toLowerCase();
    return tenants.filter(
      (t) =>
        t.tenant.name.toLowerCase().includes(query) ||
        t.tenant.slug.toLowerCase().includes(query)
    );
  }, [tenants, searchQuery]);

  const handleTenantClick = (tenantId: string) => {
    if (tenantId === activeTenantId) return;
    fetcher.submit(
      { tenantId },
      { method: "post" }
    );
  };

  const showScroll = tenants.length > 20 || filteredTenants.length > 20;

  return (
    <div className="min-h-screen flex items-center justify-center px-6 relative">
      <div className="page-backdrop" />
      <Card className="w-full max-w-lg relative z-10 animate-fade-in-up">
        <CardHeader>
          <CardTitle>Select your tenant</CardTitle>
          <CardDescription>Choose which organization context to enter.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Input
            type="text"
            placeholder="Search tenants..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full"
            autoFocus
          />
          <div
            className={`space-y-2 ${showScroll ? "max-h-[400px] overflow-y-auto pr-1" : ""}`}
          >
            {filteredTenants.length === 0 ? (
              <p className="text-sm text-[var(--text-tertiary)] text-center py-4">
                No tenants found
              </p>
            ) : (
              filteredTenants.map((tenant) => (
                <button
                  key={tenant.tenant_id}
                  type="button"
                  onClick={() => handleTenantClick(tenant.tenant_id)}
                  disabled={isSubmitting}
                  className={`w-full flex items-center gap-3 rounded-lg px-3 py-2.5 cursor-pointer transition-colors text-left
                    ${
                      tenant.tenant_id === activeTenantId
                        ? "bg-[var(--accent-blue)] text-white"
                        : "border border-[var(--glass-border-subtle)] hover:bg-[var(--surface-secondary)]"
                    }
                    ${isSubmitting ? "opacity-50 cursor-not-allowed" : ""}
                  `}
                >
                  <div
                    className={`w-8 h-8 rounded flex items-center justify-center text-sm font-bold flex-shrink-0
                      ${tenant.tenant_id === activeTenantId ? "bg-white/20 text-white" : "bg-[var(--accent-blue)] text-white"}
                    `}
                  >
                    {tenant.tenant.name.charAt(0).toUpperCase()}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate">
                      {tenant.tenant.name}
                      {tenant.tenant_id === activeTenantId && (
                        <span className="ml-2 text-xs opacity-75">(current)</span>
                      )}
                    </div>
                    <div
                      className={`text-xs truncate ${
                        tenant.tenant_id === activeTenantId ? "text-white/70" : "text-[var(--text-tertiary)]"
                      }`}
                    >
                      {tenant.tenant.slug}
                    </div>
                  </div>
                </button>
              ))
            )}
          </div>
          {filteredTenants.length > 0 && (
            <p className="text-xs text-[var(--text-tertiary)] text-center">
              {filteredTenants.length} of {tenants.length} tenants
              {searchQuery && ` (filtered from "${searchQuery}")`}
            </p>
          )}
          {(error || fetcher.data?.error) && (
            <p className="text-sm text-[var(--accent-red)]">
              {error === "tenant_exchange_failed"
                ? "Failed to access this tenant. Please try again or contact support."
                : String(error || fetcher.data?.error)}
            </p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
