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

export default function LoginConfigTotp(
  props: PageProps<Extract<KcContext, { pageId: "login-config-totp.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const { url, isAppInitiatedAction, totp, mode, messagesPerField } = kcContext;
  const { msgStr, advancedMsg } = i18n;
  const branding = useBrandingContext();

  const [isSubmitting, setIsSubmitting] = useState(false);

  const onSubmit: FormEventHandler<HTMLFormElement> = () => {
    setIsSubmitting(true);
    return true;
  };

  const hasFieldError = messagesPerField.existsError("totp", "userLabel");

  return (
    <PageLayout lightModeLabel={msgStr("lightMode")} darkModeLabel={msgStr("darkMode")}>
      <GlassCard className="totp-config-card">
        {/* Header */}
        <div className="login-header">
          <Logo
            logoUrl={branding.logo_url}
            companyName={branding.company_name}
          />
          <h2 className="login-title" style={{ fontSize: "20px" }}>
            {msgStr("loginTotpTitle")}
          </h2>
          <p className="login-subtitle">{msgStr("configTotpSubtitle")}</p>
        </div>

        {/* Error Messages */}
        {hasFieldError && (
          <GlassAlert variant="error">
            {messagesPerField.existsError("totp") && messagesPerField.getFirstError("totp")}
            {messagesPerField.existsError("userLabel") && messagesPerField.getFirstError("userLabel")}
          </GlassAlert>
        )}

        {/* Setup Steps */}
        <div className="totp-steps">
          {/* Step 1: Install authenticator app */}
          <div className="totp-step">
            <span className="totp-step-number">1</span>
            <div className="totp-step-content">
              <p className="totp-step-text">{msgStr("loginTotpStep1")}</p>
              <div className="totp-supported-apps">
                {totp.supportedApplications.map(app => (
                  <span key={app} className="totp-app-badge">
                    {advancedMsg(app)}
                  </span>
                ))}
              </div>
            </div>
          </div>

          {/* Step 2: Scan QR code or enter manually */}
          <div className="totp-step">
            <span className="totp-step-number">2</span>
            <div className="totp-step-content">
              {mode === "manual" ? (
                <>
                  <p className="totp-step-text">{msgStr("loginTotpManualStep2")}</p>
                  <div className="totp-secret-key">
                    <code id="kc-totp-secret-key">{totp.totpSecretEncoded}</code>
                  </div>
                  <a href={totp.qrUrl} id="mode-barcode" className="form-link totp-mode-link">
                    {msgStr("loginTotpScanBarcode")}
                  </a>
                  <div className="totp-policy-info">
                    <div className="totp-policy-item">
                      <span className="totp-policy-label">{msgStr("loginTotpType")}</span>
                      <span className="totp-policy-value">{msgStr(`loginTotp.${totp.policy.type}`)}</span>
                    </div>
                    <div className="totp-policy-item">
                      <span className="totp-policy-label">{msgStr("loginTotpAlgorithm")}</span>
                      <span className="totp-policy-value">{totp.policy.getAlgorithmKey()}</span>
                    </div>
                    <div className="totp-policy-item">
                      <span className="totp-policy-label">{msgStr("loginTotpDigits")}</span>
                      <span className="totp-policy-value">{totp.policy.digits}</span>
                    </div>
                    {totp.policy.type === "totp" ? (
                      <div className="totp-policy-item">
                        <span className="totp-policy-label">{msgStr("loginTotpInterval")}</span>
                        <span className="totp-policy-value">{totp.policy.period}s</span>
                      </div>
                    ) : (
                      <div className="totp-policy-item">
                        <span className="totp-policy-label">{msgStr("loginTotpCounter")}</span>
                        <span className="totp-policy-value">{totp.policy.initialCounter}</span>
                      </div>
                    )}
                  </div>
                </>
              ) : (
                <>
                  <p className="totp-step-text">{msgStr("loginTotpStep2")}</p>
                  <div className="totp-qr-code">
                    <img
                      id="kc-totp-secret-qr-code"
                      src={`data:image/png;base64, ${totp.totpSecretQrCode}`}
                      alt="QR Code"
                    />
                  </div>
                  <a href={totp.manualUrl} id="mode-manual" className="form-link totp-mode-link">
                    {msgStr("loginTotpUnableToScan")}
                  </a>
                </>
              )}
            </div>
          </div>

          {/* Step 3: Enter code */}
          <div className="totp-step">
            <span className="totp-step-number">3</span>
            <div className="totp-step-content">
              <p className="totp-step-text">{msgStr("loginTotpStep3")}</p>
            </div>
          </div>
        </div>

        {/* Form */}
        <form
          id="kc-totp-settings-form"
          onSubmit={onSubmit}
          action={url.loginAction}
          method="post"
        >
          {/* Verification Code */}
          <GlassInput
            id="totp"
            name="totp"
            type="text"
            autoComplete="off"
            autoFocus
            inputMode="numeric"
            maxLength={6}
            aria-invalid={messagesPerField.existsError("totp")}
            label={msgStr("authenticatorCode")}
            variant="otp"
            error={messagesPerField.existsError("totp") ? messagesPerField.getFirstError("totp") : undefined}
          />

          {/* Device Name */}
          <GlassInput
            id="userLabel"
            name="userLabel"
            type="text"
            autoComplete="off"
            aria-invalid={messagesPerField.existsError("userLabel")}
            label={msgStr("loginTotpDeviceName")}
            placeholder={msgStr("configTotpDevicePlaceholder")}
            error={messagesPerField.existsError("userLabel") ? messagesPerField.getFirstError("userLabel") : undefined}
          />

          <input type="hidden" id="totpSecret" name="totpSecret" value={totp.totpSecret} />
          {mode && <input type="hidden" id="mode" value={mode} />}

          {/* Logout other sessions checkbox */}
          <div className="form-options" style={{ justifyContent: "flex-start", marginBottom: "20px" }}>
            <label className="form-checkbox">
              <input
                type="checkbox"
                id="logout-sessions"
                name="logout-sessions"
                value="on"
                defaultChecked={true}
              />
              {msgStr("logoutOtherSessions")}
            </label>
          </div>

          {/* Submit / Cancel buttons */}
          {isAppInitiatedAction ? (
            <div className="totp-button-group">
              <GlassButton
                disabled={isSubmitting}
                type="submit"
                id="saveTOTPBtn"
                variant="primary"
                loading={isSubmitting}
              >
                {msgStr("doSubmit")}
              </GlassButton>
              <GlassButton
                type="submit"
                name="cancel-aia"
                value="true"
                id="cancelTOTPBtn"
                variant="secondary"
              >
                {msgStr("doCancel")}
              </GlassButton>
            </div>
          ) : (
            <GlassButton
              disabled={isSubmitting}
              type="submit"
              id="saveTOTPBtn"
              variant="primary"
              loading={isSubmitting}
            >
              {msgStr("doSubmit")}
            </GlassButton>
          )}
        </form>
      </GlassCard>
    </PageLayout>
  );
}
