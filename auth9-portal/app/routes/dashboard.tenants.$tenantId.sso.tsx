import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "~/components/ui/select";
import { Switch } from "~/components/ui/switch";
import { getAccessToken } from "~/services/session.server";
import { tenantApi, tenantSsoApi } from "~/services/api";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { useState } from "react";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.sso.metaTitle", undefined, {
    tenantName: data?.tenant.name || translate(resolveMetaLocale(matches), "tenants.title"),
  });
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId) throw new Error(translate(locale, "tenants.errors.tenantIdRequired"));
  const accessToken = await getAccessToken(request);

  try {
    const [tenantRes, connectorsRes] = await Promise.all([
      tenantApi.get(tenantId, accessToken || undefined),
      tenantSsoApi.list(tenantId, accessToken || undefined),
    ]);

    const corePublicUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";

    return {
      tenant: tenantRes.data,
      connectors: connectorsRes.data,
      corePublicUrl,
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
      const providerType = String(formData.get("provider_type") || "saml") as "saml" | "oidc" | "ldap";
      const domains = String(formData.get("domains") || "")
        .split(",")
        .map((v) => v.trim())
        .filter(Boolean);

      const config: Record<string, string> = {};
      if (providerType === "saml") {
        config.entityId = String(formData.get("entity_id") || "").trim();
        config.singleSignOnServiceUrl = String(formData.get("sso_url") || "").trim();
        config.signingCertificate = String(formData.get("certificate") || "").trim();
      } else if (providerType === "oidc") {
        config.clientId = String(formData.get("client_id") || "").trim();
        config.clientSecret = String(formData.get("client_secret") || "").trim();
        config.authorizationUrl = String(formData.get("authorization_url") || "").trim();
        config.tokenUrl = String(formData.get("token_url") || "").trim();
        config.userInfoUrl = String(formData.get("userinfo_url") || "").trim();
      } else if (providerType === "ldap") {
        config.serverUrl = String(formData.get("server_url") || "").trim();
        config.bindDn = String(formData.get("bind_dn") || "").trim();
        config.bindPassword = String(formData.get("bind_password") || "").trim();
        config.baseDn = String(formData.get("base_dn") || "").trim();
        config.userSearchFilter = String(formData.get("user_search_filter") || "").trim();
        config.attrUsername = String(formData.get("attr_username") || "").trim();
        config.attrEmail = String(formData.get("attr_email") || "").trim();
        config.attrFirstName = String(formData.get("attr_first_name") || "").trim();
        config.attrLastName = String(formData.get("attr_last_name") || "").trim();
        const isAd = formData.get("is_active_directory") === "true";
        if (isAd) {
          config.isActiveDirectory = "true";
          const adDomain = String(formData.get("ad_domain") || "").trim();
          if (adDomain) config.adDomain = adDomain;
        }
        const useTls = formData.get("use_tls");
        if (useTls === "false") config.useTls = "false";
      }

      await tenantSsoApi.create(
        tenantId,
        {
          alias: String(formData.get("alias") || "").trim(),
          display_name: String(formData.get("display_name") || "").trim() || undefined,
          provider_type: providerType,
          enabled: formData.get("enabled") === "true",
          priority: Number(formData.get("priority") || "100"),
          domains,
          config,
        },
        accessToken || undefined
      );
      return { success: true, message: translate(locale, "tenants.sso.connectorCreated") };
    }

    if (intent === "delete") {
      const connectorId = String(formData.get("connector_id") || "");
      await tenantSsoApi.delete(tenantId, connectorId, accessToken || undefined);
      return { success: true, message: translate(locale, "tenants.sso.connectorDeleted") };
    }

    if (intent === "toggle") {
      const connectorId = String(formData.get("connector_id") || "");
      const enabled = formData.get("enabled") === "true";
      await tenantSsoApi.update(tenantId, connectorId, { enabled }, accessToken || undefined);
      return { success: true, message: translate(locale, "tenants.sso.connectorUpdated") };
    }

    if (intent === "test") {
      const connectorId = String(formData.get("connector_id") || "");
      const result = await tenantSsoApi.test(tenantId, connectorId, accessToken || undefined);
      return { success: result.data.ok, message: result.data.message };
    }
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }

  return { error: translate(locale, "tenants.webhooks.invalidAction") };
}

