import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { Logo } from "../components/Logo";

const AUTHENTICATOR_ICONS: Record<string, string> = {
  "kcAuthenticatorOTPClass": "🔢",
  "kcAuthenticatorWebAuthnClass": "🔑",
  "kcAuthenticatorWebAuthnPasswordlessClass": "🔐",
  "kcAuthenticatorDefaultClass": "🛡️",
};

function getIcon(iconCssClass?: string): string {
  if (!iconCssClass) return "🛡️";
  for (const [cls, icon] of Object.entries(AUTHENTICATOR_ICONS)) {
    if (iconCssClass.includes(cls)) return icon;
  }
  return "🛡️";
}

export default function SelectAuthenticator(
  props: PageProps<Extract<KcContext, { pageId: "select-authenticator.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { url, auth } = kcContext;
  const { msgStr, advancedMsg } = i18n;
  const branding = useBrandingContext();

  return (
    <PageLayout lightModeLabel={msgStr("lightMode")} darkModeLabel={msgStr("darkMode")}>
      <GlassCard>
        {/* Header */}
        <div className="login-header">
          <Logo
            logoUrl={branding.logo_url}
            companyName={branding.company_name}
          />
          <h2 className="login-title" style={{ fontSize: "20px" }}>
            {msgStr("loginChooseAuthenticator")}
          </h2>
          <p className="login-subtitle">{msgStr("selectAuthenticatorSubtitle")}</p>
        </div>

        {/* Authenticator List */}
        <form id="kc-select-credential-form" action={url.loginAction} method="post">
          <div className="authenticator-list">
            {auth.authenticationSelections.map((selection, i) => (
              <button
                key={i}
                className="authenticator-option"
                type="submit"
                name="authenticationExecution"
                value={selection.authExecId}
              >
                <div className="authenticator-option-icon">
                  {getIcon(selection.iconCssClass)}
                </div>
                <div className="authenticator-option-body">
                  <div className="authenticator-option-heading">
                    {advancedMsg(selection.displayName)}
                  </div>
                  <div className="authenticator-option-description">
                    {advancedMsg(selection.helpText)}
                  </div>
                </div>
                <div className="authenticator-option-arrow">›</div>
              </button>
            ))}
          </div>
        </form>
      </GlassCard>
    </PageLayout>
  );
}
