import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { GlassAlert } from "../components/GlassAlert";
import { Logo } from "../components/Logo";

export default function Info(
  props: PageProps<Extract<KcContext, { pageId: "info.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { message, messageHeader, requiredActions, skipLink, pageRedirectUri, actionUri, url, realm, client } = kcContext;
  const { msg, msgStr, advancedMsg, advancedMsgStr } = i18n;
  const branding = useBrandingContext();

  return (
    <PageLayout>
      <GlassCard>
        {/* Header */}
        <div className="login-header">
          <Logo
            logoUrl={branding.logo_url}
            companyName={branding.company_name}
            fallbackText={realm.displayName}
          />
          <h2 className="login-title" style={{ fontSize: "20px" }}>
            {messageHeader ?? msg("accountUpdatedMessage")}
          </h2>
        </div>

        {/* Required Actions */}
        {requiredActions && (
          <p className="login-subtitle">
            {requiredActions.map((requiredAction) => advancedMsgStr(`requiredAction.${requiredAction}`)).join(", ")}
          </p>
        )}

        {/* Message */}
        <GlassAlert variant={message.type === "error" ? "error" : message.type === "warning" ? "warning" : "success"}>
          {advancedMsg(message.summary)}
        </GlassAlert>

        {/* Action buttons */}
        {!skipLink && (
          <div className="login-footer" style={{ marginTop: "1.5rem" }}>
            {pageRedirectUri ? (
              <a href={pageRedirectUri} className="form-link--secondary">
                {msg("backToApplication")}
              </a>
            ) : actionUri ? (
              <a href={actionUri} className="form-link--secondary">
                {msg("proceedWithAction")}
              </a>
            ) : client.baseUrl ? (
              <a href={client.baseUrl} className="form-link--secondary">
                {msg("backToApplication")}
              </a>
            ) : (
              <a href={url.loginUrl} className="form-link--secondary">
                {"\u2190 " + (msgStr("backToLogin") || "Back to Login")}
              </a>
            )}
          </div>
        )}
      </GlassCard>
    </PageLayout>
  );
}