export default function TenantSsoPage() {
  const { t } = useI18n();
  const { tenant, connectors, corePublicUrl } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const [providerType, setProviderType] = useState<"saml" | "oidc" | "ldap">("saml");

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
            {t("tenants.sso.title", { tenantName: tenant.name })}
          </h1>
          <p className="text-sm text-[var(--text-secondary)]">{t("tenants.sso.description")}</p>
        </div>
      </div>

      {actionData?.error && <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{actionData.error}</div>}
      {actionData?.message && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">{actionData.message}</div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.sso.createTitle")}</CardTitle>
          <CardDescription>{t("tenants.sso.createDescription")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <input type="hidden" name="intent" value="create" />
            <input type="hidden" name="enabled" value="true" />
            <input type="hidden" name="provider_type" value={providerType} />
            <div className="space-y-2">
              <Label htmlFor="alias">{t("tenants.sso.alias")}</Label>
              <Input id="alias" name="alias" required placeholder={t("tenants.sso.aliasPlaceholder")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="display_name">{t("tenants.sso.displayName")}</Label>
              <Input id="display_name" name="display_name" placeholder={t("tenants.sso.displayNamePlaceholder")} />
            </div>
            <div className="space-y-2">
              <Label id="provider_type_label" htmlFor="provider_type_trigger">{t("tenants.sso.providerType")}</Label>
              <Select value={providerType} onValueChange={(value) => setProviderType(value as "saml" | "oidc" | "ldap")}>
                <SelectTrigger id="provider_type_trigger" aria-labelledby="provider_type_label">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="saml">SAML</SelectItem>
                  <SelectItem value="oidc">OIDC</SelectItem>
                  <SelectItem value="ldap">LDAP / Active Directory</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="priority">{t("tenants.sso.priority")}</Label>
              <Input id="priority" name="priority" defaultValue="100" />
            </div>
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="domains">{t("tenants.sso.domains")}</Label>
              <Input id="domains" name="domains" required placeholder={t("tenants.sso.domainsPlaceholder")} />
            </div>

            {providerType === "saml" && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="entity_id">{t("tenants.sso.samlEntityId")}</Label>
                  <Input id="entity_id" name="entity_id" placeholder={t("tenants.sso.samlEntityIdPlaceholder")} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="sso_url">{t("tenants.sso.samlSsoUrl")}</Label>
                  <Input id="sso_url" name="sso_url" placeholder={t("tenants.sso.samlSsoUrlPlaceholder")} />
                </div>
                <div className="space-y-2 md:col-span-2">
                  <Label htmlFor="certificate">{t("tenants.sso.samlCertificate")}</Label>
                  <Input id="certificate" name="certificate" placeholder={t("tenants.sso.samlCertificatePlaceholder")} />
                </div>
              </>
            )}
            {providerType === "oidc" && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="client_id">{t("tenants.sso.oidcClientId")}</Label>
                  <Input id="client_id" name="client_id" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="client_secret">{t("tenants.sso.oidcClientSecret")}</Label>
                  <Input id="client_secret" name="client_secret" type="password" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="authorization_url">{t("tenants.sso.oidcAuthorizationUrl")}</Label>
                  <Input id="authorization_url" name="authorization_url" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="token_url">{t("tenants.sso.oidcTokenUrl")}</Label>
                  <Input id="token_url" name="token_url" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="userinfo_url">{t("tenants.sso.oidcUserInfoUrl")}</Label>
                  <Input id="userinfo_url" name="userinfo_url" />
                </div>
              </>
            )}
            {providerType === "ldap" && (
              <>
                <div className="space-y-2">
                  <Label htmlFor="server_url">{t("tenants.sso.ldapServerUrl")}</Label>
                  <Input id="server_url" name="server_url" required placeholder={t("tenants.sso.ldapServerUrlPlaceholder")} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="bind_dn">{t("tenants.sso.ldapBindDn")}</Label>
                  <Input id="bind_dn" name="bind_dn" required placeholder={t("tenants.sso.ldapBindDnPlaceholder")} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="bind_password">{t("tenants.sso.ldapBindPassword")}</Label>
                  <Input id="bind_password" name="bind_password" type="password" required />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="base_dn">{t("tenants.sso.ldapBaseDn")}</Label>
                  <Input id="base_dn" name="base_dn" required placeholder={t("tenants.sso.ldapBaseDnPlaceholder")} />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="user_search_filter">{t("tenants.sso.ldapUserSearchFilter")}</Label>
                  {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
                  <Input id="user_search_filter" name="user_search_filter" placeholder="(uid={username})" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="attr_username">{t("tenants.sso.ldapAttrUsername")}</Label>
                  {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
                  <Input id="attr_username" name="attr_username" placeholder="uid" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="attr_email">{t("tenants.sso.ldapAttrEmail")}</Label>
                  {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
                  <Input id="attr_email" name="attr_email" placeholder="mail" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="attr_first_name">{t("tenants.sso.ldapAttrFirstName")}</Label>
                  {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
                  <Input id="attr_first_name" name="attr_first_name" placeholder="givenName" />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="attr_last_name">{t("tenants.sso.ldapAttrLastName")}</Label>
                  {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
                  <Input id="attr_last_name" name="attr_last_name" placeholder="sn" />
                </div>
                <div className="space-y-2 flex items-center gap-3">
                  <input type="hidden" name="is_active_directory" value="false" />
                  <Switch id="is_active_directory" name="is_active_directory" value="true" />
                  <Label htmlFor="is_active_directory">{t("tenants.sso.ldapIsActiveDirectory")}</Label>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="ad_domain">{t("tenants.sso.ldapAdDomain")}</Label>
                  {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
                  <Input id="ad_domain" name="ad_domain" placeholder="company.com" />
                </div>
              </>
            )}

            <div className="md:col-span-2">
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? t("tenants.actions.saving") : t("tenants.sso.createConnector")}
              </Button>
            </div>
          </Form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.sso.configuredTitle")}</CardTitle>
          <CardDescription>{t("tenants.sso.configuredDescription")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {connectors.length === 0 ? (
            <p className="text-sm text-[var(--text-secondary)]">{t("tenants.sso.noConnectors")}</p>
          ) : (
            connectors.map((connector) => (
              <div
                key={connector.id}
                className="border border-[var(--border-primary)] rounded-lg p-4 flex flex-col gap-3"
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="font-medium text-[var(--text-primary)]">{connector.display_name || connector.alias}</div>
                    <div className="text-sm text-[var(--text-secondary)]">
                      {connector.provider_type.toUpperCase()} • {connector.domains.join(", ")}
                    </div>
                  </div>
                  <Form method="post" className="flex items-center gap-3">
                    <input type="hidden" name="intent" value="toggle" />
                    <input type="hidden" name="connector_id" value={connector.id} />
                    <input type="hidden" name="enabled" value={String(!connector.enabled)} />
                    <Switch
                      checked={connector.enabled}
                      onCheckedChange={() => null}
                      onClick={(e) => {
                        const form = (e.currentTarget as HTMLElement).closest("form") as HTMLFormElement;
                        form?.requestSubmit();
                      }}
                    />
                  </Form>
                </div>
                <div className="flex items-center gap-2">
                  {connector.provider_type === "saml" && (
                    <a
                      href={`${corePublicUrl}/api/v1/enterprise-sso/saml/metadata/${connector.alias}`}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <Button type="button" variant="outline" size="sm">
                        SP Metadata
                      </Button>
                    </a>
                  )}
                  {connector.provider_type === "ldap" && (
                    <Link to={`/dashboard/tenants/${tenant.id}/sso/${connector.id}/ldap-mappings`}>
                      <Button type="button" variant="outline" size="sm">
                        {t("tenants.sso.ldapGroupMappings")}
                      </Button>
                    </Link>
                  )}
                  <Form method="post">
                    <input type="hidden" name="intent" value="test" />
                    <input type="hidden" name="connector_id" value={connector.id} />
                    <Button type="submit" variant="outline" size="sm">
                      {t("tenants.webhooks.test")}
                    </Button>
                  </Form>
                  <Form method="post">
                    <input type="hidden" name="intent" value="delete" />
                    <input type="hidden" name="connector_id" value={connector.id} />
                    <Button type="submit" variant="outline" size="sm" className="text-[var(--accent-red)] border-[var(--accent-red)]/40">
                      {t("common.buttons.delete")}
                    </Button>
                  </Form>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
