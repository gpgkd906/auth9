import type { MetaFunction, ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { redirect, Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { useState, useCallback } from "react";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { ThemeToggle } from "~/components/ThemeToggle";
import { LockClosedIcon } from "@radix-ui/react-icons";
import { commitSession, serializeOAuthState } from "~/services/session.server";
import { enterpriseSsoApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Sign In - Auth9" }];
};

function buildAuthorizeParams(requestUrl: URL) {
  const corePublicUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const portalUrl = requestUrl.origin;
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  const redirectUri = `${portalUrl}/auth/callback`;

  const inviteToken = requestUrl.searchParams.get("invite_token");
  const statePayload = inviteToken
    ? JSON.stringify({ nonce: crypto.randomUUID(), invite_token: inviteToken })
    : crypto.randomUUID();
  const state = Buffer.from(typeof statePayload === "string" ? statePayload : statePayload).toString("base64url");

  return {
    corePublicUrl,
    response_type: "code",
    client_id: clientId,
    redirect_uri: redirectUri,
    scope: "openid email profile",
    state,
    nonce: crypto.randomUUID(),
  };
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const error = url.searchParams.get("error");
  const showPasskey = url.searchParams.get("passkey") === "true";

  const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";

  if (error) {
    return { error, showPasskey: true, apiBaseUrl };
  }

  // Default behavior: auto-redirect to SSO when no error/passkey params
  if (!showPasskey) {
    const auth = buildAuthorizeParams(url);
    const authorizeUrl = new URL(`${auth.corePublicUrl}/api/v1/auth/authorize`);
    authorizeUrl.searchParams.set("response_type", auth.response_type);
    authorizeUrl.searchParams.set("client_id", auth.client_id);
    authorizeUrl.searchParams.set("redirect_uri", auth.redirect_uri);
    authorizeUrl.searchParams.set("scope", auth.scope);
    authorizeUrl.searchParams.set("state", auth.state);
    authorizeUrl.searchParams.set("nonce", auth.nonce);

    const oauthCookie = await serializeOAuthState(auth.state);
    throw redirect(authorizeUrl.toString(), {
      headers: { "Set-Cookie": oauthCookie },
    });
  }

  return { error: null, showPasskey, apiBaseUrl };
}

export async function action({ request }: ActionFunctionArgs) {
  const url = new URL(request.url);
  const formData = await request.formData();
  const intent = formData.get("intent");

  // Handle passkey token storage
  if (intent === "passkey-login") {
    const accessToken = formData.get("accessToken") as string;
    const expiresIn = parseInt(formData.get("expiresIn") as string || "3600", 10);

    if (!accessToken) {
      return { error: "Missing access token" };
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
      return { error: "Email is required for enterprise SSO discovery" };
    }

    const auth = buildAuthorizeParams(url);
    try {
      const result = await enterpriseSsoApi.discover({ email }, auth);
      const oauthCookie = await serializeOAuthState(auth.state);
      return redirect(result.data.authorize_url, {
        headers: { "Set-Cookie": oauthCookie },
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : "Enterprise SSO discovery failed";
      return { error: message };
    }
  }

  if (intent === "password-login") {
    const auth = buildAuthorizeParams(url);
    const corePublicUrl = auth.corePublicUrl;
    const authorizeUrl = new URL(`${corePublicUrl}/api/v1/auth/authorize`);
    authorizeUrl.searchParams.set("response_type", auth.response_type);
    authorizeUrl.searchParams.set("client_id", auth.client_id);
    authorizeUrl.searchParams.set("redirect_uri", auth.redirect_uri);
    authorizeUrl.searchParams.set("scope", auth.scope);
    authorizeUrl.searchParams.set("state", auth.state);
    authorizeUrl.searchParams.set("nonce", auth.nonce);

    const oauthCookie = await serializeOAuthState(auth.state);
    return redirect(authorizeUrl.toString(), {
      headers: { "Set-Cookie": oauthCookie },
    });
  }

  return { error: "Invalid action" };
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
  const data = useLoaderData<typeof loader>() as { error: string | null; showPasskey: boolean; apiBaseUrl: string };
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

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
        const err = await startResponse.json().catch(() => ({ message: "Failed to start authentication" }));
        throw new Error(err.message || "Failed to start authentication");
      }
      const startResult = await startResponse.json();
      const { challenge_id, public_key } = startResult;

      // 2. Convert to browser-compatible format and call WebAuthn API
      // Backend returns nested structure: { publicKey: { challenge, rpId, ... } }
      const publicKeyData = (public_key as { publicKey: Record<string, unknown> }).publicKey;
      const options = toRequestOptions(publicKeyData);
      const credential = await navigator.credentials.get({ publicKey: options });

      if (!credential) {
        throw new Error("Authentication was cancelled");
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
        const err = await completeResponse.json().catch(() => ({ message: "Authentication failed" }));
        throw new Error(err.message || "Authentication failed");
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
        setPasskeyError("Authentication was cancelled or timed out.");
      } else {
        const message = error instanceof Error ? error.message : "Authentication failed";
        setPasskeyError(message);
      }
      setAuthenticating(false);
    }
  }, [data.apiBaseUrl]);

  return (
    <>
      {/* Theme Toggle */}
      <div className="fixed top-6 right-6 z-20">
        <ThemeToggle />
      </div>

      <div className="min-h-screen flex items-center justify-center px-6 relative">
        {/* Dynamic Background */}
        <div className="page-backdrop" />

        <Card className="w-full max-w-md relative z-10 animate-fade-in-up">
          <CardHeader className="text-center">
            <div className="logo-icon mx-auto mb-4">A9</div>
            <CardTitle className="text-2xl">
              {data.error ? "Sign In Failed" : "Sign In"}
            </CardTitle>
            <CardDescription>
              {data.error === "access_denied"
                ? "Access was denied. Please try again or contact your administrator."
                : data.error
                  ? `An error occurred during sign in: ${data.error}`
                  : "Choose how you want to sign in"}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {/* SSO Login Button */}
              <Form method="post">
                <input type="hidden" name="intent" value="sso-login" />
                <Input
                  type="email"
                  name="email"
                  required
                  placeholder="you@company.com"
                  className="mb-3"
                />
                <Button type="submit" className="w-full" disabled={isSubmitting || authenticating}>
                  {isSubmitting ? "Finding your SSO..." : "Continue with Enterprise SSO"}
                </Button>
              </Form>

              {data.error && (
                <p className="text-sm text-[var(--accent-red)]">{data.error}</p>
              )}
              {actionData?.error && (
                <p className="text-sm text-[var(--accent-red)]">{actionData.error}</p>
              )}

              {/* Divider */}
              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-[var(--card-bg)] px-2 text-[var(--text-tertiary)]">or</span>
                </div>
              </div>

              {/* Password Login Button */}
              <Form method="post">
                <input type="hidden" name="intent" value="password-login" />
                <Button type="submit" variant="outline" className="w-full" disabled={isSubmitting || authenticating}>
                  {isSubmitting ? "Redirecting..." : "Sign in with password"}
                </Button>
              </Form>

              {/* Passkey Login Button */}
              <Button
                variant="outline"
                className="w-full"
                onClick={handlePasskeyLogin}
                disabled={authenticating || isSubmitting}
              >
                <LockClosedIcon className="h-4 w-4 mr-2" />
                {authenticating ? "Verifying..." : "Sign in with passkey"}
              </Button>

              {/* Error Messages */}
              {passkeyError && (
                <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md text-center">
                  {passkeyError}
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      </div>
    </>
  );
}
