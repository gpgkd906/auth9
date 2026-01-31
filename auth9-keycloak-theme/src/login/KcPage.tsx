import { Suspense, lazy } from "react";
import type { KcContext } from "./KcContext";
import { useI18n } from "./i18n";
import { BrandingProvider } from "./components/BrandingProvider";
import DefaultPage from "keycloakify/login/DefaultPage";
import Template from "keycloakify/login/Template";
import UserProfileFormFields from "keycloakify/login/UserProfileFormFields";

// Lazy load custom pages
const Login = lazy(() => import("./pages/Login"));
const Register = lazy(() => import("./pages/Register"));
const LoginResetPassword = lazy(() => import("./pages/LoginResetPassword"));
const LoginOtp = lazy(() => import("./pages/LoginOtp"));

/**
 * Default auth9 API URL - can be overridden via theme.properties
 */
const DEFAULT_API_URL = "http://localhost:8080";

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

  // Get auth9 API URL from theme properties
  const apiUrl = kcContext.properties?.auth9ApiUrl || DEFAULT_API_URL;

  return (
    <BrandingProvider apiUrl={apiUrl}>
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
