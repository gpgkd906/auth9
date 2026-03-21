import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState } from "react";
import { ArrowLeftIcon, ChevronDownIcon, Cross2Icon, DownloadIcon, PlusIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { Switch } from "~/components/ui/switch";
import { getAccessToken } from "~/services/session.server";
import {
  tenantApi,
  samlApplicationApi,
  SAML_APPLICATION_API_BASE,
  VALID_ATTRIBUTE_SOURCES,
  type CreateSamlApplicationInput,
  type AttributeMapping,
} from "~/services/api";
import { CopyValue } from "~/components/services/copyable-value";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.samlApps.metaTitle", undefined, {
    tenantName: data?.tenant.name || translate(resolveMetaLocale(matches), "tenants.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) throw new Error(translate(locale, "tenants.errors.tenantIdRequired"));
  const accessToken = await getAccessToken(request);

  try {
    const [tenantRes, appsRes] = await Promise.all([
      tenantApi.get(tenantId, accessToken || undefined),
      samlApplicationApi.list(tenantId, accessToken || undefined),
    ]);

    // Fetch certificate info for each app (best-effort, don't fail if unavailable)
    const certInfoMap: Record<string, { expires_at: string; expires_soon: boolean; days_until_expiry: number } | null> = {};
    await Promise.all(
      appsRes.data.map(async (app: { id: string }) => {
        try {
          const info = await samlApplicationApi.getCertificateInfo(tenantId, app.id, accessToken || undefined);
          certInfoMap[app.id] = info.data;
        } catch {
          certInfoMap[app.id] = null;
        }
      })
    );

    return {
      tenant: tenantRes.data,
      apps: appsRes.data,
      certInfoMap,
      apiBaseUrl: SAML_APPLICATION_API_BASE,
    };
  } catch {
    throw redirect("/dashboard/tenants");
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) return { error: translate(locale, "tenants.errors.tenantIdRequired") };
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      const mappingCount = Number(formData.get("mapping_count") || "0");
      const attributeMappings: AttributeMapping[] = [];
      for (let i = 0; i < mappingCount; i++) {
        const source = String(formData.get(`mapping_source_${i}`) || "").trim();
        const samlAttribute = String(formData.get(`mapping_saml_attribute_${i}`) || "").trim();
        const friendlyName = String(formData.get(`mapping_friendly_name_${i}`) || "").trim();
        if (source && samlAttribute) {
          attributeMappings.push({
            source,
            saml_attribute: samlAttribute,
            ...(friendlyName ? { friendly_name: friendlyName } : {}),
          });
        }
      }

      const input: CreateSamlApplicationInput = {
        name: String(formData.get("name") || "").trim(),
        entity_id: String(formData.get("entity_id") || "").trim(),
        acs_url: String(formData.get("acs_url") || "").trim(),
        slo_url: String(formData.get("slo_url") || "").trim() || undefined,
        name_id_format: (String(formData.get("name_id_format") || "email")) as CreateSamlApplicationInput["name_id_format"],
        sign_assertions: formData.get("sign_assertions") === "true",
        sign_responses: formData.get("sign_responses") === "true",
        encrypt_assertions: formData.get("encrypt_assertions") === "true",
        sp_certificate: String(formData.get("sp_certificate") || "").trim() || undefined,
        attribute_mappings: attributeMappings,
      };

      await samlApplicationApi.create(tenantId, input, accessToken || undefined);
      return { success: true, message: translate(locale, "tenants.samlApps.appCreated") };
    }

    if (intent === "toggle") {
      const appId = String(formData.get("app_id") || "");
      const enabled = formData.get("enabled") === "true";
      await samlApplicationApi.update(tenantId, appId, { enabled }, accessToken || undefined);
      return { success: true, message: translate(locale, "tenants.samlApps.appUpdated") };
    }

    if (intent === "delete") {
      const appId = String(formData.get("app_id") || "");
      await samlApplicationApi.delete(tenantId, appId, accessToken || undefined);
      return { success: true, message: translate(locale, "tenants.samlApps.appDeleted") };
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }

  return { error: translate(locale, "tenants.errors.invalidIntent") };
}

