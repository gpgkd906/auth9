import { useState, type FormEventHandler, type CSSProperties } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";

export default function Login(
  props: PageProps<Extract<KcContext, { pageId: "login.ftl" }>, I18n>
) {
  const { kcContext, i18n } = props;
  const {
    realm,
    url,
    usernameHidden,
    login,
    registrationDisabled,
    messagesPerField,
    social,
    auth,
  } = kcContext;
  const { msg, msgStr } = i18n;
  const branding = useBrandingContext();

  const [isLoginButtonDisabled, setIsLoginButtonDisabled] = useState(false);

  const onSubmit: FormEventHandler<HTMLFormElement> = () => {
    setIsLoginButtonDisabled(true);
    return true;
  };

  // Common input styles
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
        {/* Logo or Company Name */}
        <div style={{ textAlign: "center", marginBottom: "2rem" }}>
          {branding.logo_url ? (
            <img
              src={branding.logo_url}
              alt={branding.company_name || "Logo"}
              style={{ height: "48px", maxWidth: "200px", objectFit: "contain" }}
            />
          ) : branding.company_name ? (
            <h1
              style={{
                fontSize: "1.75rem",
                fontWeight: 700,
                color: branding.primary_color,
                margin: 0,
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
              }}
            >
              {realm.displayName || msgStr("loginTitleHtml", realm.displayNameHtml)}
            </h1>
          )}
        </div>

        {/* Error Messages */}
        {messagesPerField.existsError("username", "password") && (
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
            {messagesPerField.getFirstError("username", "password")}
          </div>
        )}

        <form id="kc-form-login" onSubmit={onSubmit} action={url.loginAction} method="post">
          {/* Username/Email */}
          {!usernameHidden && (
            <div style={{ marginBottom: "1.25rem" }}>
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
                tabIndex={2}
                id="username"
                name="username"
                defaultValue={login.username ?? ""}
                type="text"
                autoFocus
                autoComplete="username"
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
          )}

          {/* Password */}
          <div style={{ marginBottom: "1.5rem" }}>
            <label
              htmlFor="password"
              style={{
                display: "block",
                fontSize: "0.875rem",
                fontWeight: 500,
                marginBottom: "0.5rem",
                color: branding.text_color,
              }}
            >
              {msg("password")}
            </label>
            <input
              tabIndex={3}
              id="password"
              name="password"
              type="password"
              autoComplete="current-password"
              aria-invalid={messagesPerField.existsError("password")}
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

          {/* Remember Me & Forgot Password */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              marginBottom: "1.5rem",
              fontSize: "0.875rem",
            }}
          >
            {realm.rememberMe && !usernameHidden && (
              <label
                style={{
                  display: "flex",
                  alignItems: "center",
                  color: branding.text_color,
                  cursor: "pointer",
                }}
              >
                <input
                  tabIndex={5}
                  id="rememberMe"
                  name="rememberMe"
                  type="checkbox"
                  defaultChecked={!!login.rememberMe}
                  style={{
                    marginRight: "0.5rem",
                    width: "1rem",
                    height: "1rem",
                    accentColor: branding.primary_color,
                  }}
                />
                {msg("rememberMe")}
              </label>
            )}
            {realm.resetPasswordAllowed && (
              <a
                tabIndex={6}
                href={url.loginResetCredentialsUrl}
                style={{
                  color: branding.secondary_color,
                  textDecoration: "none",
                }}
                onMouseOver={(e) => (e.currentTarget.style.textDecoration = "underline")}
                onMouseOut={(e) => (e.currentTarget.style.textDecoration = "none")}
              >
                {msg("doForgotPassword")}
              </a>
            )}
          </div>

          {/* Submit Button */}
          <button
            tabIndex={7}
            disabled={isLoginButtonDisabled}
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
              cursor: isLoginButtonDisabled ? "not-allowed" : "pointer",
              opacity: isLoginButtonDisabled ? 0.6 : 1,
              transition: "opacity 0.2s, transform 0.1s",
            }}
            onMouseOver={(e) => {
              if (!isLoginButtonDisabled) {
                e.currentTarget.style.opacity = "0.9";
              }
            }}
            onMouseOut={(e) => {
              if (!isLoginButtonDisabled) {
                e.currentTarget.style.opacity = "1";
              }
            }}
            onMouseDown={(e) => {
              if (!isLoginButtonDisabled) {
                e.currentTarget.style.transform = "scale(0.98)";
              }
            }}
            onMouseUp={(e) => {
              e.currentTarget.style.transform = "scale(1)";
            }}
          >
            {msgStr("doLogIn")}
          </button>

          {/* Try Another Way */}
          {auth?.showTryAnotherWayLink && (
            <div style={{ marginTop: "1rem", textAlign: "center" }}>
              <a
                href={url.loginRestartFlowUrl}
                style={{
                  color: branding.secondary_color,
                  fontSize: "0.875rem",
                  textDecoration: "none",
                }}
                onMouseOver={(e) => (e.currentTarget.style.textDecoration = "underline")}
                onMouseOut={(e) => (e.currentTarget.style.textDecoration = "none")}
              >
                {msg("doTryAnotherWay")}
              </a>
            </div>
          )}
        </form>

        {/* Social Providers */}
        {social?.providers && social.providers.length > 0 && (
          <div style={{ marginTop: "1.5rem" }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                marginBottom: "1rem",
              }}
            >
              <div
                style={{
                  flex: 1,
                  height: "1px",
                  backgroundColor: "#e5e7eb",
                }}
              />
              <span
                style={{
                  padding: "0 1rem",
                  color: "#9ca3af",
                  fontSize: "0.875rem",
                }}
              >
                {msgStr("identity-provider-login-label") || "Or continue with"}
              </span>
              <div
                style={{
                  flex: 1,
                  height: "1px",
                  backgroundColor: "#e5e7eb",
                }}
              />
            </div>
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "0.75rem",
              }}
            >
              {social.providers.map((provider) => (
                <a
                  key={provider.alias}
                  href={provider.loginUrl}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    gap: "0.5rem",
                    padding: "0.75rem 1rem",
                    border: "1px solid #e5e7eb",
                    borderRadius: "0.75rem",
                    backgroundColor: "#fff",
                    color: branding.text_color,
                    textDecoration: "none",
                    fontSize: "0.9375rem",
                    fontWeight: 500,
                    transition: "background-color 0.2s",
                  }}
                  onMouseOver={(e) => {
                    e.currentTarget.style.backgroundColor = "#f9fafb";
                  }}
                  onMouseOut={(e) => {
                    e.currentTarget.style.backgroundColor = "#fff";
                  }}
                >
                  {provider.displayName}
                </a>
              ))}
            </div>
          </div>
        )}

        {/* Registration Link */}
        {realm.password && realm.registrationAllowed && !registrationDisabled && branding.allow_registration && (
          <div
            style={{
              marginTop: "1.5rem",
              textAlign: "center",
              fontSize: "0.875rem",
              color: branding.text_color,
            }}
          >
            {msg("noAccount")}{" "}
            <a
              tabIndex={8}
              href={url.registrationUrl}
              style={{
                color: branding.primary_color,
                fontWeight: 600,
                textDecoration: "none",
              }}
              onMouseOver={(e) => (e.currentTarget.style.textDecoration = "underline")}
              onMouseOut={(e) => (e.currentTarget.style.textDecoration = "none")}
            >
              {msg("doRegister")}
            </a>
          </div>
        )}
      </div>
    </div>
  );
}
