import type { MetaFunction, ActionFunctionArgs } from "react-router";
import { Form, Link, useActionData, useNavigation } from "react-router";
import { useState, useEffect, useRef } from "react";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { LanguageSwitcher } from "~/components/LanguageSwitcher";
import { ThemeToggle } from "~/components/ThemeToggle";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { useI18n } from "~/i18n";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { resolveLocale } from "~/services/locale.server";
import { commitSession } from "~/services/session.server";
import { emailOtpApi } from "~/services/api";
import { redirect } from "react-router";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.emailOtp.metaTitle");
};

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = formData.get("intent") as string;

  if (intent === "send-code" || intent === "resend-code") {
    const email = String(formData.get("email") || "").trim();
    if (!email) {
      return { error: translate(locale, "auth.emailOtp.emailRequired"), phase: "email" as const };
    }

    try {
      await emailOtpApi.send(email);
      return { phase: "verify" as const, email, error: null };
    } catch (error) {
      const message = mapApiError(error, locale);
      return { error: message, phase: "email" as const };
    }
  }

  if (intent === "verify-code") {
    const email = String(formData.get("email") || "").trim();
    const code = String(formData.get("code") || "").trim();

    if (!email || !code) {
      return { error: translate(locale, "auth.emailOtp.codeRequired"), phase: "verify" as const, email };
    }

    try {
      const result = await emailOtpApi.verify(email, code);
      const session = {
        accessToken: result.access_token,
        identityAccessToken: result.access_token,
        refreshToken: undefined,
        idToken: undefined,
        expiresAt: Date.now() + result.expires_in * 1000,
        identityExpiresAt: Date.now() + result.expires_in * 1000,
      };

      return redirect("/tenant/select", {
        headers: {
          "Set-Cookie": await commitSession(session),
        },
      });
    } catch (error) {
      const message = mapApiError(error, locale);
      return { error: message, phase: "verify" as const, email };
    }
  }

  return { error: translate(locale, "auth.login.invalidAction"), phase: "email" as const };
}

const COOLDOWN_SECS = 60;

export default function EmailOtpLogin() {
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const { t } = useI18n();

  const phase = actionData?.phase || "email";
  const email = actionData?.email || "";

  const [cooldown, setCooldown] = useState(0);
  const timerRef = useRef<ReturnType<typeof setInterval>>(null);
  const codeInputRef = useRef<HTMLInputElement>(null);

  // Start cooldown when entering verify phase
  useEffect(() => {
    if (phase === "verify" && !actionData?.error) {
      setCooldown(COOLDOWN_SECS);
    }
  }, [phase, actionData?.error]);

  // Countdown timer
  useEffect(() => {
    if (cooldown > 0) {
      timerRef.current = setInterval(() => {
        setCooldown((prev) => {
          if (prev <= 1) {
            if (timerRef.current) clearInterval(timerRef.current);
            return 0;
          }
          return prev - 1;
        });
      }, 1000);
      return () => {
        if (timerRef.current) clearInterval(timerRef.current);
      };
    }
  }, [cooldown]);

  // Auto-focus code input when entering verify phase
  useEffect(() => {
    if (phase === "verify" && codeInputRef.current) {
      codeInputRef.current.focus();
    }
  }, [phase]);

  return (
    <>
      <div className="fixed top-6 right-6 z-20 flex items-center gap-3">
        <LanguageSwitcher />
        <ThemeToggle />
      </div>

      <div className="min-h-screen flex items-center justify-center px-6 relative">
        <div className="page-backdrop" />

        <Card className="w-full max-w-md relative z-10 animate-fade-in-up">
          <CardHeader className="text-center">
            <div className="logo-icon mx-auto mb-4">A9</div>
            <CardTitle className="text-2xl">
              {phase === "verify" ? t("auth.emailOtp.verifyTitle") : t("auth.emailOtp.title")}
            </CardTitle>
            <CardDescription>
              {phase === "verify"
                ? t("auth.emailOtp.verifyDescription")
                : t("auth.emailOtp.description")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {phase === "email" ? (
                <Form method="post">
                  <input type="hidden" name="intent" value="send-code" />
                  <Input
                    type="email"
                    name="email"
                    required
                    placeholder={t("auth.emailOtp.emailPlaceholder")}
                    className="mb-3"
                    autoFocus
                  />
                  <Button type="submit" className="w-full" disabled={isSubmitting}>
                    {isSubmitting ? t("auth.emailOtp.sending") : t("auth.emailOtp.sendCode")}
                  </Button>
                </Form>
              ) : (
                <>
                  <p className="text-sm text-[var(--text-secondary)] text-center">
                    {email}
                  </p>
                  <Form method="post">
                    <input type="hidden" name="intent" value="verify-code" />
                    <input type="hidden" name="email" value={email} />
                    <Input
                      ref={codeInputRef}
                      type="text"
                      name="code"
                      required
                      maxLength={6}
                      pattern="[0-9]{6}"
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      placeholder={t("auth.emailOtp.codePlaceholder")}
                      className="mb-3 text-center text-2xl tracking-[0.5em] font-mono"
                    />
                    <Button type="submit" className="w-full" disabled={isSubmitting}>
                      {isSubmitting ? t("auth.emailOtp.verifying") : t("auth.emailOtp.verify")}
                    </Button>
                  </Form>

                  {/* Resend button */}
                  <Form method="post" className="text-center">
                    <input type="hidden" name="intent" value="resend-code" />
                    <input type="hidden" name="email" value={email} />
                    <Button
                      type="submit"
                      variant="ghost"
                      size="sm"
                      disabled={cooldown > 0 || isSubmitting}
                      className="text-sm"
                    >
                      {cooldown > 0
                        ? `${t("auth.emailOtp.resendIn")} ${cooldown}s`
                        : t("auth.emailOtp.resendCode")}
                    </Button>
                  </Form>

                  {/* Back to email input */}
                  <div className="text-center">
                    <Link
                      to="/auth/email-otp"
                      className="text-sm text-[var(--text-tertiary)] hover:text-[var(--text-primary)] underline-offset-4 hover:underline"
                    >
                      {t("auth.emailOtp.backToEmail")}
                    </Link>
                  </div>
                </>
              )}

              {actionData?.error && (
                <p className="text-sm text-[var(--accent-red)] text-center">{actionData.error}</p>
              )}

              <div className="text-center pt-1">
                <Link
                  to="/login"
                  className="text-sm text-[var(--text-tertiary)] hover:text-[var(--text-primary)] underline-offset-4 hover:underline"
                >
                  {t("auth.emailOtp.backToLogin")}
                </Link>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </>
  );
}
