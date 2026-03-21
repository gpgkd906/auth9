import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState } from "react";
import { getBrandMark } from "~/components/auth/AuthBrandPanel";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { hostedLoginApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";
import { getAccessToken } from "~/services/session.server";
import { API_BASE_URL, getHeaders, handleResponse } from "~/services/api/client";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.completeProfile.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return redirect("/login");
  }

  const url = new URL(request.url);
  const actionId = url.searchParams.get("action_id") || "";
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to default branding
  }

  return { actionId, branding };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return redirect("/login");
  }

  const formData = await request.formData();
  const displayName = (formData.get("displayName") as string || "").trim();
  const actionId = formData.get("actionId") as string;

  if (!displayName) {
    return { error: translate(locale, "auth.completeProfile.nameRequired") };
  }

  try {
    // Update user profile via API
    const response = await fetch(
      `${API_BASE_URL}/api/v1/users/me`,
      {
        method: "PATCH",
        headers: getHeaders(accessToken),
        body: JSON.stringify({ display_name: displayName }),
      }
    );
    await handleResponse(response);

    // Complete the pending action if we have an action ID
    if (actionId) {
      try {
        await hostedLoginApi.completeAction(actionId, accessToken);
      } catch {
        // Non-critical
      }
    }

    return redirect("/tenant/select");
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function CompleteProfilePage() {
  const { t } = useI18n();
  const loaderData = (useLoaderData<typeof loader>() ?? {}) as {
    actionId?: string;
    branding?: BrandingConfig;
  };
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData.branding ?? {}) };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [displayName, setDisplayName] = useState("");

  const isSubmitting = navigation.state === "submitting";

  return (
    <AuthPageShell
      branding={branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.completeProfile.panelTitle")}
      panelDescription={t("auth.completeProfile.panelDescription")}
    >
      <Card className="w-full max-w-md animate-fade-in-up">
        <CardHeader className="text-center">
          {branding.logo_url ? (
            <img
              src={branding.logo_url}
              alt={branding.company_name || "Auth9"}
              className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
              referrerPolicy="no-referrer"
              crossOrigin="anonymous"
            />
          ) : (
            <div className="logo-icon mx-auto mb-4">{getBrandMark(branding.company_name || "Auth9")}</div>
          )}
          <CardTitle className="text-2xl">{t("auth.completeProfile.title")}</CardTitle>
          <CardDescription className="auth-form-description">
            {t("auth.completeProfile.description")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="actionId" value={loaderData.actionId || ""} />

            <div className="space-y-2">
              <Label htmlFor="displayName">{t("common.labels.displayName")}</Label>
              <Input
                id="displayName"
                name="displayName"
                type="text"
                placeholder={t("auth.completeProfile.displayNamePlaceholder")}
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                required
                autoFocus
              />
            </div>

            {actionData?.error && (
              <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
                {actionData.error}
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("common.buttons.saving") : t("common.buttons.continue")}
            </Button>
          </Form>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
