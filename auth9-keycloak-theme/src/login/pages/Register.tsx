import { useState, type FormEventHandler } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { getKcClsx } from "keycloakify/login/lib/kcClsx";
import UserProfileFormFields from "keycloakify/login/UserProfileFormFields";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { GlassButton } from "../components/GlassButton";
import { Logo } from "../components/Logo";

export default function Register(
  props: PageProps<Extract<KcContext, { pageId: "register.ftl" }>, I18n>
) {
  const { kcContext, i18n, doUseDefaultCss, classes } = props;
  const { url, realm, recaptchaRequired, recaptchaSiteKey } = kcContext;
  const { msg, msgStr } = i18n;
  const branding = useBrandingContext();

  const { kcClsx } = getKcClsx({ doUseDefaultCss, classes });

  const [isFormSubmitting, setIsFormSubmitting] = useState(false);
  const [isFormSubmittable, setIsFormSubmittable] = useState(false);

  const onSubmit: FormEventHandler<HTMLFormElement> = () => {
    setIsFormSubmitting(true);
    return true;
  };

  return (
    <PageLayout>
      {/* Custom styles for Keycloakify UserProfileFormFields */}
      <style>{`
        .auth9-register-form input[type="text"],
        .auth9-register-form input[type="email"],
        .auth9-register-form input[type="password"] {
          width: 100%;
          padding: 12px 16px;
          font-size: 14px;
          background: var(--input-bg);
          border: 1px solid var(--input-border);
          border-radius: var(--radius-lg);
          color: var(--text-primary);
          font-family: inherit;
          transition: all 0.2s ease;
          box-sizing: border-box;
        }
        .auth9-register-form input[type="text"]:focus,
        .auth9-register-form input[type="email"]:focus,
        .auth9-register-form input[type="password"]:focus {
          outline: none;
          background: var(--input-bg-hover);
          border-color: var(--accent-blue);
          box-shadow: 0 0 0 3px var(--accent-blue-light);
        }
        .auth9-register-form input::placeholder {
          color: var(--text-tertiary);
        }
        .auth9-register-form label {
          display: block;
          font-size: 13px;
          font-weight: 500;
          margin-bottom: 8px;
          color: var(--text-secondary);
        }
        .auth9-register-form .pf-v5-c-form__group,
        .auth9-register-form .pf-c-form__group {
          margin-bottom: 20px;
        }
        .auth9-register-form .pf-v5-c-form__helper-text,
        .auth9-register-form .pf-c-form__helper-text {
          color: var(--accent-red);
          font-size: 13px;
          margin-top: 6px;
        }
      `}</style>

      <GlassCard className="login-card" style={{ maxWidth: "480px" }}>
        {/* Header */}
        <div className="login-header">
          <Logo
            logoUrl={branding.logo_url}
            companyName={branding.company_name}
            fallbackText={realm.displayName || msgStr("registerTitle")}
          />
          <p className="login-subtitle">{msgStr("registerTitle")}</p>
        </div>

        <form
          id="kc-register-form"
          className="auth9-register-form"
          onSubmit={onSubmit}
          action={url.registrationAction}
          method="post"
        >
          <UserProfileFormFields
            kcContext={kcContext}
            i18n={i18n}
            kcClsx={kcClsx}
            doMakeUserConfirmPassword={true}
            onIsFormSubmittableValueChange={setIsFormSubmittable}
          />

          {/* reCAPTCHA */}
          {recaptchaRequired && recaptchaSiteKey && (
            <div style={{ marginBottom: "24px" }}>
              <div className="g-recaptcha" data-sitekey={recaptchaSiteKey} />
            </div>
          )}

          {/* Submit Button */}
          <GlassButton
            disabled={isFormSubmitting || !isFormSubmittable}
            type="submit"
            variant="primary"
            loading={isFormSubmitting}
          >
            {msgStr("doRegister")}
          </GlassButton>
        </form>

        {/* Back to Login */}
        <div className="login-footer">
          {msgStr("backToLogin") || "Already have an account?"}{" "}
          <a href={url.loginUrl}>{msg("doLogIn")}</a>
        </div>
      </GlassCard>
    </PageLayout>
  );
}
