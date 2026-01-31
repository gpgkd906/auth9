import { useState, type FormEventHandler } from "react";
import type { PageProps } from "keycloakify/login/pages/PageProps";
import type { KcContext } from "../KcContext";
import type { I18n } from "../i18n";
import { useBrandingContext } from "../components/BrandingProvider";
import { getKcClsx } from "keycloakify/login/lib/kcClsx";
import UserProfileFormFields from "keycloakify/login/UserProfileFormFields";

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
      <style>{`
        .auth9-register-form input[type="text"],
        .auth9-register-form input[type="email"],
        .auth9-register-form input[type="password"] {
          width: 100%;
          padding: 0.75rem 1rem;
          font-size: 1rem;
          border: 1px solid ${branding.secondary_color}40;
          border-radius: 0.75rem;
          outline: none;
          transition: border-color 0.2s, box-shadow 0.2s;
          background-color: #fff;
          box-sizing: border-box;
        }
        .auth9-register-form input[type="text"]:focus,
        .auth9-register-form input[type="email"]:focus,
        .auth9-register-form input[type="password"]:focus {
          border-color: ${branding.primary_color};
          box-shadow: 0 0 0 3px ${branding.primary_color}20;
        }
        .auth9-register-form label {
          display: block;
          font-size: 0.875rem;
          font-weight: 500;
          margin-bottom: 0.5rem;
          color: ${branding.text_color};
        }
        .auth9-register-form .pf-v5-c-form__group,
        .auth9-register-form .pf-c-form__group {
          margin-bottom: 1.25rem;
        }
        .auth9-register-form .pf-v5-c-form__helper-text,
        .auth9-register-form .pf-c-form__helper-text {
          color: #dc2626;
          font-size: 0.8125rem;
          margin-top: 0.25rem;
        }
      `}</style>
      <div
        style={{
          width: "100%",
          maxWidth: "480px",
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
              {realm.displayName || msg("registerTitle")}
            </h1>
          )}
          <p
            style={{
              marginTop: "0.75rem",
              fontSize: "0.9375rem",
              color: "#6b7280",
            }}
          >
            {msgStr("registerTitle")}
          </p>
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
            <div style={{ marginBottom: "1.5rem" }}>
              <div className="g-recaptcha" data-sitekey={recaptchaSiteKey} />
            </div>
          )}

          {/* Submit Button */}
          <button
            disabled={isFormSubmitting || !isFormSubmittable}
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
              cursor: isFormSubmitting || !isFormSubmittable ? "not-allowed" : "pointer",
              opacity: isFormSubmitting || !isFormSubmittable ? 0.6 : 1,
              transition: "opacity 0.2s, transform 0.1s",
              marginTop: "0.5rem",
            }}
            onMouseOver={(e) => {
              if (!isFormSubmitting && isFormSubmittable) {
                e.currentTarget.style.opacity = "0.9";
              }
            }}
            onMouseOut={(e) => {
              if (!isFormSubmitting && isFormSubmittable) {
                e.currentTarget.style.opacity = "1";
              }
            }}
          >
            {msgStr("doRegister")}
          </button>
        </form>

        {/* Back to Login */}
        <div
          style={{
            marginTop: "1.5rem",
            textAlign: "center",
            fontSize: "0.875rem",
            color: branding.text_color,
          }}
        >
          {msgStr("backToLogin") || "Already have an account?"}{" "}
          <a
            href={url.loginUrl}
            style={{
              color: branding.primary_color,
              fontWeight: 600,
              textDecoration: "none",
            }}
            onMouseOver={(e) => (e.currentTarget.style.textDecoration = "underline")}
            onMouseOut={(e) => (e.currentTarget.style.textDecoration = "none")}
          >
            {msg("doLogIn")}
          </a>
        </div>
      </div>
    </div>
  );
}
