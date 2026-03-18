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
import { enterpriseSsoApi, hostedLoginApi, publicBrandingApi, identityProviderApi, type BrandingConfig } from "~/services/api";
import type { PublicSocialProvider } from "~/services/api/identity-provider";
import { DEFAULT_PUBLIC_BRANDING } from "~/services/api/branding";
import type { AppLocale } from "~/i18n";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.login.metaTitle");
};

function mapLocaleToOidc(locale: AppLocale): string {
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
    ui_locales: mapLocaleToOidc(locale),
    code_challenge: codeChallenge,
    code_challenge_method: "S256" as const,
    codeVerifier,
  };
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const locale = await resolveLocale(request);
  const error = url.searchParams.get("error");
  const loginChallenge = url.searchParams.get("login_challenge") || undefined;
  const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  let branding: BrandingConfig = DEFAULT_PUBLIC_BRANDING;

  try {
    const { data } = await publicBrandingApi.get(clientId);
    branding = { ...DEFAULT_PUBLIC_BRANDING, ...data };
  } catch {
    // Fall back to default Portal-owned branding.
  }

  let socialProviders: PublicSocialProvider[] = [];
  try {
    const { data } = await identityProviderApi.listEnabledPublic();
    socialProviders = data;
  } catch {
    // Social providers unavailable — continue without them.
  }

  return { error, apiBaseUrl, locale, branding, loginChallenge, socialProviders };
}

