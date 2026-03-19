import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { redirect, Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState, useCallback, useRef } from "react";
import QRCode from "qrcode";
import { getBrandMark } from "~/components/auth/AuthBrandPanel";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { OtpInput } from "~/components/ui/otp-input";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { resolveLocale } from "~/services/locale.server";
import { getAccessToken } from "~/services/session.server";
import { hostedLoginApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.mfaSetup.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return redirect("/login");
  }

  const url = new URL(request.url);
  const actionId = url.searchParams.get("action_id") || "";
  const loginChallenge = url.searchParams.get("login_challenge") || "";

  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to default branding
  }

  // Start TOTP enrollment
  const enrollment = await hostedLoginApi.totpEnrollStart(accessToken);
  const qrDataUrl = await QRCode.toDataURL(enrollment.otpauth_uri, {
    width: 200,
    margin: 2,
    color: { dark: "#1D1D1F", light: "#FFFFFF" },
  });

  return {
    branding,
    actionId,
    loginChallenge,
    setupToken: enrollment.setup_token,
    secret: enrollment.secret,
    qrDataUrl,
  };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return redirect("/login");
  }

  const formData = await request.formData();
  const code = String(formData.get("code") || "").trim();
  const setupToken = String(formData.get("setup_token") || "");
  const actionId = String(formData.get("action_id") || "");
  const loginChallenge = String(formData.get("login_challenge") || "");

  if (!code) {
    return { error: translate(locale, "auth.mfaVerify.codeRequired") };
  }

  try {
    await hostedLoginApi.totpEnrollVerify(setupToken, code, accessToken);

    // Complete the pending action if present
    if (actionId) {
      try {
        await hostedLoginApi.completeAction(actionId, accessToken);
      } catch {
        // Non-critical — enrollment succeeded even if action completion fails
      }
    }

    // Complete OIDC authorization flow if login_challenge is present
    if (loginChallenge) {
      try {
        const authResult = await hostedLoginApi.authorizeComplete(loginChallenge, accessToken);
        return redirect(authResult.redirect_url);
      } catch {
        // Fall through to tenant select
      }
    }

    return redirect("/tenant/select");
  } catch (error) {
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function MfaSetupTotpPage() {
  const { t } = useI18n();
  const data = useLoaderData<typeof loader>();
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(data?.branding ?? {}) };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  const [showManual, setShowManual] = useState(false);
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
      panelTitle={t("auth.mfaSetup.panelTitle")}
      panelDescription={t("auth.mfaSetup.panelDescription")}
    >
      <Card className="w-full max-w-md animate-fade-in-up">
        <CardHeader className="text-center">
          {branding.logo_url ? (
            <img
              src={branding.logo_url}
              alt={branding.company_name || "Auth9"}
              className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
              referrerPolicy="no-referrer"
            />
          ) : (
            <div className="logo-icon mx-auto mb-4">{getBrandMark(branding.company_name || "Auth9")}</div>
          )}
          <CardTitle className="text-2xl">{t("auth.mfaSetup.title")}</CardTitle>
          <CardDescription className="auth-form-description">{t("auth.mfaSetup.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          {/* QR Code */}
          <div className="flex justify-center">
            <div className="rounded-2xl border border-[var(--glass-border-subtle)] bg-white p-3">
              <img
                src={data?.qrDataUrl}
                alt={t("auth.mfaSetup.qrAlt")}
                width={200}
                height={200}
              />
            </div>
          </div>

          {/* Manual Entry Toggle */}
          <div className="text-center">
            <button
              type="button"
              className="text-sm font-medium text-[var(--accent-blue)] hover:underline"
              onClick={() => setShowManual(!showManual)}
            >
              {t("auth.mfaSetup.manualEntryToggle")}
            </button>
          </div>

          {showManual && (
            <div className="rounded-2xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] p-4">
              <p className="text-xs font-medium uppercase tracking-wider text-[var(--text-tertiary)] mb-2">
                {t("auth.mfaSetup.manualEntryLabel")}
              </p>
              <code className="block break-all text-sm font-mono text-[var(--text-primary)] select-all">
                {data?.secret}
              </code>
            </div>
          )}

          {/* Verify Code */}
          <div className="space-y-2">
            <p className="text-sm text-center text-[var(--text-secondary)]">
              {t("auth.mfaSetup.verifyDescription")}
            </p>
            <Form method="post" ref={formRef} className="space-y-4">
              <input type="hidden" name="setup_token" value={data?.setupToken ?? ""} />
              <input type="hidden" name="action_id" value={data?.actionId ?? ""} />
              <input type="hidden" name="login_challenge" value={data?.loginChallenge ?? ""} />
              <input type="hidden" name="code" value="" />

              <OtpInput
                onComplete={handleOtpComplete}
                disabled={isSubmitting}
                error={!!actionData?.error}
              />

              {actionData?.error ? (
                <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
                  {actionData.error}
                </div>
              ) : null}
            </Form>
          </div>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
