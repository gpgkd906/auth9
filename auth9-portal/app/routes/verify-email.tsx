import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { useLoaderData, Link } from "react-router";
import { getBrandMark } from "~/components/auth/AuthBrandPanel";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { hostedLoginApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.verifyEmail.metaTitle");
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
    // Fall back to static branding
  }

  if (!token) {
    return { error: translate(locale, "auth.verifyEmail.invalidToken"), branding };
  }

  try {
    await hostedLoginApi.verifyEmail(token);
    return { success: true, branding };
  } catch {
    return { error: translate(locale, "auth.verifyEmail.expiredToken"), branding };
  }
}

export default function VerifyEmailPage() {
  const { t } = useI18n();
  const rawLoaderData = (useLoaderData<typeof loader>() ?? {}) as {
    success?: boolean;
    error?: string;
    branding?: BrandingConfig;
  };
  const branding = { ...DEFAULT_PUBLIC_BRANDING, ...(rawLoaderData.branding ?? {}) };

  const renderLogo = () =>
    branding.logo_url ? (
      <img
        src={branding.logo_url}
        alt={branding.company_name || "Auth9"}
        className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
        referrerPolicy="no-referrer"
        crossOrigin="anonymous"
      />
    ) : (
      <div className="logo-icon mx-auto mb-4">{getBrandMark(branding.company_name || "Auth9")}</div>
    );

  if (rawLoaderData.success) {
    return (
      <AuthPageShell
        branding={branding}
        panelEyebrow={t("auth.shared.hostedEyebrow")}
        panelTitle={t("auth.verifyEmail.panelTitle")}
        panelDescription={t("auth.verifyEmail.panelDescription")}
      >
        <Card className="w-full max-w-md animate-fade-in-up">
          <CardHeader className="text-center">
            {renderLogo()}
            <CardTitle className="text-2xl">{t("auth.verifyEmail.successTitle")}</CardTitle>
            <CardDescription className="auth-form-description">
              {t("auth.verifyEmail.successDescription")}
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
      branding={branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.verifyEmail.panelTitle")}
      panelDescription={t("auth.verifyEmail.panelDescription")}
    >
      <Card className="w-full max-w-md animate-fade-in-up">
        <CardHeader className="text-center">
          {renderLogo()}
          <CardTitle className="text-2xl">{t("auth.verifyEmail.errorTitle")}</CardTitle>
          <CardDescription className="auth-form-description">
            {rawLoaderData.error}
          </CardDescription>
        </CardHeader>
        <CardContent className="text-center">
          <Link to="/login">
            <Button variant="outline">{t("common.buttons.backToLogin")}</Button>
          </Link>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
