import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "react-router";
import { useEffect, useMemo, useRef, useState } from "react";
import { Pencil2Icon, PlusIcon, TrashIcon } from "@radix-ui/react-icons";
import { SettingsHeroCard } from "~/components/settings/settings-card-header";
import { Button } from "~/components/ui/button";
import { Card, CardContent } from "~/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Switch } from "~/components/ui/switch";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { getAccessToken } from "~/services/session.server";
import { resolveLocale } from "~/services/locale.server";
import { identityProviderApi, type CreateIdentityProviderInput, type IdentityProvider } from "~/services/api";

const PROVIDER_TEMPLATES = [
  { provider_id: "google", key: "google", icon: "G", color: "bg-red-500", requiredFields: ["clientId", "clientSecret"] },
  { provider_id: "github", key: "github", icon: "GH", color: "bg-gray-900", requiredFields: ["clientId", "clientSecret"] },
  { provider_id: "microsoft", key: "microsoft", icon: "M", color: "bg-blue-600", requiredFields: ["clientId", "clientSecret"] },
  { provider_id: "oidc", key: "oidc", icon: "OIDC", color: "bg-purple-600", requiredFields: ["clientId", "clientSecret", "authorizationUrl", "tokenUrl"] },
  { provider_id: "saml", key: "saml", icon: "SAML", color: "bg-orange-600", requiredFields: ["entityId", "singleSignOnServiceUrl", "signingCertificate"] },
] as const;

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "settings.identityProvidersPage.metaTitle");

function getProviderTemplate(providerId: string) {
  return PROVIDER_TEMPLATES.find((template) => template.provider_id === providerId);
}

function templateHasField(
  template: (typeof PROVIDER_TEMPLATES)[number] | undefined,
  field: string
) {
  return Boolean(template && (template.requiredFields as readonly string[]).includes(field));
}

function buildConfig(formData: FormData) {
  const rawConfig = formData.get("config") as string | null;
  if (rawConfig) {
    try {
      return JSON.parse(rawConfig) as Record<string, string>;
    } catch {
      return {};
    }
  }

  const config: Record<string, string> = {};
  for (const key of ["clientId", "clientSecret", "authorizationUrl", "tokenUrl", "entityId", "singleSignOnServiceUrl", "signingCertificate"]) {
    const value = formData.get(key) as string | null;
    if (value) config[key] = value;
  }
  return config;
}

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  try {
    const response = await identityProviderApi.list(accessToken || undefined);
    return { providers: response.data };
  } catch {
    return { providers: [], error: translate(locale, "settings.identityProvidersPage.loadFailed") };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const input: CreateIdentityProviderInput = {
        alias: (formData.get("alias") as string) || (formData.get("providerId") as string),
        provider_id: formData.get("providerId") as string,
        display_name: (formData.get("displayName") as string) || undefined,
        enabled: formData.get("enabled") === "true",
        config: buildConfig(formData),
      };
      await identityProviderApi.create(input, accessToken || undefined);
      return { success: true, message: translate(locale, "settings.identityProvidersPage.created") };
    }

    if (intent === "update") {
      const alias = formData.get("alias") as string;
      const input: Partial<CreateIdentityProviderInput> = {
        display_name: (formData.get("displayName") as string) || undefined,
        enabled: formData.get("enabled") === "true",
        config: buildConfig(formData),
      };
      await identityProviderApi.update(alias, input, accessToken || undefined);
      return { success: true, message: translate(locale, "settings.identityProvidersPage.updated") };
    }

    if (intent === "delete") {
      const alias = formData.get("alias") as string;
      await identityProviderApi.delete(alias, accessToken || undefined);
      return { success: true, message: translate(locale, "settings.identityProvidersPage.deleted") };
    }

    if (intent === "toggle") {
      const alias = formData.get("alias") as string;
      const enabled = formData.get("enabled") === "true";
      await identityProviderApi.update(alias, { enabled }, accessToken || undefined);
      return { success: true };
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }

  return { error: translate(locale, "settings.identityProvidersPage.invalidAction") };
}

