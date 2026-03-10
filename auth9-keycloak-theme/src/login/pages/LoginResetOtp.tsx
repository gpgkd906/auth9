import { useState, type FormEventHandler } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { GlassButton } from "../components/GlassButton";
import { GlassAlert } from "../components/GlassAlert";
import { Logo } from "../components/Logo";

export default function LoginResetOtp(
  props: PageProps<Extract<KcContext, { pageId: "login-reset-otp.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { url, messagesPerField, configuredOtpCredentials } = kcContext;
  const { msgStr } = i18n;
  const branding = useBrandingContext();

  const [isSubmitting, setIsSubmitting] = useState(false);

  const onSubmit: FormEventHandler<HTMLFormElement> = () => {
    setIsSubmitting(true);
    return true;
  };

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
            {msgStr("doLogIn")}
          </h2>
          <p className="login-subtitle">{msgStr("resetOtpDescription")}</p>
        </div>

        {/* Error Message */}
        {messagesPerField.existsError("totp") && (
          <GlassAlert variant="error">
            {messagesPerField.getFirstError("totp")}
          </GlassAlert>
        )}

        <form
          id="kc-otp-reset-form"
          onSubmit={onSubmit}
          action={url.loginAction}
          method="post"
        >
          <div className="reset-otp-list">
            {configuredOtpCredentials.userOtpCredentials.map((credential, index) => (
              <label key={credential.id} className="reset-otp-item" htmlFor={`kc-otp-credential-${index}`}>
                <input
                  id={`kc-otp-credential-${index}`}
                  type="radio"
                  name="selectedCredentialId"
                  value={credential.id}
                  defaultChecked={credential.id === configuredOtpCredentials.selectedCredentialId}
                />
                <span className="reset-otp-item-icon">🔑</span>
                <span className="reset-otp-item-label">
                  {credential.userLabel}
                </span>
              </label>
            ))}
          </div>

          <GlassButton
            disabled={isSubmitting}
            type="submit"
            id="kc-otp-reset-form-submit"
            variant="primary"
            loading={isSubmitting}
          >
            {msgStr("doSubmit")}
          </GlassButton>
        </form>
      </GlassCard>
    </PageLayout>
  );
}
