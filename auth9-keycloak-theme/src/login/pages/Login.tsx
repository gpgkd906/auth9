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
import { SocialProviders } from "../components/SocialProviderButton";

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

  // Get appropriate username label
  const usernameLabel = !realm.loginWithEmailAllowed
    ? msgStr("username")
    : !realm.registrationEmailAsUsername
      ? msgStr("usernameOrEmail")
      : msgStr("email");

  return (
    <PageLayout>
      <GlassCard>
        {/* Header with Logo */}
        <div className="login-header">
          <Logo
            logoUrl={branding.logo_url}
            companyName={branding.company_name}
            fallbackText={realm.displayName || msgStr("loginTitleHtml", realm.displayNameHtml)}
          />
        </div>

        {/* Error Messages */}
        {messagesPerField.existsError("username", "password") && (
          <GlassAlert variant="error">
            {messagesPerField.getFirstError("username", "password")}
          </GlassAlert>
        )}

        <form
          id="kc-form-login"
          onSubmit={onSubmit}
          action={url.loginAction}
          method="post"
        >
          {/* Username/Email */}
          {!usernameHidden && (
            <GlassInput
              tabIndex={2}
              id="username"
              name="username"
              defaultValue={login.username ?? ""}
              type="text"
              autoFocus
              autoComplete="username"
              aria-invalid={messagesPerField.existsError("username")}
              label={usernameLabel}
            />
          )}

          {/* Password */}
          <GlassInput
            tabIndex={3}
            id="password"
            name="password"
            type="password"
            autoComplete="current-password"
            aria-invalid={messagesPerField.existsError("password")}
            label={msgStr("password")}
          />

          {/* Remember Me & Forgot Password */}
          <div className="form-options">
            {realm.rememberMe && !usernameHidden && (
              <label className="form-checkbox">
                <input
                  tabIndex={5}
                  id="rememberMe"
                  name="rememberMe"
                  type="checkbox"
                  defaultChecked={!!login.rememberMe}
                />
                {msg("rememberMe")}
              </label>
            )}
            {realm.resetPasswordAllowed && (
              <a
                tabIndex={6}
                href={url.loginResetCredentialsUrl}
                className="form-link"
              >
                {msg("doForgotPassword")}
              </a>
            )}
          </div>

          {/* Submit Button */}
          <GlassButton
            tabIndex={7}
            disabled={isLoginButtonDisabled}
            type="submit"
            name="login"
            variant="primary"
            loading={isLoginButtonDisabled}
          >
            {msgStr("doLogIn")}
          </GlassButton>

          {/* Try Another Way */}
          {auth?.showTryAnotherWayLink && (
            <div className="login-footer" style={{ marginTop: "16px" }}>
              <a href={url.loginRestartFlowUrl} className="form-link--secondary">
                {msg("doTryAnotherWay")}
              </a>
            </div>
          )}
        </form>

        {/* Social Providers */}
        <SocialProviders
          providers={social?.providers ?? []}
          dividerText={msgStr("identity-provider-login-label") || "Or continue with"}
        />

        {/* Registration Link */}
        {realm.password &&
          realm.registrationAllowed &&
          !registrationDisabled &&
          branding.allow_registration && (
            <div className="login-footer">
              {msg("noAccount")}{" "}
              <a tabIndex={8} href={url.registrationUrl}>
                {msg("doRegister")}
              </a>
            </div>
          )}
      </GlassCard>
    </PageLayout>
  );
}
