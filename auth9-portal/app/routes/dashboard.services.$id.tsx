import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Button } from "~/components/ui/button";
import { Card } from "~/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { ServiceActionsTab } from "~/components/services/service-actions-tab";
import { ServiceBrandingTab } from "~/components/services/service-branding-tab";
import { ServiceConfigurationTab } from "~/components/services/service-configuration-tab";
import { ServiceIntegrationTab } from "~/components/services/service-integration-tab";
import { ServiceSecretDialog } from "~/components/services/service-secret-dialog";
import type { ServiceSecretDialogState } from "~/components/services/types";
import { mapApiError } from "~/lib/error-messages";
import { useConfirm } from "~/hooks/useConfirm";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import {
  actionApi,
  serviceApi,
  serviceBrandingApi,
  type Action,
  type BrandingConfig,
  type ServiceIntegrationInfo,
} from "~/services/api";
import { resolveLocale } from "~/services/locale.server";
import { getAccessToken } from "~/services/session.server";

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

  if (!id) {
    throw new Error(translate(locale, "services.errors.serviceIdRequired"));
  }

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
    const message = mapApiError(error, locale);
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: translate(locale, "services.errors.invalidIntent") }, { status: 400 });
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
  const [secretDialog, setSecretDialog] = useState<ServiceSecretDialogState | null>(null);

  const isSubmitting = navigation.state === "submitting";
  const currentIntent = navigation.formData?.get("intent")?.toString() ?? null;
  const actionError = actionData && "error" in actionData ? String(actionData.error) : null;
  const updateBrandingSucceeded =
    Boolean(actionData && "success" in actionData && actionData.success && actionData.intent === "update_branding");
  const deleteBrandingSucceeded =
    Boolean(actionData && "success" in actionData && actionData.success && actionData.intent === "delete_branding");

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      if (
        actionData.intent === "create_client" &&
        "secret" in actionData &&
        "clientId" in actionData &&
        actionData.secret &&
        actionData.clientId
      ) {
        setIsAddClientOpen(false);
        setSecretDialog({
          clientId: actionData.clientId as string,
          secret: actionData.secret as string,
          isNew: true,
        });
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

  const handleRegenerateSecret = async (clientId: string) => {
    const confirmed = await confirm({
      title: t("services.detail.regenerateSecretTitle"),
      description: t("services.detail.regenerateSecretDescription"),
      confirmLabel: t("services.detail.regenerate"),
    });

    if (confirmed) {
      submit({ intent: "regenerate_secret", client_id: clientId }, { method: "post" });
    }
  };

  const handleDeleteClient = async (clientId: string) => {
    const confirmed = await confirm({
      title: t("services.detail.deleteClientTitle"),
      description: t("services.detail.deleteClientDescription"),
      variant: "destructive",
    });

    if (confirmed) {
      submit({ intent: "delete_client", client_id: clientId }, { method: "post" });
    }
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
          <ServiceConfigurationTab
            actionError={actionError}
            clients={clients}
            formatDate={(value) => formatters.date(value)}
            isAddClientOpen={isAddClientOpen}
            isSubmitting={isSubmitting}
            service={service}
            onAddClientOpenChange={setIsAddClientOpen}
            onDeleteClient={handleDeleteClient}
            onRegenerateSecret={handleRegenerateSecret}
          />
        </TabsContent>

        <TabsContent value="integration">
          {integration ? (
            <ServiceIntegrationTab integration={integration as ServiceIntegrationInfo} />
          ) : (
            <Card>
              <div className="p-6 text-center text-[var(--text-secondary)]">
                <p>{t("services.detail.integrationUnavailable")}</p>
              </div>
            </Card>
          )}
        </TabsContent>

        <TabsContent value="actions">
          <ServiceActionsTab actions={actions} serviceId={service.id} />
        </TabsContent>

        <TabsContent value="branding">
          <ServiceBrandingTab
            branding={branding}
            currentIntent={currentIntent}
            deleteSucceeded={deleteBrandingSucceeded}
            isSubmitting={isSubmitting}
            updateSucceeded={updateBrandingSucceeded}
          />
        </TabsContent>
      </Tabs>

      <ServiceSecretDialog secretDialog={secretDialog} onOpenChange={(open) => !open && setSecretDialog(null)} />
    </div>
  );
}