export default function IdentityProvidersPage() {
  const { providers, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const submit = useSubmit();
  const { t } = useI18n();

  const [showDialog, setShowDialog] = useState(false);
  const [editingProvider, setEditingProvider] = useState<IdentityProvider | null>(null);
  const [selectedTemplate, setSelectedTemplate] = useState("");
  const [formData, setFormData] = useState({ alias: "", displayName: "", enabled: true, config: {} as Record<string, string> });
  const isSubmitting = navigation.state === "submitting";
  const wasSubmitting = useRef(false);

  useEffect(() => {
    if (isSubmitting) {
      wasSubmitting.current = true;
    }
    if (wasSubmitting.current && !isSubmitting && actionData?.success && (showDialog || editingProvider)) {
      setShowDialog(false);
      setEditingProvider(null);
      resetForm();
      wasSubmitting.current = false;
    }
  }, [actionData, editingProvider, isSubmitting, showDialog]);

  function resetForm() {
    setFormData({ alias: "", displayName: "", enabled: true, config: {} });
    setSelectedTemplate("");
  }

  function openEditDialog(provider: IdentityProvider) {
    wasSubmitting.current = false;
    setEditingProvider(provider);
    setFormData({ alias: provider.alias, displayName: provider.display_name || "", enabled: provider.enabled, config: provider.config });
    setSelectedTemplate(provider.provider_id);
  }

  function openCreateDialog() {
    wasSubmitting.current = false;
    resetForm();
    setShowDialog(true);
  }

  const template = getProviderTemplate(selectedTemplate);
  const isDuplicateAlias = useMemo(() => {
    if (editingProvider) return false;
    const alias = formData.alias.trim();
    if (!alias) return false;
    return providers.some((provider) => provider.alias.toLowerCase() === alias.toLowerCase());
  }, [editingProvider, formData.alias, providers]);

  const hasRequiredFields = !template || template.requiredFields.every((field) => formData.config[field] && formData.config[field].trim() !== "");

  return (
    <div className="space-y-6">
      <SettingsHeroCard
        title={t("settings.identityProvidersPage.title")}
        description={t("settings.identityProvidersPage.description")}
        actions={
          <Button onClick={openCreateDialog} className="w-full sm:w-auto">
              <PlusIcon className="mr-2 h-4 w-4" />
              {t("settings.identityProvidersPage.addProvider")}
          </Button>
        }
      />

      {loadError && <div className="rounded-md bg-red-50 p-3 text-sm text-[var(--accent-red)]">{loadError}</div>}
      {actionData?.error && <div className="rounded-md bg-red-50 p-3 text-sm text-[var(--accent-red)]">{actionData.error}</div>}
      {actionData?.success && actionData.message && <div className="rounded-md bg-[var(--accent-green)]/10 p-3 text-sm text-[var(--accent-green)]">{actionData.message}</div>}

      <Card>
        <CardContent className="pt-6">
          {providers.length === 0 ? (
            <div className="py-12 text-center">
              <h3 className="mb-2 text-lg font-medium text-[var(--text-primary)]">{t("settings.identityProvidersPage.emptyTitle")}</h3>
              <p className="mb-4 text-[var(--text-secondary)]">{t("settings.identityProvidersPage.emptyDescription")}</p>
              <Button onClick={openCreateDialog}>
                <PlusIcon className="mr-2 h-4 w-4" />
                {t("settings.identityProvidersPage.addFirst")}
              </Button>
            </div>
          ) : (
            <div className="divide-y">
              {providers.map((provider) => {
                const providerTemplate = getProviderTemplate(provider.provider_id);
                return (
                  <div key={provider.alias} className="flex items-center gap-4 py-4 first:pt-0 last:pb-0">
                    <div className={`flex h-10 w-10 items-center justify-center rounded-lg text-xs font-bold text-white ${providerTemplate?.color || "bg-gray-600"}`}>
                      {providerTemplate?.icon || provider.provider_id.slice(0, 2).toUpperCase()}
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="font-medium">{provider.display_name || (providerTemplate ? t(`settings.identityProvidersPage.templates.${providerTemplate.key}`) : provider.alias)}</div>
                      <div className="text-sm text-[var(--text-secondary)]">{provider.alias} • {provider.provider_id}</div>
                    </div>
                    <div className="flex items-center gap-4">
                      <Switch
                        checked={provider.enabled}
                        onCheckedChange={() => {
                          submit({ intent: "toggle", alias: provider.alias, enabled: String(!provider.enabled) }, { method: "post" });
                        }}
                      />
                      <Button variant="ghost" size="sm" aria-label={t("settings.identityProvidersPage.editAria")} onClick={() => openEditDialog(provider)}>
                        <Pencil2Icon className="h-4 w-4" />
                      </Button>
                      <Form method="post">
                        <input type="hidden" name="intent" value="delete" />
                        <input type="hidden" name="alias" value={provider.alias} />
                        <Button type="submit" variant="ghost" size="sm" aria-label={t("settings.identityProvidersPage.deleteAria")} className="text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10 hover:text-[var(--accent-red)]" disabled={isSubmitting}>
                          <TrashIcon className="h-4 w-4" />
                        </Button>
                      </Form>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      <Dialog
        open={showDialog || !!editingProvider}
        onOpenChange={(open) => {
          if (!open) {
            setShowDialog(false);
            setEditingProvider(null);
            resetForm();
          }
        }}
      >
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>{editingProvider ? t("settings.identityProvidersPage.dialogEditTitle") : t("settings.identityProvidersPage.dialogCreateTitle")}</DialogTitle>
            <DialogDescription>{editingProvider ? t("settings.identityProvidersPage.dialogEditDescription") : t("settings.identityProvidersPage.dialogCreateDescription")}</DialogDescription>
          </DialogHeader>

          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
              submit(
                {
                  intent: editingProvider ? "update" : "create",
                  alias: formData.alias,
                  providerId: selectedTemplate,
                  config: JSON.stringify(formData.config),
                  enabled: formData.enabled ? "true" : "false",
                  ...(formData.displayName ? { displayName: formData.displayName } : {}),
                },
                { method: "post" }
              );
            }}
          >
            {actionData?.error && (showDialog || editingProvider) && (
              <div className="rounded-md bg-red-50 p-3 text-sm text-[var(--accent-red)]">{actionData.error}</div>
            )}
            {!editingProvider && (
              <div className="space-y-2">
                <Label>{t("settings.identityProvidersPage.providerType")}</Label>
                <div className="grid grid-cols-3 gap-2">
                  {PROVIDER_TEMPLATES.map((providerTemplate) => (
                    <button
                      key={providerTemplate.provider_id}
                      type="button"
                      onClick={() => {
                        setSelectedTemplate(providerTemplate.provider_id);
                        setFormData((previous) => ({ ...previous, alias: providerTemplate.provider_id }));
                      }}
                      className={`rounded-lg border-2 p-3 text-center transition-colors ${selectedTemplate === providerTemplate.provider_id ? "border-blue-500 bg-blue-50" : "border-[var(--glass-border-subtle)] hover:border-gray-300"}`}
                    >
                      <div className={`mx-auto flex h-8 w-8 items-center justify-center rounded-md text-xs font-bold text-white ${providerTemplate.color}`}>{providerTemplate.icon}</div>
                      <div className="mt-1 text-sm font-medium">{t(`settings.identityProvidersPage.templates.${providerTemplate.key}`)}</div>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {(selectedTemplate || editingProvider) && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="alias">{t("settings.identityProvidersPage.alias")}</Label>
                  <Input id="alias" value={formData.alias} onChange={(event) => setFormData((previous) => ({ ...previous, alias: event.target.value }))} disabled={!!editingProvider} placeholder={t("settings.identityProvidersPage.aliasPlaceholder")} />
                  {isDuplicateAlias && <p className="text-sm text-[var(--accent-red)]">{t("settings.identityProvidersPage.aliasExists")}</p>}
                </div>

                <div className="space-y-2">
                  <Label htmlFor="displayName">{t("settings.identityProvidersPage.displayName")}</Label>
                  <Input id="displayName" name="displayName" value={formData.displayName} onChange={(event) => setFormData((previous) => ({ ...previous, displayName: event.target.value }))} placeholder={t("settings.identityProvidersPage.displayNamePlaceholder")} />
                </div>

                {templateHasField(template, "clientId") && (
                  <div className="space-y-2"><Label htmlFor="clientId">{t("settings.identityProvidersPage.clientId")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="clientId" required value={formData.config.clientId || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, clientId: event.target.value } }))} placeholder={t("settings.identityProvidersPage.clientIdPlaceholder")} /></div>
                )}
                {templateHasField(template, "clientSecret") && (
                  <div className="space-y-2"><Label htmlFor="clientSecret">{t("settings.identityProvidersPage.clientSecret")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="clientSecret" type="password" required value={formData.config.clientSecret || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, clientSecret: event.target.value } }))} placeholder={t("settings.identityProvidersPage.clientSecretPlaceholder")} /></div>
                )}
                {templateHasField(template, "authorizationUrl") && (
                  <div className="space-y-2"><Label htmlFor="authorizationUrl">{t("settings.identityProvidersPage.authorizationUrl")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="authorizationUrl" required value={formData.config.authorizationUrl || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, authorizationUrl: event.target.value } }))} placeholder={t("settings.identityProvidersPage.authorizationUrlPlaceholder")} /></div>
                )}
                {templateHasField(template, "tokenUrl") && (
                  <div className="space-y-2"><Label htmlFor="tokenUrl">{t("settings.identityProvidersPage.tokenUrl")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="tokenUrl" required value={formData.config.tokenUrl || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, tokenUrl: event.target.value } }))} placeholder={t("settings.identityProvidersPage.tokenUrlPlaceholder")} /></div>
                )}
                {templateHasField(template, "entityId") && (
                  <div className="space-y-2"><Label htmlFor="entityId">{t("settings.identityProvidersPage.entityId")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="entityId" required value={formData.config.entityId || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, entityId: event.target.value } }))} placeholder={t("settings.identityProvidersPage.entityIdPlaceholder")} /></div>
                )}
                {templateHasField(template, "singleSignOnServiceUrl") && (
                  <div className="space-y-2"><Label htmlFor="singleSignOnServiceUrl">{t("settings.identityProvidersPage.singleSignOnServiceUrl")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="singleSignOnServiceUrl" required value={formData.config.singleSignOnServiceUrl || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, singleSignOnServiceUrl: event.target.value } }))} placeholder={t("settings.identityProvidersPage.singleSignOnServiceUrlPlaceholder")} /></div>
                )}
                {templateHasField(template, "signingCertificate") && (
                  <div className="space-y-2"><Label htmlFor="signingCertificate">{t("settings.identityProvidersPage.signingCertificate")} <span className="text-[var(--accent-red)]">*</span></Label><Input id="signingCertificate" required value={formData.config.signingCertificate || ""} onChange={(event) => setFormData((previous) => ({ ...previous, config: { ...previous.config, signingCertificate: event.target.value } }))} placeholder={t("settings.identityProvidersPage.signingCertificatePlaceholder")} /></div>
                )}

                <div className="flex items-center justify-between">
                  <Label htmlFor="enabled">{t("settings.identityProvidersPage.enabled")}</Label>
                  <Switch id="enabled" checked={formData.enabled} onCheckedChange={(checked: boolean) => setFormData((previous) => ({ ...previous, enabled: checked }))} />
                </div>
              </>
            )}

            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => { setShowDialog(false); setEditingProvider(null); resetForm(); }}>{t("common.buttons.cancel")}</Button>
              <Button type="submit" disabled={isSubmitting || (!selectedTemplate && !editingProvider) || !formData.alias.trim() || (!!template && !hasRequiredFields) || isDuplicateAlias}>
                {isSubmitting ? t("settings.identityProvidersPage.saving") : editingProvider ? t("settings.identityProvidersPage.saveChanges") : t("settings.identityProvidersPage.addProviderSubmit")}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
