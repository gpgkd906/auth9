import type { MetaFunction, LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, Link, useFetcher, useLoaderData } from "react-router";
import { ArrowLeftIcon, GlobeIcon } from "@radix-ui/react-icons";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Switch } from "~/components/ui/switch";
import { tenantApi, tenantServiceApi, type ServiceWithStatus } from "~/services/api";
import { formatErrorMessage } from "~/lib/error-messages";

export const meta: MetaFunction<typeof loader> = ({ data }) => {
  return [{ title: `Services - ${data?.tenant.name || "Tenant"} - Auth9` }];
};

export async function loader({ params }: LoaderFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) throw new Error("Tenant ID is required");

  const [tenantRes, servicesRes] = await Promise.all([
    tenantApi.get(tenantId),
    tenantServiceApi.listServices(tenantId),
  ]);

  return {
    tenant: tenantRes.data,
    services: servicesRes.data,
  };
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  if (!tenantId) return Response.json({ error: "Tenant ID required" }, { status: 400 });

  const formData = await request.formData();
  const serviceId = formData.get("serviceId") as string;
  const enabled = formData.get("enabled") === "true";

  try {
    const result = await tenantServiceApi.toggleService(tenantId, serviceId, enabled);
    return { success: true, services: result.data };
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }
}

function ServiceToggleRow({ tenantId, service }: { tenantId: string; service: ServiceWithStatus }) {
  const fetcher = useFetcher();
  const isUpdating = fetcher.state !== "idle";

  // Use optimistic UI - show the state we're transitioning to
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
          {service.base_url && (
            <p className="text-sm text-[var(--text-tertiary)]">{service.base_url}</p>
          )}
          <span className={`text-xs px-2 py-0.5 rounded-full ${
            service.status === "active"
              ? "bg-[var(--accent-green)]/20 text-[var(--accent-green)]"
              : "bg-[var(--text-tertiary)]/20 text-[var(--text-tertiary)]"
          }`}>
            {service.status}
          </span>
        </div>
      </div>
      <fetcher.Form method="post">
        <input type="hidden" name="serviceId" value={service.id} />
        <input type="hidden" name="enabled" value={(!optimisticEnabled).toString()} />
        <div className="flex items-center gap-3">
          <span className={`text-sm ${optimisticEnabled ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]"}`}>
            {optimisticEnabled ? "Enabled" : "Disabled"}
          </span>
          <Switch
            checked={optimisticEnabled}
            disabled={isUpdating}
            onCheckedChange={() => {
              // Let the form handle the submission
              const form = document.createElement("form");
              form.method = "post";
              form.style.display = "none";

              const serviceIdInput = document.createElement("input");
              serviceIdInput.name = "serviceId";
              serviceIdInput.value = service.id;
              form.appendChild(serviceIdInput);

              const enabledInput = document.createElement("input");
              enabledInput.name = "enabled";
              enabledInput.value = (!optimisticEnabled).toString();
              form.appendChild(enabledInput);

              // Submit via fetcher
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
  const { tenant, services } = useLoaderData<typeof loader>();

  const enabledCount = services.filter((s) => s.enabled).length;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenant.id}`}>
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
              Services for {tenant.name}
            </h1>
            <p className="text-sm text-[var(--text-secondary)]">
              Enable or disable global services for this tenant
            </p>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold text-[var(--text-primary)]">{services.length}</div>
            <div className="text-sm text-[var(--text-secondary)]">Total Services</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold text-[var(--accent-green)]">{enabledCount}</div>
            <div className="text-sm text-[var(--text-secondary)]">Enabled</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-6">
            <div className="text-2xl font-bold text-[var(--text-tertiary)]">{services.length - enabledCount}</div>
            <div className="text-sm text-[var(--text-secondary)]">Disabled</div>
          </CardContent>
        </Card>
      </div>

      {/* Services List */}
      <Card>
        <CardHeader>
          <CardTitle>Global Services</CardTitle>
          <CardDescription>
            Toggle services on or off for this tenant. Enabled services can be accessed by users in this tenant.
          </CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          {services.length === 0 ? (
            <div className="p-8 text-center text-[var(--text-tertiary)]">
              <GlobeIcon className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>No global services available</p>
              <p className="text-sm mt-1">Create services without a tenant_id to make them available here.</p>
            </div>
          ) : (
            <div className="divide-y divide-[var(--border-primary)]">
              {services.map((service) => (
                <ServiceToggleRow
                  key={service.id}
                  tenantId={tenant.id}
                  service={service}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
