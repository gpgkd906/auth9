import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, Link, useActionData, useFetcher, useLoaderData, useNavigation } from "react-router";
import { ArrowLeftIcon, EnvelopeClosedIcon, GlobeIcon, Link2Icon, PersonIcon } from "@radix-ui/react-icons";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Switch } from "~/components/ui/switch";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { redirect } from "react-router";
import { tenantApi, serviceApi, invitationApi, webhookApi, tenantServiceApi, tenantUserApi } from "~/services/api";
import { formatErrorMessage } from "~/lib/error-messages";
import { getAccessToken } from "~/services/session.server";
import { FormattedDate } from "~/components/ui/formatted-date";
import { useI18n, useLocale } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.detail.metaTitle", undefined, {
    tenantName: data?.tenant.name || translate(resolveMetaLocale(matches), "tenants.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) throw new Error(translate(locale, "tenants.errors.tenantIdRequired"));
  const accessToken = await getAccessToken(request);

  let tenantRes;
  try {
    tenantRes = await tenantApi.get(tenantId, accessToken || undefined);
  } catch {
    throw redirect("/dashboard/tenants");
  }

  const [servicesRes, invitationsRes, webhooksRes, tenantServicesRes, tenantUsersRes] = await Promise.all([
    serviceApi.list(tenantId, 1, 1, accessToken || undefined).catch(() => ({ pagination: { total: 0 } })),
    invitationApi.list(tenantId, 1, 1, "pending", accessToken || undefined).catch(() => ({ pagination: { total: 0 } })),
    webhookApi.list(tenantId, accessToken || undefined).catch(() => ({ data: [] })),
    tenantServiceApi.listServices(tenantId, accessToken || undefined).catch(() => ({ data: [] })),
    tenantUserApi.list(tenantId, accessToken || undefined).catch(() => ({ data: [] })),
  ]);

  const enabledServicesCount = tenantServicesRes.data.filter((s: { enabled: boolean }) => s.enabled).length;
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
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) return Response.json({ error: translate(locale, "tenants.errors.tenantIdRequired") }, { status: 400 });
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

    if (intent === "update_status") {
      const status = formData.get("status") as string;
      if (!["active", "inactive", "suspended", "pending"].includes(status)) {
        return Response.json({ error: translate(locale, "tenants.errors.invalidStatus") }, { status: 400 });
      }
      await tenantApi.update(tenantId, {
        status: status as "active" | "inactive" | "suspended" | "pending",
      }, accessToken || undefined);
      return { success: true, statusUpdated: true };
    }

    if (intent === "update_settings") {
      const requireMfa = formData.get("require_mfa") === "true";
      await tenantApi.update(tenantId, {
        settings: { require_mfa: requireMfa },
      }, accessToken || undefined);
      return { success: true, settingsUpdated: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "tenants.errors.unknown");
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "tenants.errors.invalidIntent") }, { status: 400 });
}

function getStatusLabel(status: string, t: ReturnType<typeof useI18n>["t"]) {
  switch (status) {
    case "active":
    case "inactive":
    case "suspended":
    case "pending":
      return t(`tenants.statuses.${status}`);
    default:
      return status;
  }
}

