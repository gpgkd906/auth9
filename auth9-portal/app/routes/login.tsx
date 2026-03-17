import type { MetaFunction, ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { redirect, Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState, useCallback } from "react";
import { getBrandMark } from "~/components/auth/AuthBrandPanel";
import { AuthMethodStack } from "~/components/auth/AuthMethodStack";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { useI18n } from "~/i18n";
import { translate } from "~/i18n/translate";
import { mapApiError, mapOAuthError } from "~/lib/error-messages";
import { LockClosedIcon } from "@radix-ui/react-icons";
import { resolveLocale } from "~/services/locale.server";
import { commitSession, serializeOAuthState } from "~/services/session.server";
import { enterpriseSsoApi, hostedLoginApi, publicBrandingApi, type BrandingConfig } from "~/services/api";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";
import type { AppLocale } from "~/i18n";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.login.metaTitle");
};

function mapLocaleToKeycloak(locale: AppLocale): string {
  if (locale.startsWith("en")) return "en";
  return locale;
}

async function generatePkce() {
  const verifierBytes = crypto.getRandomValues(new Uint8Array(32));
  const codeVerifier = Buffer.from(verifierBytes).toString("base64url");
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(codeVerifier)
  );
  const codeChallenge = Buffer.from(digest).toString("base64url");
  return { codeVerifier, codeChallenge };
}

async function buildAuthorizeParams(requestUrl: URL, locale: AppLocale) {
  const corePublicUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const portalUrl = process.env.AUTH9_PORTAL_URL || requestUrl.origin;
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  const redirectUri = `${portalUrl}/auth/callback`;

  const inviteToken = requestUrl.searchParams.get("invite_token");
  const statePayload = inviteToken
    ? JSON.stringify({ nonce: crypto.randomUUID(), invite_token: inviteToken })
    : crypto.randomUUID();
  const state = Buffer.from(typeof statePayload === "string" ? statePayload : statePayload).toString("base64url");

  const { codeVerifier, codeChallenge } = await generatePkce();

  return {
    corePublicUrl,
    response_type: "code",
    client_id: clientId,
    redirect_uri: redirectUri,
    scope: "openid email profile",
    state,
    nonce: crypto.randomUUID(),
    ui_locales: mapLocaleToKeycloak(locale),
    code_challenge: codeChallenge,
    code_challenge_method: "S256" as const,
    codeVerifier,
  };
}

// ==================== Login Mode Rollout ====================

/**
 * Deterministic hash for percentage-based login mode rollout.
 * Uses client IP + User-Agent to ensure the same visitor consistently
 * gets the same experience across page loads.
 */
function simpleHash(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

function shouldUseHostedLogin(request: Request, percentage: number): boolean {
  if (percentage >= 100) return true;
  if (percentage <= 0) return false;
  const ip = request.headers.get("x-forwarded-for")?.split(",")[0]?.trim() || "";
  const ua = request.headers.get("user-agent") || "";
  return (simpleHash(ip + ua) % 100) < percentage;
}

async function redirectToOidcLogin(request: Request) {
  const url = new URL(request.url);
  const locale = await resolveLocale(request);
  const auth = await buildAuthorizeParams(url, locale as AppLocale);
  const authorizeUrl = `${auth.corePublicUrl}/api/v1/auth/authorize?` +
    new URLSearchParams({
      response_type: auth.response_type,
      client_id: auth.client_id,
      redirect_uri: auth.redirect_uri,
      scope: auth.scope,
      state: auth.state,
      nonce: auth.nonce,
      ui_locales: auth.ui_locales,
      code_challenge: auth.code_challenge,
      code_challenge_method: auth.code_challenge_method,
    }).toString();
  const oauthCookie = await serializeOAuthState(auth.state, auth.codeVerifier);
  return redirect(authorizeUrl, { headers: { "Set-Cookie": oauthCookie } });
}

export async function loader({ request }: LoaderFunctionArgs) {
  // Login mode rollout: "hosted" (default) | "oidc" | "percentage"
  const loginMode = process.env.LOGIN_MODE || "hosted";

  if (loginMode === "oidc") {
    return redirectToOidcLogin(request);
  }

  if (loginMode === "percentage") {
    const pct = parseInt(process.env.LOGIN_ROLLOUT_PCT || "100", 10);
    if (!shouldUseHostedLogin(request, pct)) {
      return redirectToOidcLogin(request);
    }
  }

  const url = new URL(request.url);
  const locale = await resolveLocale(request);
  const error = url.searchParams.get("error");
  const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to default Portal-owned branding.
  }

  return { error, apiBaseUrl, locale, branding };
}

