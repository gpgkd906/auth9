import { useState, type FormEventHandler } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { PageLayout } from "../components/PageLayout";
import { GlassCard } from "../components/GlassCard";
import { GlassInput } from "../components/GlassInput";
import { GlassButton } from "../components/GlassButton";
import { GlassAlert } from "../components/GlassAlert";
import { Logo } from "../components/Logo";

export default function LoginResetPassword(
  props: PageProps<Extract<KcContext, { pageId: "login-reset-password.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { url, realm, messagesPerField, auth } = kcContext;
  const { msg, msgStr } = i18n;
  const branding = useBrandingContext();

  const [isSubmitting, setIsSubmitting] = useState(false);

  const onSubmit: FormEventHandler<HTMLFormElement> = () => {
    setIsSubmitting(true);
    return true;
  };

  // Get appropriate username label
  const usernameLabel = !realm.loginWithEmailAllowed
    ? msgStr("username")
    : !realm.registrationEmailAsUsername
      ? msgStr("usernameOrEmail")
      : msgStr("email");

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
            {msgStr("emailForgotTitle")}
          </h2>
          <p className="login-subtitle">{msg("emailInstruction")}</p>
        </div>

        {/* Error Message */}
        {messagesPerField.existsError("username") && (
          <GlassAlert variant="error">
            {messagesPerField.getFirstError("username")}
          </GlassAlert>
        )}

        <form
          id="kc-reset-password-form"
          onSubmit={onSubmit}
          action={url.loginAction}
          method="post"
        >
          {/* Username/Email */}
          <GlassInput
            id="username"
            name="username"
            type="text"
            autoFocus
            defaultValue={auth.attemptedUsername ?? ""}
            aria-invalid={messagesPerField.existsError("username")}
            label={usernameLabel}
          />

          {/* Submit Button */}
          <GlassButton
            disabled={isSubmitting}
            type="submit"
            variant="primary"
            loading={isSubmitting}
          >
            {msgStr("doSubmit")}
          </GlassButton>
        </form>

        {/* Back to Login */}
        <div className="login-footer">
          <a href={url.loginUrl} className="form-link--secondary">
            {msgStr("backToLogin") || "\u2190 Back to Login"}
          </a>
        </div>
      </GlassCard>
    </PageLayout>
  );
}
