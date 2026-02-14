import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, Link, useActionData, useFetcher, useLoaderData, useNavigation } from "react-router";
import { ArrowLeftIcon, EnvelopeClosedIcon, GlobeIcon, Link2Icon, PersonIcon } from "@radix-ui/react-icons";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Switch } from "~/components/ui/switch";
import { redirect } from "react-router";
import { tenantApi, serviceApi, invitationApi, webhookApi, tenantServiceApi, tenantUserApi } from "~/services/api";
import { formatErrorMessage } from "~/lib/error-messages";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction<typeof loader> = ({ data }) => {
  return [{ title: `${data?.tenant.name || "Tenant"} - Auth9` }];
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) throw new Error("Tenant ID is required");
  const accessToken = await getAccessToken(request);

  try {
    // Fetch tenant details and related counts in parallel
    const [tenantRes, servicesRes, invitationsRes, webhooksRes, tenantServicesRes, tenantUsersRes] = await Promise.all([
      tenantApi.get(tenantId, accessToken || undefined),
      serviceApi.list(tenantId, 1, 1, accessToken || undefined), // Just get count
      invitationApi.list(tenantId, 1, 1, "pending", accessToken || undefined), // Pending invitations count
      webhookApi.list(tenantId, accessToken || undefined),
      tenantServiceApi.listServices(tenantId, accessToken || undefined), // Get global services with enabled status
      tenantUserApi.list(tenantId, accessToken || undefined),
    ]);

    const enabledServicesCount = tenantServicesRes.data.filter(s => s.enabled).length;
    const totalGlobalServicesCount = tenantServicesRes.data.length;

    return {
      tenant: tenantRes.data,
      usersCount: tenantUsersRes.data.length,
      servicesCount: servicesRes.pagination.total,
      pendingInvitationsCount: invitationsRes.pagination.total,
      webhooksCount: webhooksRes.data.length,
      enabledServicesCount,
      totalGlobalServicesCount,
    };
  } catch {
    throw redirect("/dashboard/tenants");
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) return Response.json({ error: "Tenant ID required" }, { status: 400 });
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "update") {
      const name = formData.get("name") as string;
      const slug = formData.get("slug") as string;
      const logo_url = formData.get("logo_url") as string;

      await tenantApi.update(tenantId, {
        name,
        slug,
        logo_url: logo_url || undefined,
      }, accessToken || undefined);
      return { success: true };
    }

    if (intent === "update_settings") {
      const requireMfa = formData.get("require_mfa") === "true";
      await tenantApi.update(tenantId, {
        settings: { require_mfa: requireMfa },
      }, accessToken || undefined);
      return { success: true, settingsUpdated: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

export default function TenantDetailPage() {
  const { tenant, usersCount, servicesCount, pendingInvitationsCount, webhooksCount, enabledServicesCount, totalGlobalServicesCount } =
    useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const settingsFetcher = useFetcher();

  const isSubmitting = navigation.state === "submitting";
  const isSettingsUpdating = settingsFetcher.state !== "idle";

  // Optimistic MFA state
  const requireMfa = settingsFetcher.formData
    ? settingsFetcher.formData.get("require_mfa") === "true"
    : (tenant.settings as Record<string, unknown>)?.require_mfa === true;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/dashboard/tenants">
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div className="flex items-center gap-3">
          {tenant.logo_url && (
            <img
              src={tenant.logo_url}
              alt=""
              className="h-10 w-10 rounded object-cover"
            />
          )}
          <div>
            <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">
              {tenant.name}
            </h1>
            <p className="text-sm text-[var(--text-secondary)]">
              Tenant Configuration and Management
            </p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Tenant Configuration Form */}
        <div className="lg:col-span-2">
          <Card>
            <CardHeader>
              <CardTitle>Configuration</CardTitle>
              <CardDescription>General settings for this tenant</CardDescription>
            </CardHeader>
            <CardContent>
              <Form method="post" className="space-y-4">
                <input type="hidden" name="intent" value="update" />
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="name">Name</Label>
                    <Input
                      id="name"
                      name="name"
                      defaultValue={tenant.name}
                      required
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="slug">Slug</Label>
                    <Input
                      id="slug"
                      name="slug"
                      defaultValue={tenant.slug}
                      required
                    />
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="logo_url">Logo URL</Label>
                  <Input
                    id="logo_url"
                    name="logo_url"
                    defaultValue={tenant.logo_url || ""}
                    placeholder="https://..."
                  />
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label className="text-[var(--text-tertiary)]">Status</Label>
                    <div className="text-sm capitalize">{tenant.status}</div>
                  </div>
                  <div className="space-y-2">
                    <Label className="text-[var(--text-tertiary)]">Created</Label>
                    <div className="text-sm">
                      {new Date(tenant.created_at).toLocaleString()}
                    </div>
                  </div>
                </div>
                {actionData && "error" in actionData && (
                  <p className="text-sm text-[var(--accent-red)]">
                    {formatErrorMessage(String(actionData.error))}
                  </p>
                )}
                {actionData && "success" in actionData && actionData.success && (
                  <p className="text-sm text-[var(--accent-green)]">
                    Tenant updated successfully
                  </p>
                )}
                <div className="flex justify-end pt-4">
                  <Button type="submit" disabled={isSubmitting}>
                    {isSubmitting ? "Saving..." : "Save Changes"}
                  </Button>
                </div>
              </Form>
            </CardContent>
          </Card>

          {/* Security Settings */}
          <Card>
            <CardHeader>
              <CardTitle>Security Settings</CardTitle>
              <CardDescription>Authentication and security policies for this tenant</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label>Require MFA</Label>
                  <p className="text-sm text-[var(--text-secondary)]">
                    Require all users in this tenant to enable multi-factor authentication
                  </p>
                </div>
                <Switch
                  checked={requireMfa}
                  disabled={isSettingsUpdating}
                  onCheckedChange={(checked: boolean) => {
                    settingsFetcher.submit(
                      { intent: "update_settings", require_mfa: checked.toString() },
                      { method: "post" }
                    );
                  }}
                />
              </div>
              {settingsFetcher.data && "success" in (settingsFetcher.data as Record<string, unknown>) && (
                <p className="text-sm text-[var(--accent-green)] mt-3">
                  Settings saved successfully
                </p>
              )}
              {settingsFetcher.data && "error" in (settingsFetcher.data as Record<string, unknown>) && (
                <p className="text-sm text-[var(--accent-red)] mt-3">
                  {String((settingsFetcher.data as Record<string, unknown>).error)}
                </p>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Quick Links & Stats */}
        <div className="space-y-6">
          {/* Quick Links */}
          <Card>
            <CardHeader>
              <CardTitle>Quick Links</CardTitle>
              <CardDescription>Manage tenant resources</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/services`}>
                  <GlobeIcon className="mr-2 h-4 w-4" />
                  Services
                  <span className="ml-auto text-xs text-[var(--text-tertiary)]">
                    {enabledServicesCount}/{totalGlobalServicesCount} enabled
                  </span>
                </Link>
              </Button>
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/invitations`}>
                  <EnvelopeClosedIcon className="mr-2 h-4 w-4" />
                  Invitations
                  {pendingInvitationsCount > 0 && (
                    <span className="ml-auto text-xs bg-[var(--accent-yellow)]/20 text-[var(--accent-yellow)] px-2 py-0.5 rounded-full">
                      {pendingInvitationsCount} pending
                    </span>
                  )}
                </Link>
              </Button>
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/webhooks`}>
                  <Link2Icon className="mr-2 h-4 w-4" />
                  Webhooks
                  <span className="ml-auto text-xs text-[var(--text-tertiary)]">
                    {webhooksCount}
                  </span>
                </Link>
              </Button>
            </CardContent>
          </Card>

          {/* Stats */}
          <Card>
            <CardHeader>
              <CardTitle>Overview</CardTitle>
              <CardDescription>Tenant statistics</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <PersonIcon className="h-4 w-4" />
                    Users
                  </div>
                  <span className="font-medium">{usersCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <GlobeIcon className="h-4 w-4" />
                    Global Services
                  </div>
                  <span className="font-medium">{enabledServicesCount}/{totalGlobalServicesCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <PersonIcon className="h-4 w-4" />
                    Tenant Services
                  </div>
                  <span className="font-medium">{servicesCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <EnvelopeClosedIcon className="h-4 w-4" />
                    Pending Invitations
                  </div>
                  <span className="font-medium">{pendingInvitationsCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <Link2Icon className="h-4 w-4" />
                    Webhooks
                  </div>
                  <span className="font-medium">{webhooksCount}</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
