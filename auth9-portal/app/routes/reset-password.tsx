import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { useActionData, useLoaderData, useNavigation, Link, useFetcher } from "react-router";
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
import { passwordApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.resetPassword.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const url = new URL(request.url);
  const token = url.searchParams.get("token");
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to static branding when the public config is unavailable.
  }

  if (!token) {
    return { error: translate(locale, "auth.resetPassword.invalidToken"), branding };
  }

  return { token, branding };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const token = formData.get("token") as string;
  const password = formData.get("password") as string;
  const confirmPassword = formData.get("confirmPassword") as string;

  if (!token) {
    return { error: translate(locale, "auth.resetPassword.invalidToken") };
  }

  if (!password) {
    return { error: translate(locale, "auth.resetPassword.passwordRequired") };
  }

  if (password !== confirmPassword) {
    return { error: translate(locale, "auth.resetPassword.passwordMismatch") };
  }

  try {
    await passwordApi.resetPassword(token, password);
    return { success: true };
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function ResetPasswordPage() {
  const { t } = useI18n();
  const rawLoaderData = (useLoaderData<typeof loader>() ?? {}) as {
    token?: string;
    error?: string;
    branding?: BrandingConfig;
  };
  const loaderData = {
    ...rawLoaderData,
    branding: { ...DEFAULT_PUBLIC_BRANDING, ...(rawLoaderData.branding ?? {}) },
  };
  const fetcher = useFetcher<typeof action>();
  const routeActionData = useActionData<typeof action>();
  const actionData = fetcher.data ?? routeActionData;
  const navigation = useNavigation();
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  const isSubmitting = fetcher.state === "submitting" || navigation.state === "submitting";

  // Show error if no token
  if ("error" in loaderData) {
    return (
      <AuthPageShell
        branding={loaderData.branding}
        panelEyebrow={t("auth.shared.hostedEyebrow")}
        panelTitle={t("auth.resetPassword.panelTitle")}
        panelDescription={t("auth.resetPassword.panelDescription")}
      >
        <Card className="w-full max-w-md animate-fade-in-up">
          <CardHeader className="text-center">
            {loaderData.branding.logo_url ? (
              <img
                src={loaderData.branding.logo_url}
                alt={loaderData.branding.company_name || "Auth9"}
                className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
                referrerPolicy="no-referrer"
                crossOrigin="anonymous"
              />
            ) : (
              <div className="logo-icon mx-auto mb-4">{getBrandMark(loaderData.branding.company_name || "Auth9")}</div>
            )}
            <CardTitle className="text-2xl">{t("auth.resetPassword.invalidTitle")}</CardTitle>
            <CardDescription className="auth-form-description">{loaderData.error}</CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Link to="/forgot-password">
              <Button>{t("common.buttons.requestNewResetLink")}</Button>
            </Link>
          </CardContent>
        </Card>
      </AuthPageShell>
    );
  }

  // Show success message
  if (actionData?.success) {
    return (
      <AuthPageShell
        branding={loaderData.branding}
        panelEyebrow={t("auth.shared.hostedEyebrow")}
        panelTitle={t("auth.resetPassword.panelTitle")}
        panelDescription={t("auth.resetPassword.panelDescription")}
      >
        <Card className="w-full max-w-md animate-fade-in-up">
          <CardHeader className="text-center">
            {loaderData.branding.logo_url ? (
              <img
                src={loaderData.branding.logo_url}
                alt={loaderData.branding.company_name || "Auth9"}
                className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
                referrerPolicy="no-referrer"
                crossOrigin="anonymous"
              />
            ) : (
              <div className="logo-icon mx-auto mb-4">{getBrandMark(loaderData.branding.company_name || "Auth9")}</div>
            )}
            <CardTitle className="text-2xl">{t("auth.resetPassword.successTitle")}</CardTitle>
            <CardDescription className="auth-form-description">
              {t("auth.resetPassword.successDescription")}
            </CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Link to="/login">
              <Button>{t("common.buttons.signIn")}</Button>
            </Link>
          </CardContent>
        </Card>
      </AuthPageShell>
    );
  }

  return (
    <AuthPageShell
      branding={loaderData.branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.resetPassword.panelTitle")}
      panelDescription={t("auth.resetPassword.panelDescription")}
    >
      <Card className="w-full max-w-md animate-fade-in-up">
        <CardHeader className="text-center">
          {loaderData.branding.logo_url ? (
            <img
              src={loaderData.branding.logo_url}
              alt={loaderData.branding.company_name || "Auth9"}
              className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
              referrerPolicy="no-referrer"
              crossOrigin="anonymous"
            />
          ) : (
            <div className="logo-icon mx-auto mb-4">{getBrandMark(loaderData.branding.company_name || "Auth9")}</div>
          )}
          <CardTitle className="text-2xl">{t("auth.resetPassword.title")}</CardTitle>
          <CardDescription className="auth-form-description">{t("auth.resetPassword.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <fetcher.Form method="post" className="space-y-4">
            <input type="hidden" name="token" value={loaderData.token} />

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
              <p className="text-xs leading-5 text-[var(--text-secondary)]">{t("auth.resetPassword.passwordHint")}</p>
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
              {isSubmitting ? t("common.buttons.resetting") : t("common.buttons.resetPassword")}
            </Button>

            <div className="text-center text-sm">
              <Link to="/login" className="font-medium text-[var(--accent-blue)] hover:underline">
                {t("common.buttons.backToLogin")}
              </Link>
            </div>
          </fetcher.Form>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
