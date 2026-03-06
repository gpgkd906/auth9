import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import {
  ArrowLeftIcon,
  CheckCircledIcon,
  CopyIcon,
  EyeClosedIcon,
  EyeOpenIcon,
  LightningBoltIcon,
  PlusIcon,
  ResetIcon,
  TrashIcon,
  UpdateIcon,
} from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { useConfirm } from "~/hooks/useConfirm";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import { Badge } from "~/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "~/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { actionApi, serviceApi, serviceBrandingApi } from "~/services/api";
import type { Action, BrandingConfig, ServiceIntegrationInfo } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction = ({ data, matches }) => {
  const locale = resolveMetaLocale(matches);
  const routeData = data as { service?: { name?: string } } | undefined;
  return buildMeta(locale, "services.detail.metaTitle", undefined, {
    serviceName: routeData?.service?.name || translate(locale, "services.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { id } = params;
  const locale = await resolveLocale(request);
  if (!id) throw new Error(translate(locale, "services.errors.serviceIdRequired"));
  const accessToken = await getAccessToken(request);

  const [serviceRes, clientsRes, integrationRes, actionsRes, brandingRes] = await Promise.all([
    serviceApi.get(id, accessToken || undefined),
    serviceApi.listClients(id, accessToken || undefined),
    serviceApi.getIntegration(id, accessToken || undefined).catch(() => null),
    actionApi.list(id, undefined, accessToken || undefined).catch(() => ({ data: [] as Action[] })),
    serviceBrandingApi.get(id, accessToken || undefined).catch(() => null),
  ]);

  return {
    locale,
    service: serviceRes.data,
    clients: clientsRes.data,
    integration: integrationRes?.data ?? null,
    actions: actionsRes.data,
    branding: brandingRes?.data ?? null,
  };
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { id } = params;
  const locale = await resolveLocale(request);
  if (!id) {
    return Response.json({ error: translate(locale, "services.errors.serviceIdRequired") }, { status: 400 });
  }
  const accessToken = await getAccessToken(request);

  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "update_service") {
      const name = formData.get("name") as string;
      const baseUrl = formData.get("base_url") as string;
      const redirectUris = (formData.get("redirect_uris") as string)
        ?.split(",")
        .map((item) => item.trim())
        .filter(Boolean);
      const logoutUris = (formData.get("logout_uris") as string)
        ?.split(",")
        .map((item) => item.trim())
        .filter(Boolean);

      await serviceApi.update(
        id,
        {
          name,
          base_url: baseUrl || undefined,
          redirect_uris: redirectUris,
          logout_uris: logoutUris,
        },
        accessToken || undefined
      );
      return { success: true, intent };
    }

    if (intent === "create_client") {
      const name = formData.get("name") as string;
      const result = await serviceApi.createClient(id, { name: name || undefined }, accessToken || undefined);
      return { success: true, intent, secret: result.data.client_secret, clientId: result.data.client_id };
    }

    if (intent === "delete_client") {
      const clientId = formData.get("client_id") as string;
      await serviceApi.deleteClient(id, clientId, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "regenerate_secret") {
      const clientId = formData.get("client_id") as string;
      const result = await serviceApi.regenerateClientSecret(id, clientId, accessToken || undefined);
      return { success: true, intent, secret: result.data.client_secret, regeneratedClientId: clientId };
    }

    if (intent === "update_branding") {
      const config: BrandingConfig = {
        logo_url: (formData.get("logo_url") as string) || undefined,
        primary_color: formData.get("primary_color") as string,
        secondary_color: formData.get("secondary_color") as string,
        background_color: formData.get("background_color") as string,
        text_color: formData.get("text_color") as string,
        custom_css: (formData.get("custom_css") as string) || undefined,
        company_name: (formData.get("company_name") as string) || undefined,
        favicon_url: (formData.get("favicon_url") as string) || undefined,
        allow_registration: formData.get("allow_registration") === "true",
      };
      await serviceBrandingApi.update(id, config, accessToken || undefined);
      return { success: true, intent };
    }

    if (intent === "delete_branding") {
      await serviceBrandingApi.delete(id, accessToken || undefined);
      return { success: true, intent };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "services.errors.unknown");
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "services.errors.invalidIntent") }, { status: 400 });
}

