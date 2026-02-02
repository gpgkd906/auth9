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

export default function LoginOtp(
  props: PageProps<Extract<KcContext, { pageId: "login-otp.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { url, messagesPerField, otpLogin } = kcContext;
  const { msg, msgStr } = i18n;
  const branding = useBrandingContext();

  const [isSubmitting, setIsSubmitting] = useState(false);

  const onSubmit: FormEventHandler<HTMLFormElement> = () => {
    setIsSubmitting(true);
    return true;
  };

  return (
    <PageLayout>
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
          <p className="login-subtitle">{msg("loginOtpOneTime")}</p>
        </div>

        {/* Error Message */}
        {messagesPerField.existsError("totp") && (
          <GlassAlert variant="error">
            {messagesPerField.getFirstError("totp")}
          </GlassAlert>
        )}

        <form
          id="kc-otp-login-form"
          onSubmit={onSubmit}
          action={url.loginAction}
          method="post"
        >
          {/* OTP Device Selector (if multiple devices) */}
          {otpLogin.userOtpCredentials.length > 1 && (
            <div className="form-group">
              <label className="form-label">Select OTP Device</label>
              <div className="otp-devices">
                {otpLogin.userOtpCredentials.map((credential, index) => (
                  <label key={credential.id} className="otp-device-option">
                    <input
                      type="radio"
                      id={`kc-otp-credential-${index}`}
                      name="selectedCredentialId"
                      value={credential.id}
                      defaultChecked={credential.id === otpLogin.selectedCredentialId}
                    />
                    <span>{credential.userLabel}</span>
                  </label>
                ))}
              </div>
            </div>
          )}

          {/* Single device - hidden input */}
          {otpLogin.userOtpCredentials.length === 1 && (
            <input
              type="hidden"
              name="selectedCredentialId"
              value={otpLogin.userOtpCredentials[0].id}
            />
          )}

          {/* OTP Input */}
          <GlassInput
            id="otp"
            name="otp"
            type="text"
            autoComplete="one-time-code"
            autoFocus
            inputMode="numeric"
            maxLength={6}
            aria-invalid={messagesPerField.existsError("totp")}
            label={msgStr("loginOtpOneTime")}
            variant="otp"
          />

          {/* Submit Button */}
          <GlassButton
            disabled={isSubmitting}
            type="submit"
            name="login"
            variant="primary"
            loading={isSubmitting}
          >
            {msgStr("doLogIn")}
          </GlassButton>
        </form>
      </GlassCard>
    </PageLayout>
  );
}
