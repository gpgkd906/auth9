import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Link, useFetcher, useLoaderData } from "react-router";
import { ArrowLeftIcon, GlobeIcon } from "@radix-ui/react-icons";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Switch } from "~/components/ui/switch";
import { redirect } from "react-router";
import { tenantApi, tenantServiceApi, type ServiceWithStatus } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.services.metaTitle", undefined, {
    tenantName: data?.tenant.name || translate(resolveMetaLocale(matches), "tenants.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) throw new Error(translate(locale, "tenants.errors.tenantIdRequired"));
  const accessToken = await getAccessToken(request);

  try {
    const [tenantRes, servicesRes] = await Promise.all([
      tenantApi.get(tenantId, accessToken || undefined),
      tenantServiceApi.listServices(tenantId, accessToken || undefined),
    ]);

    return {
      tenant: tenantRes.data,
      services: servicesRes.data,
    };
  } catch {
    throw redirect("/dashboard/tenants");
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) return Response.json({ error: translate(locale, "tenants.errors.tenantIdRequired") }, { status: 400 });
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const serviceId = formData.get("serviceId") as string;
  const enabled = formData.get("enabled") === "true";

  try {
    const result = await tenantServiceApi.toggleService(tenantId, serviceId, enabled, accessToken || undefined);
    return { success: true, services: result.data };
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "tenants.errors.unknown");
    return Response.json({ error: message }, { status: 400 });
  }
}

function getServiceStatusLabel(status: string, t: ReturnType<typeof useI18n>["t"]) {
  switch (status) {
    case "active":
      return t("tenants.statuses.active");
    case "inactive":
      return t("tenants.statuses.inactive");
    case "suspended":
      return t("tenants.statuses.suspended");
    case "pending":
      return t("tenants.statuses.pending");
    default:
      return status;
  }
}

function ServiceToggleRow({ service }: { service: ServiceWithStatus }) {
  const { t } = useI18n();
  const fetcher = useFetcher();
  const isUpdating = fetcher.state !== "idle";

  const optimisticEnabled = fetcher.formData
    ? fetcher.formData.get("enabled") === "true"
    : service.enabled;

  return (
    <div className="flex items-center justify-between p-4 border-b border-[var(--border-primary)] last:border-b-0">
      <div className="flex items-center gap-3">
        <div className="p-2 rounded-lg bg-[var(--bg-secondary)]">
          <GlobeIcon className="h-5 w-5 text-[var(--text-secondary)]" />
        </div>
        <div>
          <h3 className="font-medium text-[var(--text-primary)]">{service.name}</h3>
          {service.base_url && <p className="text-sm text-[var(--text-tertiary)]">{service.base_url}</p>}
          <span className={`text-xs px-2 py-0.5 rounded-full ${
            service.status === "active"
              ? "bg-[var(--accent-green)]/20 text-[var(--accent-green)]"
              : "bg-[var(--text-tertiary)]/20 text-[var(--text-tertiary)]"
          }`}>
            {getServiceStatusLabel(service.status, t)}
          </span>
        </div>
      </div>
      <fetcher.Form method="post">
        <input type="hidden" name="serviceId" value={service.id} />
        <input type="hidden" name="enabled" value={(!optimisticEnabled).toString()} />
        <div className="flex items-center gap-3">
          <span className={`text-sm ${optimisticEnabled ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]"}`}>
            {optimisticEnabled ? t("tenants.services.enabledState") : t("tenants.services.disabledState")}
          </span>
          <Switch
            checked={optimisticEnabled}
            disabled={isUpdating}
            onCheckedChange={() => {
              fetcher.submit(
                { serviceId: service.id, enabled: (!optimisticEnabled).toString() },
                { method: "post" }
              );
            }}
          />
        </div>
      </fetcher.Form>
    </div>
  );
}

export default function TenantServicesPage() {
  const { t } = useI18n();
  const { tenant, services } = useLoaderData<typeof loader>();

  const enabledCount = services.filter((s) => s.enabled).length;

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenant.id}`} aria-label={t("tenants.actions.backToList")}>
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
              {t("tenants.services.title", { tenantName: tenant.name })}
            </h1>
            <p className="text-sm text-[var(--text-secondary)]">{t("tenants.services.description")}</p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold text-[var(--text-primary)]">{services.length}</div>
            <div className="text-sm text-[var(--text-secondary)]">{t("tenants.services.totalServices")}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold text-[var(--accent-green)]">{enabledCount}</div>
            <div className="text-sm text-[var(--text-secondary)]">{t("tenants.services.enabled")}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold text-[var(--text-tertiary)]">{services.length - enabledCount}</div>
            <div className="text-sm text-[var(--text-secondary)]">{t("tenants.services.disabled")}</div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.services.globalServices")}</CardTitle>
          <CardDescription>{t("tenants.services.globalServicesDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          {services.length === 0 ? (
            <div className="p-8 text-center text-[var(--text-tertiary)]">
              <GlobeIcon className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>{t("tenants.services.noServices")}</p>
              <p className="text-sm mt-1">{t("tenants.services.noServicesDescription")}</p>
            </div>
          ) : (
            <div className="divide-y divide-[var(--border-primary)]">
              {services.map((service) => (
                <ServiceToggleRow key={service.id} service={service} />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
