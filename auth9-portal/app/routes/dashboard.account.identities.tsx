import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { redirect } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { identityProviderApi, type LinkedIdentity } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { Cross2Icon } from "@radix-ui/react-icons";

type AvailableIdentityProvider = {
  alias: string;
  provider_id: string;
  display_name?: string;
};

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  try {
    const response = await identityProviderApi.listMyLinkedIdentities(accessToken);
    const linkedAliases = new Set(response.data.map((identity) => identity.provider_alias));

    let availableProviders: AvailableIdentityProvider[] = [];
    try {
      const providers = await identityProviderApi.list(accessToken);
      availableProviders = providers.data
        .filter((provider) => provider.enabled && !linkedAliases.has(provider.alias))
        .map((provider) => ({
          alias: provider.alias,
          provider_id: provider.provider_id,
          display_name: provider.display_name,
        }));
    } catch {
      // Keep identity management usable even if provider discovery fails.
    }

    return { identities: response.data, availableProviders };
  } catch {
    return {
      identities: [],
      availableProviders: [],
      error: translate(await resolveLocale(request), "accountIdentities.loadError"),
    };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return { error: translate(await resolveLocale(request), "accountIdentities.notAuthenticated") };
  }

  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "link") {
      const providerAlias = String(formData.get("providerAlias") || "").trim();
      if (!providerAlias) {
        return { error: translate(await resolveLocale(request), "accountIdentities.providerAliasRequired") };
      }

      const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
      const linkUrl = `${apiBaseUrl}/api/v1/social-login/link/${encodeURIComponent(providerAlias)}`;

      // Server-side fetch with Bearer token to get redirect URL from Auth9 social broker
      const linkResponse = await fetch(linkUrl, {
        headers: { Authorization: `Bearer ${accessToken}` },
        redirect: "manual",
      });

      const redirectUrl = linkResponse.headers.get("Location");
      if (!redirectUrl) {
        return { error: translate(await resolveLocale(request), "accountIdentities.linkFailed") };
      }

      return redirect(redirectUrl);
    }

    if (intent === "unlink") {
      const identityId = formData.get("identityId") as string;
      await identityProviderApi.unlinkIdentity(identityId, accessToken);
      return { success: true, message: translate(await resolveLocale(request), "accountIdentities.unlinkSuccess") };
    }
  } catch (error) {
    const locale = await resolveLocale(request);
    const message = mapApiError(error, locale);
    return { error: message };
  }

  return { error: translate(await resolveLocale(request), "accountIdentities.invalidAction") };
}

function getProviderIcon(providerType: string) {
  switch (providerType.toLowerCase()) {
    case "google":
      return "G";
    case "github":
      return "GH";
    case "microsoft":
      return "MS";
    case "apple":
      return "AP";
    case "facebook":
      return "FB";
    default:
      return providerType.slice(0, 2).toUpperCase();
  }
}

function getProviderName(providerAlias: string, providerType: string) {
  const name = providerAlias || providerType;
  return name.charAt(0).toUpperCase() + name.slice(1);
}

export default function AccountIdentitiesPage() {
  const { t } = useI18n();
  const formatters = useFormatters();
  const { identities, availableProviders, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{t("accountIdentities.title")}</CardTitle>
          <CardDescription>
            {t("accountIdentities.description")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {availableProviders.length > 0 && (
            <div className="mb-6 rounded-lg border border-[var(--border-subtle)] bg-[var(--surface-elevated)] p-4">
              <div className="mb-3">
                <h3 className="font-medium text-[var(--text-primary)]">{t("accountIdentities.linkAnother")}</h3>
                <p className="text-sm text-[var(--text-secondary)]">
                  {t("accountIdentities.linkAnotherDescription")}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                {availableProviders.map((provider) => (
                  <Form method="post" key={provider.alias}>
                    <input type="hidden" name="intent" value="link" />
                    <input type="hidden" name="providerAlias" value={provider.alias} />
                    <Button type="submit" variant="outline" size="sm" disabled={isSubmitting}>
                      {t("accountIdentities.linkAction", {
                        provider: getProviderName(provider.display_name || provider.alias, provider.provider_id),
                      })}
                    </Button>
                  </Form>
                ))}
              </div>
            </div>
          )}

          {loadError && (
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md mb-4">
              {loadError}
            </div>
          )}

          {actionData?.error && (
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md mb-4">
              {actionData.error}
            </div>
          )}

          {actionData?.success && (
            <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md mb-4">
              {actionData.message}
            </div>
          )}

          {identities.length === 0 ? (
            <div className="text-center py-12">
              <div className="mx-auto w-12 h-12 bg-[var(--sidebar-item-hover)] rounded-full flex items-center justify-center mb-4">
                <LinkIcon className="h-6 w-6 text-[var(--text-tertiary)]" />
              </div>
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">
                {t("accountIdentities.noIdentities")}
              </h3>
              <p className="text-[var(--text-secondary)]">
                {availableProviders.length > 0
                  ? t("accountIdentities.noIdentitiesWithProviders")
                  : t("accountIdentities.noIdentitiesWithoutProviders")}
              </p>
            </div>
          ) : (
            <div className="divide-y">
              {identities.map((identity: LinkedIdentity) => (
                <div
                  key={identity.id}
                  className="flex items-center gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div className="w-10 h-10 bg-[var(--sidebar-item-hover)] rounded-full flex items-center justify-center text-sm font-semibold text-[var(--text-secondary)]">
                    {getProviderIcon(identity.provider_type)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">
                      {getProviderName(identity.provider_alias, identity.provider_type)}
                    </div>
                    <div className="text-sm text-[var(--text-secondary)] mt-0.5">
                      {identity.external_email || identity.external_user_id}
                      <span className="text-xs text-[var(--text-tertiary)] ml-2">
                        {t("accountIdentities.linkedOn", { date: formatters.date(identity.linked_at) })}
                      </span>
                    </div>
                  </div>
                  <Form method="post">
                    <input type="hidden" name="intent" value="unlink" />
                    <input type="hidden" name="identityId" value={identity.id} />
                    <Button
                      type="submit"
                      variant="ghost"
                      size="sm"
                      className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                      disabled={isSubmitting}
                    >
                      <Cross2Icon className="h-4 w-4 mr-1" />
                      {t("accountIdentities.unlink")}
                    </Button>
                  </Form>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function LinkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
    </svg>
  );
}