function CertExpiryBadge({ certInfo }: { certInfo: { expires_soon: boolean; days_until_expiry: number } | null }) {
  const { t } = useI18n();
  if (!certInfo) return null;

  const { days_until_expiry, expires_soon } = certInfo;
  if (days_until_expiry <= 0) {
    return (
      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-[var(--accent-red)]/15 text-[var(--accent-red)]">
        {t("tenants.samlApps.certExpired")}
      </span>
    );
  }
  if (expires_soon) {
    return (
      <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-[var(--accent-amber)]/15 text-[var(--accent-amber)]">
        {t("tenants.samlApps.certExpiresSoon", { days: days_until_expiry })}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-[var(--accent-green)]/15 text-[var(--accent-green)]">
      {t("tenants.samlApps.certValid", { days: days_until_expiry })}
    </span>
  );
}

function SetupSection({ title, steps }: { title: string; steps: string[] }) {
  return (
    <div>
      <div className="font-medium text-[var(--text-primary)] mb-1">{title}</div>
      <ol className="list-decimal list-inside space-y-0.5 pl-1">
        {steps.map((step, i) => (
          <li key={i}>{step}</li>
        ))}
      </ol>
    </div>
  );
}

function SamlAppRow({
  app,
  tenantId,
  apiBaseUrl,
  certInfo,
}: {
  app: {
    id: string;
    tenant_id: string;
    name: string;
    entity_id: string;
    enabled: boolean;
    name_id_format: string;
    sign_assertions: boolean;
    sign_responses: boolean;
    attribute_mappings: AttributeMapping[];
    sso_url: string;
  };
  tenantId: string;
  apiBaseUrl: string;
  certInfo: { expires_at: string; expires_soon: boolean; days_until_expiry: number } | null;
}) {
  const { t } = useI18n();

  return (
    <div className="border border-[var(--border-primary)] rounded-lg p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <div>
          <div className="font-medium text-[var(--text-primary)]">{app.name}</div>
          <div className="text-sm text-[var(--text-secondary)]">{app.entity_id}</div>
        </div>
        <Form method="post" className="flex items-center gap-3">
          <input type="hidden" name="intent" value="toggle" />
          <input type="hidden" name="app_id" value={app.id} />
          <input type="hidden" name="enabled" value={String(!app.enabled)} />
          <Switch
            checked={app.enabled}
            onCheckedChange={() => null}
            onClick={(e) => {
              const form = (e.currentTarget as HTMLElement).closest("form") as HTMLFormElement;
              form?.requestSubmit();
            }}
          />
        </Form>
      </div>

      <div className="space-y-2">
        <div>
          <Label className="text-xs text-[var(--text-tertiary)]">{t("tenants.samlApps.metadataUrl")}</Label>
          <CopyValue
            value={`${apiBaseUrl}/api/v1/tenants/${tenantId}/saml-apps/${app.id}/metadata`}
            fieldId="metadata-url"
          />
        </div>
        <div>
          <Label className="text-xs text-[var(--text-tertiary)]">{t("tenants.samlApps.ssoUrl")}</Label>
          <CopyValue value={app.sso_url} fieldId="sso-url" />
        </div>
        <div className="flex items-center gap-3">
          <a
            href={`${apiBaseUrl}/api/v1/tenants/${tenantId}/saml-apps/${app.id}/certificate`}
            download="idp-signing.crt"
            className="inline-flex items-center gap-1 text-xs text-[var(--text-link)] hover:underline"
          >
            <DownloadIcon className="h-3 w-3" />
            {t("tenants.samlApps.downloadCertificate")}
          </a>
          <CertExpiryBadge certInfo={certInfo} />
        </div>
      </div>

      <div className="text-xs text-[var(--text-tertiary)]">
        NameID: {app.name_id_format} | Assertions: {app.sign_assertions ? "Signed" : "Unsigned"} | Responses: {app.sign_responses ? "Signed" : "Unsigned"} | Mappings: {app.attribute_mappings.length}
      </div>

      <details className="group">
        <summary className="flex items-center gap-1.5 cursor-pointer text-xs font-medium text-[var(--text-link)] hover:underline select-none">
          <ChevronDownIcon className="h-3 w-3 transition-transform group-open:rotate-180" />
          {t("tenants.samlApps.setupInstructions")}
        </summary>
        <div className="mt-3 space-y-3 text-xs text-[var(--text-secondary)]">
          <SetupSection title={t("tenants.samlApps.setupGenericTitle")} steps={[
            t("tenants.samlApps.setupGenericStep1"),
            t("tenants.samlApps.setupGenericStep2"),
            t("tenants.samlApps.setupGenericStep3"),
            t("tenants.samlApps.setupGenericStep4"),
          ]} />
          <SetupSection title={t("tenants.samlApps.setupSalesforceTitle")} steps={[
            t("tenants.samlApps.setupSalesforceStep1"),
            t("tenants.samlApps.setupSalesforceStep2"),
            t("tenants.samlApps.setupSalesforceStep3"),
            t("tenants.samlApps.setupSalesforceStep4"),
            t("tenants.samlApps.setupSalesforceStep5"),
          ]} />
          <SetupSection title={t("tenants.samlApps.setupAwsTitle")} steps={[
            t("tenants.samlApps.setupAwsStep1"),
            t("tenants.samlApps.setupAwsStep2"),
            t("tenants.samlApps.setupAwsStep3"),
            t("tenants.samlApps.setupAwsStep4"),
          ]} />
          <SetupSection title={t("tenants.samlApps.setupGoogleTitle")} steps={[
            t("tenants.samlApps.setupGoogleStep1"),
            t("tenants.samlApps.setupGoogleStep2"),
            t("tenants.samlApps.setupGoogleStep3"),
            t("tenants.samlApps.setupGoogleStep4"),
          ]} />
        </div>
      </details>

      <div className="flex items-center gap-2">
        <Form method="post">
          <input type="hidden" name="intent" value="delete" />
          <input type="hidden" name="app_id" value={app.id} />
          <Button type="submit" variant="outline" size="sm" className="text-[var(--accent-red)] border-[var(--accent-red)]/40">
            {t("common.buttons.delete")}
          </Button>
        </Form>
      </div>
    </div>
  );
}

export default function TenantSamlAppsPage() {
  const { t } = useI18n();
  const { tenant, apps, certInfoMap, apiBaseUrl } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  const [nameIdFormat, setNameIdFormat] = useState("email");
  const [signAssertions, setSignAssertions] = useState(true);
  const [signResponses, setSignResponses] = useState(true);
  const [encryptAssertions, setEncryptAssertions] = useState(false);
  const [mappingRows, setMappingRows] = useState<number[]>([]);
  const [mappingSources, setMappingSources] = useState<Record<number, string>>({});

  let nextRowId = mappingRows.length > 0 ? Math.max(...mappingRows) + 1 : 0;

  const addMappingRow = () => {
    setMappingRows((prev) => [...prev, nextRowId]);
    nextRowId++;
  };

  const removeMappingRow = (id: number) => {
    setMappingRows((prev) => prev.filter((r) => r !== id));
    setMappingSources((prev) => {
      const copy = { ...prev };
      delete copy[id];
      return copy;
    });
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link to={`/dashboard/tenants/${tenant.id}`} aria-label={t("tenants.actions.backToList")}>
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">
            {t("tenants.samlApps.title", { tenantName: tenant.name })}
          </h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("tenants.samlApps.description")}</p>
        </div>
      </div>

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{actionData.error}</div>
      )}
      {actionData?.message && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">{actionData.message}</div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.samlApps.createTitle")}</CardTitle>
          <CardDescription>{t("tenants.samlApps.createDescription")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <input type="hidden" name="intent" value="create" />
            <input type="hidden" name="name_id_format" value={nameIdFormat} />
            <input type="hidden" name="sign_assertions" value={String(signAssertions)} />
            <input type="hidden" name="sign_responses" value={String(signResponses)} />
            <input type="hidden" name="encrypt_assertions" value={String(encryptAssertions)} />
            <input type="hidden" name="mapping_count" value={mappingRows.length} />

            <div className="space-y-2">
              <Label htmlFor="name">{t("tenants.samlApps.name")}</Label>
              <Input id="name" name="name" required placeholder={t("tenants.samlApps.namePlaceholder")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="entity_id">{t("tenants.samlApps.entityId")}</Label>
              <Input id="entity_id" name="entity_id" required placeholder={t("tenants.samlApps.entityIdPlaceholder")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="acs_url">{t("tenants.samlApps.acsUrl")}</Label>
              <Input id="acs_url" name="acs_url" required type="url" placeholder={t("tenants.samlApps.acsUrlPlaceholder")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="slo_url">{t("tenants.samlApps.sloUrl")}</Label>
              <Input id="slo_url" name="slo_url" type="url" placeholder={t("tenants.samlApps.sloUrlPlaceholder")} />
            </div>

            <div className="space-y-2">
              <Label id="name_id_format_label">{t("tenants.samlApps.nameIdFormat")}</Label>
              <Select value={nameIdFormat} onValueChange={setNameIdFormat}>
                <SelectTrigger aria-labelledby="name_id_format_label">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="email">{t("tenants.samlApps.nameIdFormats.email")}</SelectItem>
                  <SelectItem value="persistent">{t("tenants.samlApps.nameIdFormats.persistent")}</SelectItem>
                  <SelectItem value="transient">{t("tenants.samlApps.nameIdFormats.transient")}</SelectItem>
                  <SelectItem value="unspecified">{t("tenants.samlApps.nameIdFormats.unspecified")}</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="flex flex-wrap gap-6 items-center">
              <label className="flex items-center gap-2 text-sm text-[var(--text-primary)] cursor-pointer">
                <Switch checked={signAssertions} onCheckedChange={setSignAssertions} />
                {t("tenants.samlApps.signAssertions")}
              </label>
              <label className="flex items-center gap-2 text-sm text-[var(--text-primary)] cursor-pointer">
                <Switch checked={signResponses} onCheckedChange={setSignResponses} />
                {t("tenants.samlApps.signResponses")}
              </label>
              <label className="flex items-center gap-2 text-sm text-[var(--text-primary)] cursor-pointer">
                <Switch checked={encryptAssertions} onCheckedChange={setEncryptAssertions} />
                {t("tenants.samlApps.encryptAssertions")}
              </label>
            </div>

            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="sp_certificate">
                {t("tenants.samlApps.spCertificate")}
                {encryptAssertions && <span className="text-[var(--accent-red)] ml-1">*</span>}
              </Label>
              <textarea
                id="sp_certificate"
                name="sp_certificate"
                rows={3}
                required={encryptAssertions}
                className={`w-full rounded-md border px-3 py-2 text-sm font-mono ${encryptAssertions ? "border-[var(--accent-amber)]" : "border-gray-300"}`}
                placeholder={t("tenants.samlApps.spCertificatePlaceholder")}
              />
              {encryptAssertions && (
                <p className="text-xs text-[var(--accent-amber)]">{t("tenants.samlApps.encryptionRequiresCert")}</p>
              )}
            </div>

            <div className="space-y-3 md:col-span-2">
              <Label>{t("tenants.samlApps.attributeMappings")}</Label>
              {mappingRows.map((rowId, index) => (
                <div key={rowId} className="flex gap-2 items-start">
                  <div className="flex-1">
                    <input type="hidden" name={`mapping_source_${index}`} value={mappingSources[rowId] || ""} />
                    <Select
                      value={mappingSources[rowId] || ""}
                      onValueChange={(value) => setMappingSources((prev) => ({ ...prev, [rowId]: value }))}
                    >
                      <SelectTrigger className="w-full">
                        <SelectValue placeholder={t("tenants.samlApps.attributeSource")} />
                      </SelectTrigger>
                      <SelectContent>
                        {VALID_ATTRIBUTE_SOURCES.map((s) => (
                          <SelectItem key={s} value={s}>{s}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {(mappingSources[rowId] === "tenant_roles" || mappingSources[rowId] === "tenant_permissions") && (
                      <p className="text-[11px] text-[var(--accent-amber)] mt-1">{t("tenants.samlApps.advancedSourceHint")}</p>
                    )}
                  </div>
                  <div className="flex-[2]">
                    <Input
                      name={`mapping_saml_attribute_${index}`}
                      placeholder={t("tenants.samlApps.samlAttribute")}
                    />
                  </div>
                  <div className="flex-1">
                    <Input
                      name={`mapping_friendly_name_${index}`}
                      placeholder={t("tenants.samlApps.friendlyName")}
                    />
                  </div>
                  <Button type="button" variant="ghost" size="icon" onClick={() => removeMappingRow(rowId)}>
                    <Cross2Icon className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              <Button type="button" variant="outline" size="sm" onClick={addMappingRow}>
                <PlusIcon className="h-4 w-4 mr-1" />
                {t("tenants.samlApps.addMapping")}
              </Button>
            </div>

            <div className="md:col-span-2">
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? t("tenants.actions.saving") : t("tenants.samlApps.createApp")}
              </Button>
            </div>
          </Form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.samlApps.configuredTitle")}</CardTitle>
          <CardDescription>{t("tenants.samlApps.configuredDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {apps.length === 0 ? (
            <p className="text-sm text-[var(--text-secondary)]">{t("tenants.samlApps.noApps")}</p>
          ) : (
            apps.map((app) => (
              <SamlAppRow
                key={app.id}
                app={app}
                tenantId={tenant.id}
                apiBaseUrl={apiBaseUrl}
                certInfo={certInfoMap[app.id] || null}
              />
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
