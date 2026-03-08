import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { GlassAlert } from "../components/GlassAlert";
import { GlassButton } from "../components/GlassButton";
import { Logo } from "../components/Logo";

export default function LoginPageExpired(
  props: PageProps<Extract<KcContext, { pageId: "login-page-expired.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { url, realm } = kcContext;
  const { msg, msgStr } = i18n;
  const branding = useBrandingContext();

  return (
    <PageLayout lightModeLabel={msgStr("lightMode")} darkModeLabel={msgStr("darkMode")}>
      <GlassCard>
        <div className="login-header">
          <Logo
            logoUrl={branding.logo_url}
            companyName={branding.company_name}
            fallbackText={realm.displayName}
          />
          <h2 className="login-title" style={{ fontSize: "20px" }}>
            {msg("pageExpiredTitle")}
          </h2>
        </div>

        <GlassAlert variant="warning">
          {msg("pageExpiredMsg1")}
        </GlassAlert>

        <div style={{ display: "flex", flexDirection: "column", gap: "0.75rem", marginTop: "1.5rem" }}>
          <GlassButton
            variant="primary"
            onClick={() => { window.location.href = url.loginRestartFlowUrl; }}
          >
            {msgStr("doClickHere")} — {msgStr("pageExpiredMsg1")}
          </GlassButton>

          <GlassButton
            variant="secondary"
            onClick={() => { window.location.href = url.loginAction; }}
          >
            {msgStr("doClickHere")} — {msgStr("pageExpiredMsg2")}
          </GlassButton>
        </div>
      </GlassCard>
    </PageLayout>
  );
}
