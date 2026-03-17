import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { getBrandMark } from "~/components/auth/AuthBrandPanel";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { resolveLocale } from "~/services/locale.server";
import { publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.mfaVerify.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to default Portal branding when public config is unavailable.
  }

  return {
    locale,
    branding,
  };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const code = String(formData.get("code") || "").trim();

  if (!code) {
    return { error: translate(locale, "auth.mfaVerify.codeRequired") };
  }

  return { error: translate(locale, "auth.mfaVerify.pendingIntegration") };
}

export default function MfaVerifyPage() {
  const { t } = useI18n();
  const loaderData = (useLoaderData<typeof loader>() ?? {}) as { branding?: BrandingConfig };
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData.branding ?? {}) };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <AuthPageShell
      branding={branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.mfaVerify.panelTitle")}
      panelDescription={t("auth.mfaVerify.panelDescription")}
    >
      <Card className="auth-form-card w-full max-w-md animate-fade-in-up">
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
          <CardTitle className="text-2xl">{t("auth.mfaVerify.title")}</CardTitle>
          <CardDescription className="auth-form-description">{t("auth.mfaVerify.description")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-2xl border border-dashed border-[var(--glass-border-subtle)] bg-white/65 p-4 text-left text-sm leading-6 text-[var(--text-secondary)] dark:bg-white/6">
            <p className="font-medium text-[var(--text-primary)]">{t("auth.mfaVerify.extensionTitle")}</p>
            <p className="mt-2">{t("auth.mfaVerify.extensionDescription")}</p>
          </div>

          <Form method="post" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="code">{t("auth.mfaVerify.codeLabel")}</Label>
              <Input
                id="code"
                name="code"
                inputMode="numeric"
                autoComplete="one-time-code"
                placeholder={t("auth.mfaVerify.codePlaceholder")}
              />
            </div>

            {actionData?.error ? (
              <div className="rounded-xl border border-[var(--accent-red)]/25 bg-[var(--accent-red)]/12 p-3 text-sm text-[var(--accent-red)]">
                {actionData.error}
              </div>
            ) : null}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("auth.mfaVerify.verifying") : t("auth.mfaVerify.submit")}
            </Button>

            <div className="text-center text-sm">
              <Link to="/login" className="font-medium text-[var(--accent-blue)] hover:underline">
                {t("common.buttons.backToLogin")}
              </Link>
            </div>
          </Form>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