export async function action({ request }: ActionFunctionArgs) {
  const url = new URL(request.url);
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  // Handle passkey token storage
  if (intent === "passkey-login") {
    const accessToken = formData.get("accessToken") as string;
    const expiresIn = parseInt(formData.get("expiresIn") as string || "3600", 10);

    if (!accessToken) {
      return { error: translate(locale, "auth.login.missingAccessToken") };
    }

    const session = {
      accessToken,
      identityAccessToken: accessToken,
      refreshToken: undefined,
      idToken: undefined,
      expiresAt: Date.now() + (expiresIn * 1000),
      identityExpiresAt: Date.now() + (expiresIn * 1000),
    };

    return redirect("/tenant/select", {
      headers: {
        "Set-Cookie": await commitSession(session),
      },
    });
  }

  if (intent === "sso-login") {
    const email = String(formData.get("email") || "").trim();
    if (!email) {
      return { error: translate(locale, "auth.login.ssoEmailRequired") };
    }

    const auth = await buildAuthorizeParams(url, locale);
    try {
      const result = await enterpriseSsoApi.discover({ email }, auth);
      const oauthCookie = await serializeOAuthState(auth.state, auth.codeVerifier);
      return redirect(result.data.authorize_url, {
        headers: { "Set-Cookie": oauthCookie },
      });
    } catch (error) {
      const message = mapApiError(error, locale);
      return { error: message };
    }
  }

  if (intent === "password-login") {
    const email = String(formData.get("email") || "").trim();
    const password = String(formData.get("password") || "").trim();

    if (!email || !password) {
      return { error: translate(locale, "auth.login.credentialsRequired") };
    }

    try {
      const result = await hostedLoginApi.passwordLogin(email, password);
      const session = {
        accessToken: result.access_token,
        identityAccessToken: result.access_token,
        refreshToken: undefined,
        idToken: undefined,
        expiresAt: Date.now() + result.expires_in * 1000,
        identityExpiresAt: Date.now() + result.expires_in * 1000,
      };
      return redirect("/tenant/select", {
        headers: {
          "Set-Cookie": await commitSession(session),
        },
      });
    } catch (error) {
      const message = mapApiError(error, locale);
      return { error: message };
    }
  }

  return { error: translate(locale, "auth.login.invalidAction") };
}

// ==================== Base64URL Helpers ====================

