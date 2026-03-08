import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { GlassAlert } from "../components/GlassAlert";
import { GlassButton } from "../components/GlassButton";
import { Logo } from "../components/Logo";

export default function Error(
  props: PageProps<Extract<KcContext, { pageId: "error.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { message, client, skipLink, realm } = kcContext;
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
            {msg("errorTitle")}
          </h2>
        </div>

        <GlassAlert variant="error">
          <span dangerouslySetInnerHTML={{ __html: message.summary }} />
        </GlassAlert>

        {!skipLink && client !== undefined && client.baseUrl !== undefined && (
          <div className="login-footer" style={{ marginTop: "1.5rem" }}>
            <GlassButton
              variant="secondary"
              onClick={() => { window.location.href = client.baseUrl!; }}
            >
              {msgStr("backToApplication")}
            </GlassButton>
          </div>
        )}
      </GlassCard>
    </PageLayout>
  );
}
