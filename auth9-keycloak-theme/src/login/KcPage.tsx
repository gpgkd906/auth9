import { Suspense, lazy } from "react";
import type { KcContext } from "./KcContext";
import { useI18n } from "./i18n";
import { BrandingProvider } from "./components/BrandingProvider";
import DefaultPage from "keycloakify/login/DefaultPage";
import Template from "keycloakify/login/Template";
import UserProfileFormFields from "keycloakify/login/UserProfileFormFields";

// Import CSS styles
import "./styles/index.css";

// Lazy load custom pages
const Login = lazy(() => import("./pages/Login"));
const Register = lazy(() => import("./pages/Register"));
const LoginResetPassword = lazy(() => import("./pages/LoginResetPassword"));
const LoginOtp = lazy(() => import("./pages/LoginOtp"));
const LoginConfigTotp = lazy(() => import("./pages/LoginConfigTotp"));
const SelectAuthenticator = lazy(() => import("./pages/SelectAuthenticator"));
const LoginResetOtp = lazy(() => import("./pages/LoginResetOtp"));
const Info = lazy(() => import("./pages/Info"));
const ErrorPage = lazy(() => import("./pages/Error"));
const LoginPageExpired = lazy(() => import("./pages/LoginPageExpired"));

/**
 * Local development fallback.
 * Production should resolve the API URL from theme properties or OAuth context.
 */
const LOCAL_DEFAULT_API_URL = "http://localhost:8080";

function isLoopbackHostname(hostname: string): boolean {
  return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
}

function isLoopbackUrl(url: string): boolean {
  try {
    return isLoopbackHostname(new URL(url).hostname);
  } catch {
    return false;
  }
}

function deriveApiUrlFromOAuthContext(): string | undefined {
  if (typeof window === "undefined") {
    return undefined;
  }

  try {
    const currentUrl = new URL(window.location.href);
    const redirectUri = currentUrl.searchParams.get("redirect_uri");
    if (!redirectUri) {
      return undefined;
    }

    return new URL(redirectUri).origin;
  } catch {
    return undefined;
  }
}

function resolveApiUrl(kcContext: KcContext): string {
  const configuredApiUrl = kcContext.properties?.auth9ApiUrl?.trim();
  if (configuredApiUrl) {
    if (!isLoopbackUrl(configuredApiUrl)) {
      return configuredApiUrl;
    }

    if (typeof window !== "undefined" && isLoopbackHostname(window.location.hostname)) {
      return configuredApiUrl;
    }
  }

  const inferredApiUrl = deriveApiUrlFromOAuthContext();
  if (inferredApiUrl) {
    return inferredApiUrl;
  }

  return configuredApiUrl || LOCAL_DEFAULT_API_URL;
}

/**
 * Loading fallback component
 */
function LoadingFallback() {
  return (
    <div
      style={{
        minHeight: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "#f5f5f7",
      }}
    >
      <div
        style={{
          color: "#9ca3af",
          fontSize: "0.875rem",
        }}
      >
        Loading...
      </div>
    </div>
  );
}

export default function KcPage(props: { kcContext: KcContext }) {
  const { kcContext } = props;
  const { i18n } = useI18n({ kcContext });

  // Prefer explicit theme config. If unavailable or left at localhost in a public deployment,
  // infer the API origin from the OAuth redirect_uri to avoid browser-side localhost fetches.
  const apiUrl = resolveApiUrl(kcContext);

  // Extract client_id for service-level branding
  const clientId = (kcContext as Record<string, unknown> & { client?: { clientId?: string } }).client?.clientId;

  return (
    <BrandingProvider apiUrl={apiUrl} clientId={clientId}>
      <Suspense fallback={<LoadingFallback />}>
        {(() => {
          switch (kcContext.pageId) {
            case "login.ftl":
              return (
                <Login
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "register.ftl":
              return (
                <Register
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "login-reset-password.ftl":
              return (
                <LoginResetPassword
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "login-otp.ftl":
              return (
                <LoginOtp
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "login-config-totp.ftl":
              return (
                <LoginConfigTotp
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "select-authenticator.ftl":
              return (
                <SelectAuthenticator
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "login-reset-otp.ftl":
              return (
                <LoginResetOtp
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "info.ftl":
              return (
                <Info
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "error.ftl":
              return (
                <ErrorPage
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            case "login-page-expired.ftl":
              return (
                <LoginPageExpired
                  kcContext={kcContext}
                  i18n={i18n}
                  doUseDefaultCss={false}
                  classes={{}}
                  Template={Template}
                />
              );

            default:
              // Use default Keycloakify page for other pages
              return (
                <DefaultPage
                  kcContext={kcContext}
                  i18n={i18n}
                  Template={Template}
                  doUseDefaultCss={true}
                  UserProfileFormFields={UserProfileFormFields}
                  doMakeUserConfirmPassword={true}
                  classes={{}}
                />
              );
          }
        })()}
      </Suspense>
    </BrandingProvider>
  );
}
