import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { ArrowLeftIcon, TrashIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { getAccessToken } from "~/services/session.server";
import { tenantSsoApi } from "~/services/api";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";

export const meta: MetaFunction<typeof loader> = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "tenants.sso.metaTitle");
};

export async function loader({ params, request }: LoaderFunctionArgs) {
  const { tenantId, connectorId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId || !connectorId)
    throw new Error(translate(locale, "tenants.errors.tenantIdRequired"));
  const accessToken = await getAccessToken(request);

  try {
    const mappingsRes = await tenantSsoApi.listLdapGroupMappings(
      tenantId,
      connectorId,
      accessToken || undefined
    );
    return { tenantId, connectorId, mappings: mappingsRes.data };
  } catch {
    throw redirect(`/dashboard/tenants/${tenantId}/sso`);
  }
}

export async function action({ request, params }: ActionFunctionArgs) {
  const { tenantId, connectorId } = params;
  const locale = await resolveLocale(request);
  if (!tenantId || !connectorId)
    return { error: translate(locale, "tenants.errors.tenantIdRequired") };
  const accessToken = await getAccessToken(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "create") {
      await tenantSsoApi.createLdapGroupMapping(
        tenantId,
        connectorId,
        {
          ldap_group_dn: String(formData.get("ldap_group_dn") || "").trim(),
          ldap_group_display_name:
            String(formData.get("ldap_group_display_name") || "").trim() || undefined,
          role_id: String(formData.get("role_id") || "").trim(),
        },
        accessToken || undefined
      );
      return { success: true, message: "Group mapping created." };
    }

    if (intent === "delete") {
      const mappingId = String(formData.get("mapping_id") || "");
      await tenantSsoApi.deleteLdapGroupMapping(
        tenantId,
        connectorId,
        mappingId,
        accessToken || undefined
      );
      return { success: true, message: "Group mapping deleted." };
    }
  } catch (error) {
    return { error: mapApiError(error, locale) };
  }

  return { error: "Invalid action" };
}

export default function LdapGroupMappingsPage() {
  const { t } = useI18n();
  const { tenantId, mappings } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4">
        <Button variant="ghost" size="icon" asChild>
          <Link
            to={`/dashboard/tenants/${tenantId}/sso`}
            aria-label={t("tenants.actions.backToList")}
          >
            <ArrowLeftIcon className="h-4 w-4" />
          </Link>
        </Button>
        <div>
          <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">
            {t("tenants.sso.ldapGroupMappingsTitle")}
          </h1>
          <p className="text-sm text-[var(--text-secondary)]">
            {t("tenants.sso.ldapGroupMappingsDescription")}
          </p>
        </div>
      </div>

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {actionData.error}
        </div>
      )}
      {actionData?.message && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {actionData.message}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.sso.ldapAddMapping")}</CardTitle>
          <CardDescription>
            {t("tenants.sso.ldapGroupMappingsDescription")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <input type="hidden" name="intent" value="create" />
            <div className="space-y-2">
              <Label htmlFor="ldap_group_dn">{t("tenants.sso.ldapGroupDn")}</Label>
              <Input
                id="ldap_group_dn"
                name="ldap_group_dn"
                required
                placeholder="cn=admins,ou=groups,dc=example,dc=com" // eslint-disable-line auth9-i18n/no-bare-ui-strings
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="ldap_group_display_name">{t("tenants.sso.ldapGroupDisplayName")}</Label>
              <Input
                id="ldap_group_display_name"
                name="ldap_group_display_name"
                placeholder="Administrators" // eslint-disable-line auth9-i18n/no-bare-ui-strings
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="role_id">{t("tenants.sso.ldapRoleId")}</Label>
              {/* eslint-disable-next-line auth9-i18n/no-bare-ui-strings */}
              <Input id="role_id" name="role_id" required placeholder="role UUID" />
            </div>
            <div className="md:col-span-3">
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? t("tenants.actions.saving") : t("tenants.sso.ldapAddMapping")}
              </Button>
            </div>
          </Form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("tenants.sso.ldapGroupMappings")}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {mappings.length === 0 ? (
            <p className="text-sm text-[var(--text-secondary)]">{t("tenants.sso.ldapNoMappings")}</p>
          ) : (
            mappings.map((mapping) => (
              <div
                key={mapping.id}
                className="border border-[var(--border-primary)] rounded-lg p-4 flex items-center justify-between"
              >
                <div>
                  <div className="font-medium text-[var(--text-primary)] text-sm font-mono">
                    {mapping.ldap_group_dn}
                  </div>
                  <div className="text-sm text-[var(--text-secondary)]">
                    {mapping.ldap_group_display_name
                      ? `${mapping.ldap_group_display_name} → `
                      : ""}
                    Role: {mapping.role_id}
                  </div>
                </div>
                <Form method="post">
                  <input type="hidden" name="intent" value="delete" />
                  <input type="hidden" name="mapping_id" value={mapping.id} />
                  <Button
                    type="submit"
                    variant="ghost"
                    size="icon"
                    className="text-[var(--accent-red)]"
                  >
                    <TrashIcon className="h-4 w-4" />
                  </Button>
                </Form>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}
