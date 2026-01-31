import { useState, type FormEventHandler, type CSSProperties } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";

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

  const inputStyle: CSSProperties = {
    width: "100%",
    padding: "0.75rem 1rem",
    fontSize: "1.25rem",
    fontFamily: "monospace",
    letterSpacing: "0.25em",
    textAlign: "center",
    border: `1px solid ${branding.secondary_color}40`,
    borderRadius: "0.75rem",
    outline: "none",
    transition: "border-color 0.2s, box-shadow 0.2s",
    backgroundColor: "#fff",
  };

  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "1rem",
        backgroundColor: branding.background_color,
        fontFamily:
          "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
      }}
    >
      <div
        style={{
          width: "100%",
          maxWidth: "420px",
          backgroundColor: "#fff",
          borderRadius: "1.5rem",
          boxShadow:
            "0 10px 40px rgba(0, 0, 0, 0.08), 0 2px 10px rgba(0, 0, 0, 0.04)",
          padding: "2.5rem",
        }}
      >
        {/* Header */}
        <div style={{ textAlign: "center", marginBottom: "2rem" }}>
          {branding.logo_url ? (
            <img
              src={branding.logo_url}
              alt={branding.company_name || "Logo"}
              style={{
                height: "48px",
                maxWidth: "200px",
                objectFit: "contain",
                marginBottom: "1rem",
              }}
            />
          ) : branding.company_name ? (
            <h1
              style={{
                fontSize: "1.75rem",
                fontWeight: 700,
                color: branding.primary_color,
                margin: 0,
                marginBottom: "1rem",
              }}
            >
              {branding.company_name}
            </h1>
          ) : null}

          <h2
            style={{
              fontSize: "1.25rem",
              fontWeight: 600,
              color: branding.text_color,
              margin: 0,
            }}
          >
            {msgStr("doLogIn")}
          </h2>
          <p
            style={{
              marginTop: "0.75rem",
              fontSize: "0.9375rem",
              color: "#6b7280",
              lineHeight: 1.5,
            }}
          >
            {msg("loginOtpOneTime")}
          </p>
        </div>

        {/* Error Message */}
        {messagesPerField.existsError("totp") && (
          <div
            style={{
              marginBottom: "1.25rem",
              padding: "0.75rem 1rem",
              backgroundColor: "#fef2f2",
              border: "1px solid #fecaca",
              borderRadius: "0.75rem",
              color: "#dc2626",
              fontSize: "0.875rem",
            }}
          >
            {messagesPerField.getFirstError("totp")}
          </div>
        )}

        <form id="kc-otp-login-form" onSubmit={onSubmit} action={url.loginAction} method="post">
          {/* OTP Device Selector (if multiple devices) */}
          {otpLogin.userOtpCredentials.length > 1 && (
            <div style={{ marginBottom: "1.5rem" }}>
              <label
                style={{
                  display: "block",
                  fontSize: "0.875rem",
                  fontWeight: 500,
                  marginBottom: "0.5rem",
                  color: branding.text_color,
                }}
              >
                Select OTP Device
              </label>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "0.5rem",
                }}
              >
                {otpLogin.userOtpCredentials.map((credential, index) => (
                  <label
                    key={credential.id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      padding: "0.75rem 1rem",
                      border: `1px solid ${branding.secondary_color}40`,
                      borderRadius: "0.75rem",
                      cursor: "pointer",
                      transition: "border-color 0.2s",
                    }}
                  >
                    <input
                      type="radio"
                      id={`kc-otp-credential-${index}`}
                      name="selectedCredentialId"
                      value={credential.id}
                      defaultChecked={credential.id === otpLogin.selectedCredentialId}
                      style={{
                        marginRight: "0.75rem",
                        accentColor: branding.primary_color,
                      }}
                    />
                    <span style={{ color: branding.text_color }}>
                      {credential.userLabel}
                    </span>
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
          <div style={{ marginBottom: "1.5rem" }}>
            <label
              htmlFor="otp"
              style={{
                display: "block",
                fontSize: "0.875rem",
                fontWeight: 500,
                marginBottom: "0.5rem",
                color: branding.text_color,
              }}
            >
              {msg("loginOtpOneTime")}
            </label>
            <input
              type="text"
              id="otp"
              name="otp"
              autoComplete="one-time-code"
              autoFocus
              inputMode="numeric"
              maxLength={6}
              aria-invalid={messagesPerField.existsError("totp")}
              style={inputStyle}
              onFocus={(e) => {
                e.target.style.borderColor = branding.primary_color;
                e.target.style.boxShadow = `0 0 0 3px ${branding.primary_color}20`;
              }}
              onBlur={(e) => {
                e.target.style.borderColor = `${branding.secondary_color}40`;
                e.target.style.boxShadow = "none";
              }}
            />
          </div>

          {/* Submit Button */}
          <button
            disabled={isSubmitting}
            type="submit"
            name="login"
            style={{
              width: "100%",
              padding: "0.875rem 1rem",
              backgroundColor: branding.primary_color,
              color: "#fff",
              fontSize: "1rem",
              fontWeight: 600,
              border: "none",
              borderRadius: "0.75rem",
              cursor: isSubmitting ? "not-allowed" : "pointer",
              opacity: isSubmitting ? 0.6 : 1,
              transition: "opacity 0.2s, transform 0.1s",
            }}
            onMouseOver={(e) => {
              if (!isSubmitting) {
                e.currentTarget.style.opacity = "0.9";
              }
            }}
            onMouseOut={(e) => {
              if (!isSubmitting) {
                e.currentTarget.style.opacity = "1";
              }
            }}
          >
            {msgStr("doLogIn")}
          </button>
        </form>
      </div>
    </div>
  );
}