export async function action({ request }: ActionFunctionArgs) {
  const url = new URL(request.url);
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = formData.get("intent");

  // Handle social login: create login_challenge if needed, then redirect to social broker
  if (intent === "social-login") {
    const providerAlias = String(formData.get("providerAlias") || "").trim();
    let loginChallenge = formData.get("loginChallenge") as string | null;

    if (!providerAlias) {
      return { error: translate(locale, "auth.login.socialProviderRequired") };
    }

    // If no login_challenge, initiate OIDC authorize to get one
    if (!loginChallenge) {
      const auth = await buildAuthorizeParams(url, locale);
      const coreInternalUrl = process.env.AUTH9_CORE_URL || auth.corePublicUrl;
      const authorizeUrl = `${coreInternalUrl}/api/v1/auth/authorize?response_type=${auth.response_type}&client_id=${auth.client_id}&redirect_uri=${encodeURIComponent(auth.redirect_uri)}&scope=${encodeURIComponent(auth.scope)}&state=${auth.state}&code_challenge=${auth.code_challenge}&code_challenge_method=${auth.code_challenge_method}`;

      const authorizeResponse = await fetch(authorizeUrl, { redirect: "manual" });
      const location = authorizeResponse.headers.get("Location") || "";
      // Extract login_challenge from redirect: try login_challenge= first, then state=
      // (Keycloak-mode authorize redirects use state= as the login_challenge_id)
      const challengeMatch = location.match(/login_challenge=([^&]+)/) || location.match(/[?&]state=([^&]+)/);
      loginChallenge = challengeMatch ? challengeMatch[1] : null;

      if (!loginChallenge) {
        return { error: translate(locale, "auth.login.socialLoginFailed") };
      }

      // Store OAuth state for the callback to validate later
      const oauthCookie = await serializeOAuthState(auth.state, auth.codeVerifier);
      const corePublicUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
      const socialUrl = `${corePublicUrl}/api/v1/social-login/authorize/${encodeURIComponent(providerAlias)}?login_challenge=${encodeURIComponent(loginChallenge)}`;
      return redirect(socialUrl, { headers: { "Set-Cookie": oauthCookie } });
    }

    // login_challenge already exists, redirect directly
    const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
    const socialUrl = `${apiBaseUrl}/api/v1/social-login/authorize/${encodeURIComponent(providerAlias)}?login_challenge=${encodeURIComponent(loginChallenge)}`;
    return redirect(socialUrl);
  }

  // Handle passkey token storage
  if (intent === "passkey-login") {
    const accessToken = formData.get("accessToken") as string;
    const expiresIn = parseInt(formData.get("expiresIn") as string || "3600", 10);
    const loginChallenge = formData.get("loginChallenge") as string | null;

    if (!accessToken) {
      return { error: translate(locale, "auth.login.missingAccessToken") };
    }

    // If login_challenge is present, complete the OIDC authorization flow
    if (loginChallenge) {
      try {
        const result = await hostedLoginApi.authorizeComplete(loginChallenge, accessToken);
        return redirect(result.redirect_url);
      } catch (error) {
        const message = mapApiError(error, locale);
        return { error: message };
      }
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
    const loginChallenge = formData.get("loginChallenge") as string | null;

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
      const cookie = await commitSession(session);

      // Redirect to first pending action if any, otherwise tenant select
      if (result.pending_actions && result.pending_actions.length > 0) {
        const first = result.pending_actions[0];
        let actionUrl = first.redirect_url.includes("?")
          ? `${first.redirect_url}&action_id=${first.id}`
          : `${first.redirect_url}?action_id=${first.id}`;
        // Carry login_challenge through pending actions (MFA, etc.)
        if (loginChallenge) {
          actionUrl += `&login_challenge=${encodeURIComponent(loginChallenge)}`;
        }
        return redirect(actionUrl, { headers: { "Set-Cookie": cookie } });
      }

      // If login_challenge is present, complete the OIDC authorization flow
      if (loginChallenge && result.access_token) {
        try {
          const authResult = await hostedLoginApi.authorizeComplete(loginChallenge, result.access_token);
          return redirect(authResult.redirect_url);
        } catch (error) {
          const message = mapApiError(error, locale);
          return { error: message };
        }
      }

      return redirect("/tenant/select", { headers: { "Set-Cookie": cookie } });
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

function SocialProviderIcon({ providerType }: { providerType: string }) {
  switch (providerType.toLowerCase()) {
    case "google":
      return <span className="inline-block w-4 h-4 text-center font-semibold text-sm leading-4">G</span>;
    case "github":
      return <span className="inline-block w-4 h-4 text-center font-semibold text-sm leading-4">GH</span>;
    case "microsoft":
      return <span className="inline-block w-4 h-4 text-center font-semibold text-sm leading-4">MS</span>;
    default:
      return <span className="inline-block w-4 h-4 text-center font-semibold text-sm leading-4">{providerType.slice(0, 2).toUpperCase()}</span>;
  }
}

export default function Login() {
  const loaderData = (useLoaderData<typeof loader>() ?? {}) as {
    error: string | null;
    apiBaseUrl: string;
    locale: string;
    branding: BrandingConfig;
    socialProviders?: PublicSocialProvider[];
  };
  const data = {
    error: loaderData.error ?? null,
    apiBaseUrl: loaderData.apiBaseUrl ?? "http://localhost:8080",
    locale: loaderData.locale ?? "zh-CN",
    branding: { ...DEFAULT_PUBLIC_BRANDING, ...(loaderData.branding ?? {}) },
    loginChallenge: (loaderData as Record<string, unknown>).loginChallenge as string | undefined,
    socialProviders: loaderData.socialProviders ?? [],
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

      if (data.loginChallenge) {
        const challengeInput = document.createElement("input");
        challengeInput.name = "loginChallenge";
        challengeInput.value = data.loginChallenge;
        form.appendChild(challengeInput);
      }

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
                  {data.loginChallenge && (
                    <input type="hidden" name="loginChallenge" value={data.loginChallenge} />
                  )}
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
                      {data.loginChallenge && (
                        <input type="hidden" name="loginChallenge" value={data.loginChallenge} />
                      )}
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

                {data.socialProviders.length > 0 && (
                  <div className="space-y-2">
                    <div className="relative my-2">
                      <div className="absolute inset-0 flex items-center">
                        <span className="w-full border-t border-[var(--glass-border-subtle)]" />
                      </div>
                      <div className="relative flex justify-center text-xs uppercase">
                        <span className="bg-[var(--surface-elevated)] px-2 text-[var(--text-tertiary)]">
                          {t("auth.login.socialDivider")}
                        </span>
                      </div>
                    </div>
                    {data.socialProviders.map((provider) => (
                      <Form method="post" action="/login" key={provider.alias}>
                        <input type="hidden" name="intent" value="social-login" />
                        <input type="hidden" name="providerAlias" value={provider.alias} />
                        {data.loginChallenge && (
                          <input type="hidden" name="loginChallenge" value={data.loginChallenge} />
                        )}
                        <Button
                          type="submit"
                          variant="outline"
                          className="w-full"
                          disabled={isSubmitting || authenticating}
                        >
                          <SocialProviderIcon providerType={provider.provider_id} />
                          <span className="ml-2">
                            {provider.display_name || provider.alias}
                          </span>
                        </Button>
                      </Form>
                    ))}
                  </div>
                )}

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
