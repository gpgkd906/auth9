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
import { hostedLoginApi, passwordApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.forceUpdatePassword.metaTitle");
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
  const newPassword = formData.get("password") as string;
  const confirmPassword = formData.get("confirmPassword") as string;
  const actionId = formData.get("actionId") as string;

  if (!newPassword) {
    return { error: translate(locale, "auth.forceUpdatePassword.passwordRequired") };
  }

  if (newPassword !== confirmPassword) {
    return { error: translate(locale, "auth.forceUpdatePassword.passwordMismatch") };
  }

  try {
    // Use the password change API (current password not required for force-update)
    await passwordApi.changePassword("", newPassword, accessToken);

    // Complete the pending action if we have an action ID
    if (actionId) {
      try {
        await hostedLoginApi.completeAction(actionId, accessToken);
      } catch {
        // Non-critical — continue even if action completion fails
      }
    }

    return redirect("/tenant/select");
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function ForceUpdatePasswordPage() {
  const { t } = useI18n();
  const loaderData = (useLoaderData<typeof loader>() ?? {}) as {
    actionId?: string;
    branding?: BrandingConfig;
  };
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData.branding ?? {}) };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  const isSubmitting = navigation.state === "submitting";

  return (
    <AuthPageShell
      branding={branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.forceUpdatePassword.panelTitle")}
      panelDescription={t("auth.forceUpdatePassword.panelDescription")}
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
          <CardTitle className="text-2xl">{t("auth.forceUpdatePassword.title")}</CardTitle>
          <CardDescription className="auth-form-description">
            {t("auth.forceUpdatePassword.description")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="actionId" value={loaderData.actionId || ""} />

            <div className="space-y-2">
              <Label htmlFor="password">{t("common.labels.newPassword")}</Label>
              <Input
                id="password"
                name="password"
                type="password"
                placeholder={t("common.placeholders.newPassword")}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                autoFocus
              />
              <p className="text-xs leading-5 text-[var(--text-secondary)]">
                {t("auth.forceUpdatePassword.passwordHint")}
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirmPassword">{t("common.labels.confirmPassword")}</Label>
              <Input
                id="confirmPassword"
                name="confirmPassword"
                type="password"
                placeholder={t("common.placeholders.confirmNewPassword")}
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                required
              />
            </div>

            {actionData?.error && (
              <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
                {actionData.error}
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("common.buttons.updating") : t("common.buttons.updatePassword")}
            </Button>
          </Form>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