export default function TenantDetailPage() {
  const { t } = useI18n();
  const { locale } = useLocale();
  const { tenant, usersCount, servicesCount, pendingInvitationsCount, webhooksCount, enabledServicesCount, totalGlobalServicesCount } =
    useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const settingsFetcher = useFetcher();
  const statusFetcher = useFetcher();

  const isSubmitting = navigation.state === "submitting";
  const isSettingsUpdating = settingsFetcher.state !== "idle";
  const isStatusUpdating = statusFetcher.state !== "idle";

  const currentStatus = statusFetcher.formData
    ? (statusFetcher.formData.get("status") as string)
    : tenant.status;

  const requireMfa = settingsFetcher.formData
    ? settingsFetcher.formData.get("require_mfa") === "true"
    : (tenant.settings as Record<string, unknown>)?.require_mfa === true;

  const statusSuccess = statusFetcher.data && "success" in (statusFetcher.data as Record<string, unknown>);
  const statusError = statusFetcher.data && "error" in (statusFetcher.data as Record<string, unknown>)
    ? String((statusFetcher.data as Record<string, unknown>).error)
    : null;
  const settingsSuccess = settingsFetcher.data && "success" in (settingsFetcher.data as Record<string, unknown>);
  const settingsError = settingsFetcher.data && "error" in (settingsFetcher.data as Record<string, unknown>)
    ? String((settingsFetcher.data as Record<string, unknown>).error)
    : null;

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/dashboard/tenants" aria-label={t("tenants.actions.backToList")}>
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
            <p className="text-sm text-[var(--text-secondary)]">{t("tenants.detail.description")}</p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>{t("tenants.detail.configurationTitle")}</CardTitle>
              <CardDescription>{t("tenants.detail.configurationDescription")}</CardDescription>
            </CardHeader>
            <CardContent>
              <Form method="post" className="space-y-4">
                <input type="hidden" name="intent" value="update" />
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="name">{t("tenants.fields.name")}</Label>
                    <Input id="name" name="name" defaultValue={tenant.name} required />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="slug">{t("tenants.fields.slug")}</Label>
                    <Input id="slug" name="slug" defaultValue={tenant.slug} required />
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="logo_url">{t("tenants.fields.logoUrl")}</Label>
                  <Input
                    id="logo_url"
                    name="logo_url"
                    defaultValue={tenant.logo_url || ""}
                    placeholder={t("tenants.placeholders.logoUrl")}
                  />
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label>{t("tenants.fields.status")}</Label>
                    <Select
                      value={currentStatus}
                      disabled={isStatusUpdating}
                      onValueChange={(value: string) => {
                        statusFetcher.submit(
                          { intent: "update_status", status: value },
                          { method: "post" }
                        );
                      }}
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="active">{t("tenants.statuses.active")}</SelectItem>
                        <SelectItem value="inactive">{t("tenants.statuses.inactive")}</SelectItem>
                        <SelectItem value="suspended">{t("tenants.statuses.suspended")}</SelectItem>
                        <SelectItem value="pending">{t("tenants.statuses.pending")}</SelectItem>
                      </SelectContent>
                    </Select>
                    {statusSuccess && (
                      <p className="text-sm text-[var(--accent-green)]">{t("tenants.detail.statusUpdated")}</p>
                    )}
                    {statusError && (
                      <p className="text-sm text-[var(--accent-red)]">{formatErrorMessage(statusError, locale)}</p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label className="text-[var(--text-tertiary)]">{t("tenants.fields.created")}</Label>
                    <div className="text-sm">
                      <FormattedDate date={tenant.created_at} />
                    </div>
                  </div>
                </div>
                {actionData && "error" in actionData && (
                  <p className="text-sm text-[var(--accent-red)]">
                    {formatErrorMessage(String(actionData.error), locale)}
                  </p>
                )}
                {actionData && "success" in actionData && actionData.success && (
                  <p className="text-sm text-[var(--accent-green)]">{t("tenants.detail.tenantUpdated")}</p>
                )}
                <div className="flex justify-end pt-4">
                  <Button type="submit" disabled={isSubmitting}>
                    {isSubmitting ? t("tenants.actions.saving") : t("tenants.actions.save")}
                  </Button>
                </div>
              </Form>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("tenants.detail.securityTitle")}</CardTitle>
              <CardDescription>{t("tenants.detail.securityDescription")}</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between gap-4">
                <div className="space-y-0.5">
                  <Label>{t("tenants.detail.requireMfa")}</Label>
                  <p className="text-sm text-[var(--text-secondary)]">{t("tenants.detail.requireMfaDescription")}</p>
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
              {settingsSuccess && (
                <p className="text-sm text-[var(--accent-green)] mt-3">{t("tenants.detail.settingsUpdated")}</p>
              )}
              {settingsError && (
                <p className="text-sm text-[var(--accent-red)] mt-3">{formatErrorMessage(settingsError, locale)}</p>
              )}
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>{t("tenants.detail.quickLinksTitle")}</CardTitle>
              <CardDescription>{t("tenants.detail.quickLinksDescription")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2">
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/services`}>
                  <GlobeIcon className="mr-2 h-4 w-4" />
                  {t("tenants.actions.services")}
                  <span className="ml-auto text-xs text-[var(--text-tertiary)]">
                    {t("tenants.detail.enabledServices", {
                      enabled: enabledServicesCount,
                      total: totalGlobalServicesCount,
                    })}
                  </span>
                </Link>
              </Button>
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/invitations`}>
                  <EnvelopeClosedIcon className="mr-2 h-4 w-4" />
                  {t("tenants.actions.invitations")}
                  {pendingInvitationsCount > 0 && (
                    <span className="ml-auto text-xs bg-[var(--accent-yellow)]/20 text-[var(--accent-yellow)] px-2 py-0.5 rounded-full">
                      {t("tenants.detail.pendingInvitations", { count: pendingInvitationsCount })}
                    </span>
                  )}
                </Link>
              </Button>
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/webhooks`}>
                  <Link2Icon className="mr-2 h-4 w-4" />
                  {t("tenants.actions.webhooks")}
                  <span className="ml-auto text-xs text-[var(--text-tertiary)]">{webhooksCount}</span>
                </Link>
              </Button>
              <Button variant="outline" className="w-full justify-start" asChild>
                <Link to={`/dashboard/tenants/${tenant.id}/sso`}>
                  <Link2Icon className="mr-2 h-4 w-4" />
                  {t("tenants.actions.enterpriseSso")}
                </Link>
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("tenants.detail.overviewTitle")}</CardTitle>
              <CardDescription>{t("tenants.detail.overviewDescription")}</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <PersonIcon className="h-4 w-4" />
                    {t("tenants.detail.overviewStats.users")}
                  </div>
                  <span className="font-medium">{usersCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <GlobeIcon className="h-4 w-4" />
                    {t("tenants.detail.overviewStats.globalServices")}
                  </div>
                  <span className="font-medium">{enabledServicesCount}/{totalGlobalServicesCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <PersonIcon className="h-4 w-4" />
                    {t("tenants.detail.overviewStats.tenantServices")}
                  </div>
                  <span className="font-medium">{servicesCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <EnvelopeClosedIcon className="h-4 w-4" />
                    {t("tenants.detail.overviewStats.pendingInvitations")}
                  </div>
                  <span className="font-medium">{pendingInvitationsCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <Link2Icon className="h-4 w-4" />
                    {t("tenants.detail.overviewStats.webhooks")}
                  </div>
                  <span className="font-medium">{webhooksCount}</span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-sm text-[var(--text-secondary)]">
                    <GlobeIcon className="h-4 w-4" />
                    {t("tenants.fields.status")}
                  </div>
                  <span className="font-medium">{getStatusLabel(currentStatus, t)}</span>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