function arrayBufferToBase64url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function base64urlToArrayBuffer(base64url: string): ArrayBuffer {
  const base64 = base64url.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
  const binary = atob(padded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

/**
 * Convert webauthn-rs RequestChallengeResponse to browser-compatible PublicKeyCredentialRequestOptions.
 */
function toRequestOptions(publicKey: Record<string, unknown>): PublicKeyCredentialRequestOptions {
  const challenge = base64urlToArrayBuffer(publicKey.challenge as string);

  const allowCredentials = ((publicKey.allowCredentials as Array<Record<string, unknown>>) || []).map(
    (cred) => ({
      id: base64urlToArrayBuffer(cred.id as string),
      type: cred.type as PublicKeyCredentialType,
      transports: cred.transports as AuthenticatorTransport[] | undefined,
    })
  );

  return {
    ...publicKey,
    challenge,
    allowCredentials: allowCredentials.length > 0 ? allowCredentials : undefined,
  } as PublicKeyCredentialRequestOptions;
}

export default function Login() {
  const loaderData = (useLoaderData<typeof loader>() ?? {}) as {
    error: string | null;
    apiBaseUrl: string;
    locale: string;
    branding: BrandingConfig;
  };
  const data = {
    error: loaderData.error ?? null,
    apiBaseUrl: loaderData.apiBaseUrl ?? "http://localhost:8080",
    locale: loaderData.locale ?? "zh-CN",
    branding: { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData.branding ?? {}) },
  };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const { t } = useI18n();
  const [passwordExpanded, setPasswordExpanded] = useState(false);

  const [authenticating, setAuthenticating] = useState(false);
  const [passkeyError, setPasskeyError] = useState<string | null>(null);

  const handlePasskeyLogin = useCallback(async () => {
    setPasskeyError(null);
    setAuthenticating(true);

    try {
      // 1. Get authentication challenge from backend
      const startResponse = await fetch(
        `${data.apiBaseUrl}/api/v1/auth/webauthn/authenticate/start`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
        }
      );
      if (!startResponse.ok) {
        const err = await startResponse.json().catch(() => ({ message: t("auth.login.authStartFailed") }));
        throw new Error(err.message || t("auth.login.authStartFailed"));
      }
      const startResult = await startResponse.json();
      const { challenge_id, public_key } = startResult;

      // 2. Convert to browser-compatible format and call WebAuthn API
      // Backend returns nested structure: { publicKey: { challenge, rpId, ... } }
      const publicKeyData = (public_key as { publicKey: Record<string, unknown> }).publicKey;
      const options = toRequestOptions(publicKeyData);
      const credential = await navigator.credentials.get({ publicKey: options });

      if (!credential) {
        throw new Error(t("auth.login.authCancelled"));
      }

      // 3. Send result to backend and get token
      const pkCred = credential as PublicKeyCredential;
      const assertion = pkCred.response as AuthenticatorAssertionResponse;
      const completeResponse = await fetch(
        `${data.apiBaseUrl}/api/v1/auth/webauthn/authenticate/complete`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            challenge_id,
            credential: {
              id: pkCred.id,
              rawId: arrayBufferToBase64url(pkCred.rawId),
              type: pkCred.type,
              response: {
                authenticatorData: arrayBufferToBase64url(assertion.authenticatorData),
                clientDataJSON: arrayBufferToBase64url(assertion.clientDataJSON),
                signature: arrayBufferToBase64url(assertion.signature),
                userHandle: assertion.userHandle
                  ? arrayBufferToBase64url(assertion.userHandle)
                  : undefined,
              },
            },
          }),
        }
      );
      if (!completeResponse.ok) {
        const err = await completeResponse.json().catch(() => ({ message: t("auth.login.authFailed") }));
        throw new Error(err.message || t("auth.login.authFailed"));
      }
      const tokenResult = await completeResponse.json();

      // 4. Store token via server action (form submit)
      const form = document.createElement("form");
      form.method = "POST";
      form.style.display = "none";

      const intentInput = document.createElement("input");
      intentInput.name = "intent";
      intentInput.value = "passkey-login";
      form.appendChild(intentInput);

      const tokenInput = document.createElement("input");
      tokenInput.name = "accessToken";
      tokenInput.value = tokenResult.access_token;
      form.appendChild(tokenInput);

      const expiresInput = document.createElement("input");
      expiresInput.name = "expiresIn";
      expiresInput.value = String(tokenResult.expires_in);
      form.appendChild(expiresInput);

      document.body.appendChild(form);
      form.submit();
    } catch (error) {
      if (error instanceof DOMException && error.name === "NotAllowedError") {
        setPasskeyError(t("auth.login.passkeyCancelled"));
      } else {
        const message = error instanceof Error ? error.message : t("auth.login.authFailed");
        setPasskeyError(message);
      }
      setAuthenticating(false);
    }
  }, [data.apiBaseUrl, t]);

  return (
    <AuthPageShell
      branding={data.branding}
      panelEyebrow={t("auth.shared.hostedEyebrow")}
      panelTitle={t("auth.shared.hostedTitle")}
      panelDescription={t("auth.shared.hostedDescription")}
    >
      <Card className="auth-form-card w-full max-w-xl animate-fade-in-up">
          <CardHeader className="text-center">
            {data.branding.logo_url ? (
              <img
                src={data.branding.logo_url}
                alt={data.branding.company_name || "Auth9"}
                className="mx-auto mb-4 h-14 w-14 rounded-2xl border border-black/5 bg-white/90 object-contain p-2"
                referrerPolicy="no-referrer"
              />
            ) : (
              <div className="logo-icon mx-auto mb-4">
                {getBrandMark(data.branding.company_name || "Auth9")}
              </div>
            )}
            <CardTitle className="text-2xl">
              {data.error ? t("auth.login.failedTitle") : t("auth.login.title")}
            </CardTitle>
            <CardDescription>
              {data.error
                ? mapOAuthError(data.error, data.locale as AppLocale)
                : t("auth.login.chooseMethod")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <AuthMethodStack>
                <Form method="post" action="/login">
                  <input type="hidden" name="intent" value="sso-login" />
                  <Input
                    type="email"
                    name="email"
                    required
                    placeholder={t("common.placeholders.companyEmail")}
                    className="mb-3"
                  />
                  <Button type="submit" className="w-full" disabled={isSubmitting || authenticating}>
                    {isSubmitting ? t("auth.login.ssoFinding") : t("auth.login.ssoButton")}
                  </Button>
                </Form>

                <Button
                  type="button"
                  variant="outline"
                  className="w-full justify-between"
                  onClick={() => setPasswordExpanded((current) => !current)}
                  disabled={isSubmitting || authenticating}
                >
                  <span>{t("auth.login.passwordButton")}</span>
                  <span className="text-xs text-[var(--text-tertiary)]">
                    {passwordExpanded ? t("auth.login.passwordHideDetails") : t("auth.login.passwordRevealDetails")}
                  </span>
                </Button>

                {passwordExpanded ? (
                  <div className="rounded-2xl border border-[var(--glass-border-subtle)] bg-white/70 p-4 text-left dark:bg-white/6">
                    <p className="text-sm font-medium text-[var(--text-primary)]">
                      {t("auth.login.passwordFallbackTitle")}
                    </p>
                    <p className="mt-2 text-sm leading-6 text-[var(--text-secondary)]">
                      {t("auth.login.passwordFallbackDescription")}
                    </p>
                    <Form method="post" action="/login" className="mt-4">
                      <input type="hidden" name="intent" value="password-login" />
                      <Button type="submit" variant="outline" className="w-full" disabled={isSubmitting || authenticating}>
                        {isSubmitting ? t("auth.login.redirecting") : t("auth.login.passwordFallbackContinue")}
                      </Button>
                    </Form>
                  </div>
                ) : null}

                {data.branding.email_otp_enabled && (
                  <Link to="/auth/email-otp">
                    <Button variant="outline" className="w-full" disabled={isSubmitting || authenticating}>
                      {t("auth.login.emailOtpButton")}
                    </Button>
                  </Link>
                )}

                <Button
                  variant="outline"
                  className="w-full"
                  onClick={handlePasskeyLogin}
                  disabled={authenticating || isSubmitting}
                >
                  <LockClosedIcon className="h-4 w-4 mr-2" />
                  {authenticating ? t("auth.login.verifying") : t("auth.login.passkeyButton")}
                </Button>

                <div className="rounded-2xl border border-dashed border-[var(--glass-border-subtle)] px-4 py-3 text-left">
                  <p className="text-xs font-semibold uppercase tracking-[0.18em] text-[var(--text-tertiary)]">
                    {t("auth.login.futureMethodsEyebrow")}
                  </p>
                  <p className="mt-2 text-sm leading-6 text-[var(--text-secondary)]">
                    {t("auth.login.futureMethodsDescription")}
                  </p>
                </div>
              </AuthMethodStack>

              {actionData?.error && (
                <p className="text-sm text-[var(--accent-red)]">{actionData.error}</p>
              )}

              <div className="flex items-center gap-4 my-1">
                <span className="flex-1 h-px bg-[var(--glass-border-subtle)]" />
                <span className="text-xs uppercase text-[var(--text-tertiary)] tracking-wide">{t("auth.login.or")}</span>
                <span className="flex-1 h-px bg-[var(--glass-border-subtle)]" />
              </div>

              {/* Error Messages */}
              {passkeyError && (
                <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md text-center">
                  {passkeyError}
                </div>
              )}

              <div className="flex items-center justify-between text-sm text-[var(--text-tertiary)] pt-1">
                <Link to="/forgot-password" className="hover:text-[var(--text-primary)] underline-offset-4 hover:underline">
                  {t("auth.login.forgotPassword")}
                </Link>
                {data.branding.allow_registration ? (
                  <Link to="/register" className="hover:text-[var(--text-primary)] underline-offset-4 hover:underline">
                    {t("auth.login.createAccount")}
                  </Link>
                ) : (
                  <span />
                )}
              </div>
            </div>
          </CardContent>
      </Card>
    </AuthPageShell>
  );
}
