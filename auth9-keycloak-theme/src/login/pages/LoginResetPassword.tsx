import { useState, type FormEventHandler, type CSSProperties } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";

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

  const inputStyle: CSSProperties = {
    width: "100%",
    padding: "0.75rem 1rem",
    fontSize: "1rem",
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
          ) : (
            <h1
              style={{
                fontSize: "1.75rem",
                fontWeight: 700,
                color: branding.primary_color,
                margin: 0,
                marginBottom: "1rem",
              }}
            >
              {realm.displayName}
            </h1>
          )}
          <h2
            style={{
              fontSize: "1.25rem",
              fontWeight: 600,
              color: branding.text_color,
              margin: 0,
            }}
          >
            {msgStr("emailForgotTitle")}
          </h2>
          <p
            style={{
              marginTop: "0.75rem",
              fontSize: "0.9375rem",
              color: "#6b7280",
              lineHeight: 1.5,
            }}
          >
            {msg("emailInstruction")}
          </p>
        </div>

        {/* Error Message */}
        {messagesPerField.existsError("username") && (
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
            {messagesPerField.getFirstError("username")}
          </div>
        )}

        <form id="kc-reset-password-form" onSubmit={onSubmit} action={url.loginAction} method="post">
          {/* Username/Email */}
          <div style={{ marginBottom: "1.5rem" }}>
            <label
              htmlFor="username"
              style={{
                display: "block",
                fontSize: "0.875rem",
                fontWeight: 500,
                marginBottom: "0.5rem",
                color: branding.text_color,
              }}
            >
              {!realm.loginWithEmailAllowed
                ? msg("username")
                : !realm.registrationEmailAsUsername
                  ? msg("usernameOrEmail")
                  : msg("email")}
            </label>
            <input
              type="text"
              id="username"
              name="username"
              autoFocus
              defaultValue={auth.attemptedUsername ?? ""}
              aria-invalid={messagesPerField.existsError("username")}
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
            {msgStr("doSubmit")}
          </button>
        </form>

        {/* Back to Login */}
        <div
          style={{
            marginTop: "1.5rem",
            textAlign: "center",
            fontSize: "0.875rem",
          }}
        >
          <a
            href={url.loginUrl}
            style={{
              color: branding.secondary_color,
              textDecoration: "none",
            }}
            onMouseOver={(e) => (e.currentTarget.style.textDecoration = "underline")}
            onMouseOut={(e) => (e.currentTarget.style.textDecoration = "none")}
          >
            {msgStr("backToLogin") || "← Back to Login"}
          </a>
        </div>
      </div>
    </div>
  );
}
