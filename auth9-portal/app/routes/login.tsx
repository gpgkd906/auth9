import type { MetaFunction, ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { redirect, Form, useLoaderData, useNavigation } from "react-router";
import { useState, useCallback } from "react";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { ThemeToggle } from "~/components/ThemeToggle";
import { LockClosedIcon } from "@radix-ui/react-icons";
import { commitSession } from "~/services/session.server";

export const meta: MetaFunction = () => {
  return [{ title: "Sign In - Auth9" }];
};

function buildAuthorizeUrl(requestUrl: URL) {
  const corePublicUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const portalUrl = process.env.AUTH9_PORTAL_URL || requestUrl.origin;
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  const redirectUri = `${portalUrl}/auth/callback`;

  const state = crypto.randomUUID();

  const authorizeUrl = new URL(`${corePublicUrl}/api/v1/auth/authorize`);
  authorizeUrl.searchParams.set("response_type", "code");
  authorizeUrl.searchParams.set("client_id", clientId);
  authorizeUrl.searchParams.set("redirect_uri", redirectUri);
  authorizeUrl.searchParams.set("scope", "openid email profile");
  authorizeUrl.searchParams.set("state", state);

  return authorizeUrl.toString();
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const error = url.searchParams.get("error");
  const showPasskey = url.searchParams.get("passkey") === "true";

  const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";

  if (error) {
    return { error, showPasskey: true, apiBaseUrl };
  }

  // If passkey mode requested, show the login page with passkey option
  if (showPasskey) {
    return { error: null, showPasskey: true, apiBaseUrl };
  }

  // Default: auto-redirect to SSO
  const authorizeUrl = buildAuthorizeUrl(url);
  return redirect(authorizeUrl);
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
      refreshToken: undefined,
      idToken: undefined,
      expiresAt: Date.now() + (expiresIn * 1000),
    };

    return redirect("/dashboard", {
      headers: {
        "Set-Cookie": await commitSession(session),
      },
    });
  }

  // Default: redirect to SSO
  const authorizeUrl = buildAuthorizeUrl(url);
  return redirect(authorizeUrl);
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
                <Button type="submit" className="w-full" disabled={isSubmitting || authenticating}>
                  {isSubmitting ? "Redirecting..." : "Sign in with SSO"}
                </Button>
              </Form>

              {/* Divider */}
              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-[var(--card-bg)] px-2 text-[var(--text-tertiary)]">or</span>
                </div>
              </div>

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
