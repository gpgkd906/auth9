import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { redirect, Form, Link, useActionData, useLoaderData, useNavigation, useSearchParams } from "react-router";
import { useState, useCallback, useRef } from "react";
import { getBrandMark } from "~/components/auth/AuthBrandPanel";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { OtpInput } from "~/components/ui/otp-input";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { resolveLocale } from "~/services/locale.server";
import { commitSession } from "~/services/session.server";
import { hostedLoginApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.mfaVerify.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const url = new URL(request.url);
  const mfaSessionToken = url.searchParams.get("mfa_session_token");

  if (!mfaSessionToken) {
    return redirect("/login");
  }

  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to default Portal branding
  }

  return {
    locale,
    branding,
    mfaSessionToken,
    mfaMethods: url.searchParams.get("mfa_methods")?.split(",") ?? ["totp"],
    loginChallenge: url.searchParams.get("login_challenge"),
  };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = String(formData.get("intent") || "verify-totp");
  const code = String(formData.get("code") || "").trim();
  const mfaSessionToken = String(formData.get("mfa_session_token") || "");
  const loginChallenge = formData.get("login_challenge") as string | null;

  if (!code) {
    return { error: translate(locale, "auth.mfaVerify.codeRequired") };
  }

  if (!mfaSessionToken) {
    return { error: translate(locale, "auth.mfaVerify.sessionExpired") };
  }

  try {
    const result =
      intent === "verify-recovery"
        ? await hostedLoginApi.challengeRecoveryCode(mfaSessionToken, code)
        : await hostedLoginApi.challengeTotp(mfaSessionToken, code);

    const session = {
      accessToken: result.access_token,
      identityAccessToken: result.access_token,
      refreshToken: undefined,
      idToken: undefined,
      expiresAt: Date.now() + result.expires_in * 1000,
      identityExpiresAt: Date.now() + result.expires_in * 1000,
    };
    const cookie = await commitSession(session);

    // Complete OIDC authorization flow if login_challenge is present
    if (loginChallenge && result.access_token) {
      try {
        const authResult = await hostedLoginApi.authorizeComplete(loginChallenge, result.access_token);
        return redirect(authResult.redirect_url);
      } catch {
        // Fall through to tenant select
      }
    }

    return redirect("/tenant/select", { headers: { "Set-Cookie": cookie } });
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function MfaVerifyPage() {
  const { t } = useI18n();
  const loaderData = useLoaderData<typeof loader>();
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData?.branding ?? {}) };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const [searchParams] = useSearchParams();

  const mfaSessionToken = loaderData?.mfaSessionToken ?? searchParams.get("mfa_session_token") ?? "";
  const loginChallenge = loaderData?.loginChallenge ?? null;

  const [mode, setMode] = useState<"totp" | "recovery">("totp");
  const formRef = useRef<HTMLFormElement>(null);

  const handleOtpComplete = useCallback(
    (code: string) => {
      if (!formRef.current) return;
      const codeInput = formRef.current.querySelector<HTMLInputElement>('input[name="code"]');
      if (codeInput) {
        codeInput.value = code;
        formRef.current.requestSubmit();
      }
    },
    []
  );

  return (
    <AuthPageShell
      branding={branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.mfaVerify.panelTitle")}
      panelDescription={t("auth.mfaVerify.panelDescription")}
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
          <CardTitle className="text-2xl">{t("auth.mfaVerify.title")}</CardTitle>
          <CardDescription className="auth-form-description">
            {mode === "totp" ? t("auth.mfaVerify.totpDescription") : t("auth.mfaVerify.recoveryDescription")}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Form method="post" ref={formRef} className="space-y-4">
            <input type="hidden" name="intent" value={mode === "totp" ? "verify-totp" : "verify-recovery"} />
            <input type="hidden" name="mfa_session_token" value={mfaSessionToken} />
            {loginChallenge && <input type="hidden" name="login_challenge" value={loginChallenge} />}

            {mode === "totp" ? (
              <>
                <input type="hidden" name="code" value="" />
                <OtpInput
                  onComplete={handleOtpComplete}
                  disabled={isSubmitting}
                  error={!!actionData?.error}
                />
              </>
            ) : (
              <Input
                name="code"
                placeholder={t("auth.mfaVerify.recoveryPlaceholder")}
                autoComplete="off"
                disabled={isSubmitting}
              />
            )}

            {actionData?.error ? (
              <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
                {actionData.error}
              </div>
            ) : null}

            {mode === "recovery" && (
              <Button type="submit" className="w-full" disabled={isSubmitting}>
                {isSubmitting ? t("auth.mfaVerify.verifying") : t("auth.mfaVerify.submit")}
              </Button>
            )}
          </Form>

          <div className="text-center">
              <button
                type="button"
                className="text-sm font-medium text-[var(--accent-blue)] hover:underline"
                onClick={() => setMode(mode === "totp" ? "recovery" : "totp")}
              >
                {mode === "totp" ? t("auth.mfaVerify.switchToRecovery") : t("auth.mfaVerify.switchToTotp")}
              </button>
          </div>

          <div className="text-center text-sm">
            <Link to="/login" className="font-medium text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:underline">
              {t("common.buttons.backToLogin")}
            </Link>
          </div>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
