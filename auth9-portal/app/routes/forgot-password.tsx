import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { useLoaderData, useFetcher, Link } from "react-router";
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
import { passwordApi, publicBrandingApi, captchaApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";
import { Captcha, type CaptchaConfig, DEFAULT_CAPTCHA_CONFIG } from "~/components/captcha";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.forgotPassword.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";

  try {
    const { data } = await publicBrandingApi.get(clientId);
    let captchaConfig: CaptchaConfig = DEFAULT_CAPTCHA_CONFIG;
    try { captchaConfig = await captchaApi.getConfig(); } catch { /* ignore */ }
    return { branding: { ...DEFAULT_PUBLIC_BRANDING, ...data }, captchaConfig };
  } catch {
    void request;
    return { branding: DEFAULT_PUBLIC_BRANDING, captchaConfig: DEFAULT_CAPTCHA_CONFIG };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const email = formData.get("email") as string;

  if (!email) {
    return { error: translate(locale, "auth.forgotPassword.emailRequired") };
  }

  try {
    await passwordApi.forgotPassword(email);
    return { success: true };
  } catch {
    // Don't reveal whether email exists - always show success
    return { success: true };
  }
}

export default function ForgotPasswordPage() {
  const { t } = useI18n();
  const loaderData = (useLoaderData<typeof loader>() ?? {}) as { branding?: BrandingConfig; captchaConfig?: CaptchaConfig };
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData.branding ?? {}) };
  const captchaConfig = loaderData.captchaConfig ?? DEFAULT_CAPTCHA_CONFIG;
  const fetcher = useFetcher<typeof action>();
  const [email, setEmail] = useState("");
  const [captchaToken, setCaptchaToken] = useState("");

  const isSubmitting = fetcher.state === "submitting";

  if (fetcher.data && "success" in fetcher.data && fetcher.data.success) {
    return (
      <AuthPageShell
        branding={branding}
        panelEyebrow={t("auth.shared.hostedEyebrow")}
        panelTitle={t("auth.forgotPassword.panelTitle")}
        panelDescription={t("auth.forgotPassword.panelDescription")}
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
            <CardTitle className="text-2xl">{t("auth.forgotPassword.successTitle")}</CardTitle>
            <CardDescription className="auth-form-description">
              {t("auth.forgotPassword.successDescription", { email })}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-center text-sm text-[var(--text-secondary)]">
              {t("auth.forgotPassword.successHint")}{" "}
              <Link to="/forgot-password" className="font-medium text-[var(--accent-blue)] hover:underline">
                {t("auth.forgotPassword.tryAgain")}
              </Link>
              .
            </p>
            <div className="text-center">
              <Link to="/login">
                <Button variant="outline">{t("common.buttons.backToLogin")}</Button>
              </Link>
            </div>
          </CardContent>
        </Card>
      </AuthPageShell>
    );
  }

  return (
    <AuthPageShell
      branding={branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.forgotPassword.panelTitle")}
      panelDescription={t("auth.forgotPassword.panelDescription")}
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
          <CardTitle className="text-2xl">{t("auth.forgotPassword.title")}</CardTitle>
          <CardDescription className="auth-form-description">{t("auth.forgotPassword.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <fetcher.Form method="post" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="email">{t("common.labels.emailAddress")}</Label>
              <Input
                id="email"
                name="email"
                type="email"
                placeholder={t("common.placeholders.email")}
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
                autoFocus
              />
            </div>

            {fetcher.data && "error" in fetcher.data && fetcher.data.error && (
              <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
                {fetcher.data.error}
              </div>
            )}

            <input type="hidden" name="captchaToken" value={captchaToken} />
            {captchaConfig.enabled && captchaConfig.mode === "always" && (
              <Captcha config={captchaConfig} onVerify={setCaptchaToken} />
            )}
            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("common.buttons.sending") : t("common.buttons.sendResetLink")}
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