function copyToClipboard(text: string): Promise<void> {
  return navigator.clipboard.writeText(text);
}

function CodeBlock({ children, label }: { children: string; label?: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await copyToClipboard(children);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative group">
      {label && <div className="mb-1 text-xs text-[var(--text-tertiary)]">{label}</div>}
      <div className="overflow-x-auto whitespace-pre rounded-lg bg-[#0d1117] p-4 font-mono text-sm text-[#c9d1d9]">{children}</div>
      <Button
        variant="ghost"
        size="icon"
        className="absolute top-2 right-2 h-11 w-11 text-[#8b949e] opacity-0 transition-opacity group-hover:opacity-100 hover:bg-[#30363d] hover:text-white sm:h-7 sm:w-7"
        onClick={handleCopy}
        title={t("common.buttons.copy")}
      >
        {copied ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-3.5 w-3.5" />}
      </Button>
    </div>
  );
}

function CopyValue({ value, fieldId }: { value: string; fieldId: string }) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);

  return (
    <div className="flex min-w-0 items-center gap-2">
      <code className="min-w-0 flex-1 select-all break-all whitespace-normal font-mono text-sm text-[var(--text-primary)] [word-break:break-all]">{value}</code>
      <Button
        variant="ghost"
        className="h-8 w-8 shrink-0 p-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
        onClick={async () => {
          await copyToClipboard(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        }}
        title={t("common.buttons.copyField", { field: fieldId })}
      >
        {copied ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-3.5 w-3.5" />}
      </Button>
    </div>
  );
}

function IntegrationTab({ integration }: { integration: ServiceIntegrationInfo }) {
  const { t } = useI18n();
  const [revealedSecrets, setRevealedSecrets] = useState<Set<string>>(new Set());

  const toggleReveal = (clientId: string) => {
    setRevealedSecrets((previous) => {
      const next = new Set(previous);
      if (next.has(clientId)) next.delete(clientId);
      else next.add(clientId);
      return next;
    });
  };

  const envBlock = integration.environment_variables.map((variable) => `${variable.key}=${variable.value}`).join("\n");

  const endpoints = [
    [t("services.integration.endpointLabels.authorize"), integration.endpoints.authorize],
    [t("services.integration.endpointLabels.token"), integration.endpoints.token],
    [t("services.integration.endpointLabels.callback"), integration.endpoints.callback],
    [t("services.integration.endpointLabels.logout"), integration.endpoints.logout],
    [t("services.integration.endpointLabels.userinfo"), integration.endpoints.userinfo],
    [t("services.integration.endpointLabels.openidConfiguration"), integration.endpoints.openid_configuration],
    [t("services.integration.endpointLabels.jwks"), integration.endpoints.jwks],
  ] as const;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.clientsCredentials")}</CardTitle>
          <CardDescription>{t("services.integration.clientsCredentialsDescription")}</CardDescription>
        </CardHeader>
        <div className="space-y-4 p-6 pt-0">
          {integration.clients.length === 0 && (
            <p className="text-sm text-[var(--text-secondary)]">{t("services.integration.noClientsConfigured")}</p>
          )}
          {integration.clients.map((client) => (
            <div key={client.client_id} className="space-y-3 rounded-lg border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-[var(--text-primary)]">{client.name || client.client_id}</span>
                  <span
                    className={`rounded-full px-2 py-0.5 text-[11px] font-medium ${
                      client.public_client
                        ? "bg-[var(--accent-blue)]/10 text-[var(--accent-blue)]"
                        : "bg-[var(--accent-purple)]/10 text-[var(--accent-purple)]"
                    }`}
                  >
                    {client.public_client ? t("services.integration.public") : t("services.integration.confidential")}
                  </span>
                </div>
              </div>
              <div className="space-y-2">
                <div>
                  <Label className="text-xs text-[var(--text-tertiary)]">{t("services.clientId")}</Label>
                  <CopyValue value={client.client_id} fieldId={t("services.clientId")} />
                </div>
                {client.public_client ? (
                  <div className="text-sm italic text-[var(--text-secondary)]">{t("services.integration.publicNoSecret")}</div>
                ) : (
                  <div>
                    <Label className="text-xs text-[var(--text-tertiary)]">{t("services.detail.clientSecret")}</Label>
                    {client.client_secret ? (
                      <div className="flex min-w-0 items-center gap-2">
                        <code className="min-w-0 flex-1 select-all break-all whitespace-normal font-mono text-sm text-[var(--text-primary)] [word-break:break-all]">
                          {revealedSecrets.has(client.client_id) ? client.client_secret : "••••••••••••••••••••••••"}
                        </code>
                        <Button
                          variant="ghost"
                          className="h-8 w-8 shrink-0 p-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
                          onClick={() => toggleReveal(client.client_id)}
                          title={revealedSecrets.has(client.client_id) ? t("services.integration.hide") : t("services.integration.reveal")}
                        >
                          {revealedSecrets.has(client.client_id) ? <EyeClosedIcon className="h-3.5 w-3.5" /> : <EyeOpenIcon className="h-3.5 w-3.5" />}
                        </Button>
                        <Button
                          variant="ghost"
                          className="h-8 w-8 shrink-0 p-0 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)]"
                          onClick={() => copyToClipboard(client.client_secret!)}
                          title={t("services.integration.copySecret")}
                        >
                          <CopyIcon className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    ) : (
                      <span className="text-sm italic text-[var(--text-secondary)]">{t("services.integration.keycloakUnavailable")}</span>
                    )}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.environmentVariables")}</CardTitle>
          <CardDescription>{t("services.integration.environmentVariablesDescription")}</CardDescription>
        </CardHeader>
        <div className="p-6 pt-0">
          <CodeBlock label=".env">{envBlock}</CodeBlock>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.endpoints")}</CardTitle>
          <CardDescription>{t("services.integration.endpointsDescription")}</CardDescription>
        </CardHeader>
        <div className="p-6 pt-0">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--glass-border-subtle)]">
                  <th className="py-2 pr-4 text-left font-medium text-[var(--text-secondary)]">{t("services.integration.endpoint")}</th>
                  <th className="py-2 text-left font-medium text-[var(--text-secondary)]">{t("services.integration.url")}</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {endpoints.map(([name, url]) => (
                  <tr key={name}>
                    <td className="whitespace-nowrap py-2 pr-4 font-medium text-[var(--text-primary)]">{name}</td>
                    <td className="py-2">
                      <CopyValue value={url} fieldId={name} />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("services.integration.sdkInitialization")}</CardTitle>
          <CardDescription>{t("services.integration.sdkInitializationDescription")}</CardDescription>
        </CardHeader>
        <div className="space-y-4 p-6 pt-0">
          <CodeBlock label="TypeScript - SDK Setup">{`import { Auth9 } from '@auth9/sdk';

const auth9 = new Auth9({
  domain: '${integration.endpoints.auth9_domain}',
  audience: '${integration.clients[0]?.client_id || "<your-client-id>"}',${integration.clients[0] && !integration.clients[0].public_client ? `
  clientSecret: process.env.AUTH9_CLIENT_SECRET,` : ""}
});`}</CodeBlock>

          <CodeBlock label="TypeScript - Express Middleware">{`import { auth9Middleware, requireRole } from '@auth9/express';

app.use(auth9Middleware({
  domain: process.env.AUTH9_DOMAIN!,
  audience: process.env.AUTH9_AUDIENCE!,
}));

// Protect a route with role check
app.get('/admin', requireRole('admin'), (req, res) => {
  res.json({ user: req.auth });
});`}</CodeBlock>

          <CodeBlock label="TypeScript - gRPC Token Exchange">{`import { Auth9GrpcClient } from '@auth9/grpc';

const grpc = new Auth9GrpcClient({
  address: '${integration.grpc.address}',
  apiKey: process.env.AUTH9_GRPC_API_KEY!,
});

// Exchange identity token first, then use tenant access token for downstream calls
const { accessToken } = await grpc.exchangeToken({
  identityToken: userIdToken,
  tenantId: 'tenant-uuid',
  audience: '${integration.clients[0]?.client_id || "<your-client-id>"}',
});`}</CodeBlock>
        </div>
      </Card>
    </div>
  );
}

const DEFAULT_BRANDING: BrandingConfig = {
  primary_color: "#007AFF",
  secondary_color: "#5856D6",
  background_color: "#F5F5F7",
  text_color: "#1D1D1F",
  allow_registration: false,
};

function ColorPicker({
  id,
  label,
  value,
  onChange,
  defaultValue,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  defaultValue: string;
}) {
  const { t } = useI18n();

  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="flex items-center gap-2">
        <label htmlFor={`${id}_picker`} className="block h-10 w-10 cursor-pointer rounded-md border border-gray-300 shadow-sm" style={{ backgroundColor: value }}>
          <span className="sr-only">{t("services.branding.chooseColor", { label })}</span>
        </label>
        <input type="color" id={`${id}_picker`} value={value} onChange={(event) => onChange(event.target.value)} className="sr-only" />
        <Input id={id} name={id} value={value} onChange={(event) => onChange(event.target.value)} placeholder={defaultValue} className="font-mono uppercase" maxLength={7} />
      </div>
    </div>
  );
}

function ActionsTab({ actions, serviceId }: { actions: Action[]; serviceId: string }) {
  const { t } = useI18n();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">{t("serviceActions.title")}</h3>
          <p className="text-sm text-[var(--text-secondary)]">{t("services.detail.tabs.actions", { count: actions.length })}</p>
        </div>
        <Button asChild>
          <Link to={`/dashboard/services/${serviceId}/actions/new`}>
            <PlusIcon className="mr-2 h-4 w-4" />
            {t("serviceActions.newAction")}
          </Link>
        </Button>
      </div>

      {actions.length === 0 ? (
        <Card>
          <CardContent className="py-12">
            <div className="text-center">
              <LightningBoltIcon className="mx-auto mb-3 h-8 w-8 text-[var(--text-tertiary)]" />
              <h3 className="mb-2 text-lg font-semibold">{t("serviceActions.noActions")}</h3>
              <p className="mb-4 text-[var(--text-secondary)]">{t("serviceActions.noActionsDescription")}</p>
              <Button asChild>
                <Link to={`/dashboard/services/${serviceId}/actions/new`}>
                  <PlusIcon className="mr-2 h-4 w-4" />
                  {t("serviceActions.createAction")}
                </Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {actions.map((item) => (
            <Card key={item.id}>
              <div className="p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Link to={`/dashboard/services/${serviceId}/actions/${item.id}`} className="font-medium hover:underline">
                      {item.name}
                    </Link>
                    <Badge variant={item.enabled ? "default" : "secondary"}>
                      {item.enabled ? t("serviceActions.enabled") : t("serviceActions.disabled")}
                    </Badge>
                    <Badge variant="outline">{item.trigger_id}</Badge>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button asChild variant="outline" size="sm">
                      <Link to={`/dashboard/services/${serviceId}/actions/${item.id}`}>{t("serviceActions.viewDetails")}</Link>
                    </Button>
                    <Button asChild variant="outline" size="sm">
                      <Link to={`/dashboard/services/${serviceId}/actions/${item.id}/edit`}>{t("serviceActions.edit")}</Link>
                    </Button>
                  </div>
                </div>
                {item.description && <p className="mt-1 text-sm text-[var(--text-secondary)]">{item.description}</p>}
              </div>
            </Card>
          ))}
          <div className="pt-2 text-center">
            <Button asChild variant="outline">
              <Link to={`/dashboard/services/${serviceId}/actions`}>{t("serviceActions.title")}</Link>
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

function BrandingTab({ branding }: { branding: BrandingConfig | null }) {
  const navigation = useNavigation();
  const actionData = useActionData<typeof action>();
  const { t } = useI18n();
  const [isCustomizing, setIsCustomizing] = useState(Boolean(branding));

  const config = branding || DEFAULT_BRANDING;
  const [logoUrl, setLogoUrl] = useState(config.logo_url || "");
  const [primaryColor, setPrimaryColor] = useState(config.primary_color);
  const [secondaryColor, setSecondaryColor] = useState(config.secondary_color);
  const [backgroundColor, setBackgroundColor] = useState(config.background_color);
  const [textColor, setTextColor] = useState(config.text_color);
  const [customCss, setCustomCss] = useState(config.custom_css || "");
  const [companyName, setCompanyName] = useState(config.company_name || "");
  const [faviconUrl, setFaviconUrl] = useState(config.favicon_url || "");
  const [allowRegistration, setAllowRegistration] = useState(config.allow_registration ?? false);

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent");

  const resetToDefault = () => {
    setLogoUrl("");
    setPrimaryColor(DEFAULT_BRANDING.primary_color);
    setSecondaryColor(DEFAULT_BRANDING.secondary_color);
    setBackgroundColor(DEFAULT_BRANDING.background_color);
    setTextColor(DEFAULT_BRANDING.text_color);
    setCustomCss("");
    setCompanyName("");
    setFaviconUrl("");
    setAllowRegistration(false);
  };

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success && actionData.intent === "delete_branding") {
      setIsCustomizing(false);
      resetToDefault();
    }
  }, [actionData]);

  if (!isCustomizing) {
    return (
      <Card>
        <CardContent className="py-12">
          <div className="text-center">
            <h3 className="mb-2 text-lg font-semibold">{t("services.branding.systemDefaultTitle")}</h3>
            <p className="mb-4 text-[var(--text-secondary)]">{t("services.branding.systemDefaultDescription")}</p>
            <Button onClick={() => setIsCustomizing(true)}>{t("services.branding.customize")}</Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {actionData && "success" in actionData && actionData.success && actionData.intent === "update_branding" && (
        <div className="flex items-center gap-2 rounded-xl border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-4 text-sm text-[var(--accent-green)]">
          <CheckCircledIcon className="h-4 w-4" />
          {t("services.detail.brandingSaved")}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("services.branding.title")}</CardTitle>
          <CardDescription>{t("services.branding.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6">
            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">{t("services.branding.companyIdentity")}</h3>
              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="company_name">{t("services.branding.companyName")}</Label>
                  <Input id="company_name" name="company_name" placeholder={t("services.branding.companyNamePlaceholder")} value={companyName} onChange={(event) => setCompanyName(event.target.value)} maxLength={100} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="logo_url">{t("services.branding.logoUrl")}</Label>
                  <Input id="logo_url" name="logo_url" type="url" placeholder={t("services.branding.logoUrlPlaceholder")} value={logoUrl} onChange={(event) => setLogoUrl(event.target.value)} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="favicon_url">{t("services.branding.faviconUrl")}</Label>
                  <Input id="favicon_url" name="favicon_url" type="url" placeholder={t("services.branding.faviconUrlPlaceholder")} value={faviconUrl} onChange={(event) => setFaviconUrl(event.target.value)} />
                </div>
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">{t("services.branding.loginOptions")}</h3>
              <div className="flex items-center justify-between">
                <div className="space-y-0.5">
                  <Label htmlFor="allow_registration">{t("services.branding.allowRegistration")}</Label>
                  <p className="text-xs text-[var(--text-secondary)]">{t("services.branding.allowRegistrationHint")}</p>
                </div>
                <label htmlFor="allow_registration" className="relative inline-flex cursor-pointer items-center">
                  <span className="sr-only">{t("services.branding.toggleAllowRegistration")}</span>
                  <input type="checkbox" id="allow_registration" name="allow_registration" value="true" checked={allowRegistration} onChange={(event) => setAllowRegistration(event.target.checked)} className="peer sr-only" />
                  <div className="peer h-6 w-11 rounded-full bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white after:absolute after:top-[2px] after:left-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-['']"></div>
                </label>
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">{t("services.branding.colors")}</h3>
              <div className="grid grid-cols-2 gap-4">
                <ColorPicker id="primary_color" label={t("services.branding.primaryColor")} value={primaryColor} onChange={setPrimaryColor} defaultValue={DEFAULT_BRANDING.primary_color} />
                <ColorPicker id="secondary_color" label={t("services.branding.secondaryColor")} value={secondaryColor} onChange={setSecondaryColor} defaultValue={DEFAULT_BRANDING.secondary_color} />
                <ColorPicker id="background_color" label={t("services.branding.backgroundColor")} value={backgroundColor} onChange={setBackgroundColor} defaultValue={DEFAULT_BRANDING.background_color} />
                <ColorPicker id="text_color" label={t("services.branding.textColor")} value={textColor} onChange={setTextColor} defaultValue={DEFAULT_BRANDING.text_color} />
              </div>
            </div>

            <div className="space-y-2">
              <h3 className="border-b pb-2 text-sm font-medium text-[var(--text-primary)]">
                {t("services.branding.customCss")}
                <span className="ml-2 font-normal text-[var(--text-secondary)]">({t("services.branding.advanced")})</span>
              </h3>
              <Textarea id="custom_css" name="custom_css" placeholder={t("services.branding.customCssPlaceholder")} value={customCss} onChange={(event) => setCustomCss(event.target.value)} className="min-h-[120px] font-mono text-sm" />
            </div>

            <div className="flex flex-wrap items-center gap-3 border-t pt-4">
              <Button type="submit" name="intent" value="update_branding" disabled={isSubmitting && currentIntent === "update_branding"}>
                {isSubmitting && currentIntent === "update_branding" ? t("services.detail.saving") : t("services.branding.saveBranding")}
              </Button>

              {branding ? (
                <Button type="submit" name="intent" value="delete_branding" variant="destructive" disabled={isSubmitting}>
                  <ResetIcon className="mr-2 h-4 w-4" />
                  {t("services.branding.resetToDefault")}
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="destructive"
                  disabled={isSubmitting}
                  onClick={() => {
                    resetToDefault();
                    setIsCustomizing(false);
                  }}
                >
                  <ResetIcon className="mr-2 h-4 w-4" />
                  {t("services.branding.resetToDefault")}
                </Button>
              )}
            </div>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}

export default function ServiceDetailPage() {
  const { service, clients, integration, actions, branding } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const confirm = useConfirm();
  const { t } = useI18n();
  const formatters = useFormatters();

  const [isAddClientOpen, setIsAddClientOpen] = useState(false);
  const [secretDialog, setSecretDialog] = useState<{ clientId: string; secret: string; isNew: boolean } | null>(null);
  const [copiedField, setCopiedField] = useState<string | null>(null);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      if (actionData.intent === "create_client" && "secret" in actionData && "clientId" in actionData && actionData.secret && actionData.clientId) {
        setIsAddClientOpen(false);
        setSecretDialog({ clientId: actionData.clientId as string, secret: actionData.secret as string, isNew: true });
      }
      if (actionData.intent === "regenerate_secret" && "secret" in actionData && "regeneratedClientId" in actionData) {
        setSecretDialog({
          clientId: actionData.regeneratedClientId as string,
          secret: actionData.secret as string,
          isNew: false,
        });
      }
    }
  }, [actionData]);

  const handleCopy = async (text: string, fieldName: string) => {
    await copyToClipboard(text);
    setCopiedField(fieldName);
    setTimeout(() => setCopiedField(null), 2000);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to="/dashboard/services">
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-[24px] font-semibold tracking-tight text-[var(--text-primary)]">{service.name}</h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("services.detail.description")}</p>
        </div>
      </div>

      <Tabs defaultValue="configuration">
        <TabsList>
          <TabsTrigger value="configuration">{t("services.detail.tabs.configuration")}</TabsTrigger>
          <TabsTrigger value="integration">{t("services.detail.tabs.integration")}</TabsTrigger>
          <TabsTrigger value="actions">{t("services.detail.tabs.actions", { count: actions.length })}</TabsTrigger>
          <TabsTrigger value="branding">{t("services.detail.tabs.branding")}</TabsTrigger>
        </TabsList>

        <TabsContent value="configuration">
          <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
            <div className="md:col-span-2">
              <Card>
                <CardHeader>
                  <CardTitle>{t("services.detail.configurationTitle")}</CardTitle>
                  <CardDescription>{t("services.detail.configurationDescription")}</CardDescription>
                </CardHeader>
                <div className="p-6">
                  {actionData && "error" in actionData && (
                    <div className="mb-4 rounded-lg border border-[var(--accent-red)]/30 bg-[var(--accent-red)]/10 p-3 text-sm text-[var(--accent-red)]">
                      {String(actionData.error)}
                    </div>
                  )}
                  <Form method="post" className="space-y-4">
                    <input type="hidden" name="intent" value="update_service" />
                    <div className="space-y-2">
                      <Label htmlFor="name">{t("services.serviceName")}</Label>
                      <Input id="name" name="name" defaultValue={service.name} required />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="base_url">{t("services.baseUrl")}</Label>
                      <Input id="base_url" name="base_url" defaultValue={service.base_url} placeholder={t("services.baseUrlPlaceholder")} />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="redirect_uris">{t("services.redirectUris")}</Label>
                      <Input id="redirect_uris" name="redirect_uris" defaultValue={service.redirect_uris?.join(", ")} />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="logout_uris">{t("services.logoutUris")}</Label>
                      <Input id="logout_uris" name="logout_uris" defaultValue={service.logout_uris?.join(", ")} />
                    </div>
                    <div className="flex justify-end pt-4">
                      <Button type="submit" disabled={isSubmitting}>
                        {isSubmitting ? t("services.detail.saving") : t("services.detail.saveChanges")}
                      </Button>
                    </div>
                  </Form>
                </div>
              </Card>
            </div>

            <div>
              <Card className="h-full">
                <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                  <div className="space-y-1">
                    <CardTitle>{t("services.detail.clientsTitle")}</CardTitle>
                    <CardDescription>{t("services.detail.clientsDescription")}</CardDescription>
                  </div>
                  <Dialog open={isAddClientOpen} onOpenChange={setIsAddClientOpen}>
                    <DialogTrigger asChild>
                      <Button size="sm" variant="outline" title={t("services.detail.createClientTitle")}>
                        <PlusIcon className="h-4 w-4" />
                      </Button>
                    </DialogTrigger>
                    <DialogContent>
                      <DialogHeader>
                        <DialogTitle>{t("services.detail.createClientTitle")}</DialogTitle>
                        <DialogDescription>{t("services.detail.createClientDescription")}</DialogDescription>
                      </DialogHeader>
                      <Form method="post" className="space-y-4">
                        <input type="hidden" name="intent" value="create_client" />
                        <div className="space-y-2">
                          <Label htmlFor="client-name">{t("services.detail.clientDescriptionOptional")}</Label>
                          <Input id="client-name" name="name" placeholder={t("services.detail.clientDescriptionPlaceholder")} />
                        </div>
                        <DialogFooter>
                          <Button type="button" variant="outline" onClick={() => setIsAddClientOpen(false)}>
                            {t("common.buttons.cancel")}
                          </Button>
                          <Button type="submit" disabled={isSubmitting}>
                            {t("services.detail.create")}
                          </Button>
                        </DialogFooter>
                      </Form>
                    </DialogContent>
                  </Dialog>
                </CardHeader>
                <div className="p-0">
                  <ul className="divide-y divide-[var(--glass-border-subtle)]">
                    {clients.map((client) => (
                      <li key={client.id} className="p-4 hover:bg-[var(--sidebar-item-hover)]">
                        <div className="mb-2 flex items-start justify-between">
                          <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                              <code className="truncate font-mono text-sm font-medium text-[var(--text-primary)]">{client.client_id}</code>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-11 w-11 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] sm:h-6 sm:w-6"
                                onClick={() => handleCopy(client.client_id, `client-${client.id}`)}
                                title={t("services.detail.copyClientId")}
                              >
                                {copiedField === `client-${client.id}` ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-3 w-3" />}
                              </Button>
                            </div>
                            <div className="mt-1 text-xs text-[var(--text-secondary)]">{client.name || t("services.detail.noDescription")}</div>
                            <div className="mt-0.5 text-xs text-[var(--text-tertiary)]">
                              {t("services.detail.createdOn", { date: formatters.date(client.created_at) })}
                            </div>
                          </div>
                        </div>
                        <div className="mt-2 flex items-center gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-7 text-xs"
                            onClick={async () => {
                              const confirmed = await confirm({
                                title: t("services.detail.regenerateSecretTitle"),
                                description: t("services.detail.regenerateSecretDescription"),
                                confirmLabel: t("services.detail.regenerate"),
                              });
                              if (confirmed) {
                                submit({ intent: "regenerate_secret", client_id: client.client_id }, { method: "post" });
                              }
                            }}
                          >
                            <UpdateIcon className="mr-1 h-3 w-3" />
                            {t("services.detail.regenerate")}
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 text-xs text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10 hover:text-[var(--accent-red)]"
                            onClick={async () => {
                              const confirmed = await confirm({
                                title: t("services.detail.deleteClientTitle"),
                                description: t("services.detail.deleteClientDescription"),
                                variant: "destructive",
                              });
                              if (confirmed) {
                                submit({ intent: "delete_client", client_id: client.client_id }, { method: "post" });
                              }
                            }}
                          >
                            <TrashIcon className="mr-1 h-3 w-3" />
                            {t("common.buttons.delete")}
                          </Button>
                        </div>
                      </li>
                    ))}
                    {clients.length === 0 && <li className="p-4 text-center text-sm text-[var(--text-secondary)]">{t("services.detail.noClients")}</li>}
                  </ul>
                </div>
              </Card>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="integration">
          {integration ? (
            <IntegrationTab integration={integration} />
          ) : (
            <Card>
              <div className="p-6 text-center text-[var(--text-secondary)]">
                <p>{t("services.detail.integrationUnavailable")}</p>
              </div>
            </Card>
          )}
        </TabsContent>

        <TabsContent value="actions">
          <ActionsTab actions={actions} serviceId={service.id} />
        </TabsContent>

        <TabsContent value="branding">
          <BrandingTab branding={branding} />
        </TabsContent>
      </Tabs>

      <Dialog open={Boolean(secretDialog)} onOpenChange={(open) => !open && setSecretDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {secretDialog?.isNew ? t("services.detail.secretDialogNewTitle") : t("services.detail.secretDialogRegeneratedTitle")}
            </DialogTitle>
            <DialogDescription>{t("services.detail.secretDialogDescription")}</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label className="text-xs text-[var(--text-secondary)]">{t("services.detail.clientCreated")}</Label>
              <div className="mt-1 flex items-center gap-2">
                <div className="flex-1 select-all break-all rounded border bg-[var(--sidebar-item-hover)] p-2 font-mono text-sm">{secretDialog?.clientId}</div>
                <Button variant="outline" size="icon" className="h-8 w-8 shrink-0" onClick={() => secretDialog && handleCopy(secretDialog.clientId, "dialog-id")}>
                  {copiedField === "dialog-id" ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-4 w-4" />}
                </Button>
              </div>
            </div>
            <div>
              <Label className="text-xs text-[var(--text-secondary)]">{t("services.detail.clientSecret")}</Label>
              <div className="mt-1 flex items-center gap-2">
                <div className="flex-1 select-all break-all rounded border border-[var(--accent-green)]/20 bg-[var(--accent-green)]/10 p-3 text-center font-mono font-bold text-[var(--accent-green)] [word-break:break-all]">
                  {secretDialog?.secret}
                </div>
                <Button variant="outline" size="icon" className="h-8 w-8 shrink-0" onClick={() => secretDialog && handleCopy(secretDialog.secret, "dialog-secret")}>
                  {copiedField === "dialog-secret" ? <span className="text-xs text-[var(--accent-green)]">&#10003;</span> : <CopyIcon className="h-4 w-4" />}
                </Button>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button onClick={() => setSecretDialog(null)}>{t("services.detail.close")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
