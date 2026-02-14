import type { LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useRevalidator } from "react-router";
import { useState, useCallback } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { webauthnApi, type WebAuthnCredential } from "~/services/api";
import { LockClosedIcon, TrashIcon, PlusIcon } from "@radix-ui/react-icons";
import { getAccessToken } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  try {
    const accessToken = await getAccessToken(request);
    const response = await webauthnApi.listPasskeys(accessToken || "");
    return { passkeys: response.data, accessToken: accessToken || "", apiBaseUrl };
  } catch {
    return { passkeys: [], accessToken: "", apiBaseUrl, error: "Failed to load passkeys" };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request) || "";

  try {
    if (intent === "delete") {
      const credentialId = formData.get("credentialId") as string;
      await webauthnApi.deletePasskey(credentialId, accessToken);
      return { success: true, message: "Passkey deleted" };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function getCredentialTypeLabel(type: string) {
  switch (type) {
    case "webauthn-passwordless":
      return "Passwordless";
    case "webauthn":
      return "Two-Factor";
    default:
      return type;
  }
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
 * Convert webauthn-rs CreationChallengeResponse to browser-compatible PublicKeyCredentialCreationOptions.
 * webauthn-rs serializes binary fields as base64url strings; the browser API needs ArrayBuffers.
 */
function toCreationOptions(options: Record<string, unknown>): PublicKeyCredentialCreationOptions {
  const publicKey = (options.publicKey || options) as Record<string, unknown>;

  const challenge = base64urlToArrayBuffer(publicKey.challenge as string);

  const user = publicKey.user as Record<string, unknown>;
  const userId = base64urlToArrayBuffer(user.id as string);

  const excludeCredentials = ((publicKey.excludeCredentials as Array<Record<string, unknown>>) || []).map(
    (cred) => ({
      id: base64urlToArrayBuffer(cred.id as string),
      type: cred.type as PublicKeyCredentialType,
      transports: cred.transports as AuthenticatorTransport[] | undefined,
    })
  );

  return {
    ...publicKey,
    challenge,
    user: { ...user, id: userId },
    excludeCredentials,
  } as PublicKeyCredentialCreationOptions;
}

export default function AccountPasskeysPage() {
  const { passkeys, accessToken, apiBaseUrl, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const revalidator = useRevalidator();

  const isSubmitting = navigation.state === "submitting";

  const [registering, setRegistering] = useState(false);
  const [clientError, setClientError] = useState<string | null>(null);
  const [clientSuccess, setClientSuccess] = useState<string | null>(null);

  const handleRegisterPasskey = useCallback(async () => {
    setClientError(null);
    setClientSuccess(null);
    setRegistering(true);

    try {
      // 1. Get creation options from backend
      const startResponse = await fetch(
        `${apiBaseUrl}/api/v1/users/me/passkeys/register/start`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${accessToken}`,
          },
        }
      );

      if (!startResponse.ok) {
        const err = await startResponse.json().catch(() => ({ message: "Failed to start registration" }));
        throw new Error(err.message || "Failed to start registration");
      }

      const creationOptions = await startResponse.json();

      // 2. Convert to browser-compatible format and call WebAuthn API
      const options = toCreationOptions(creationOptions);
      const credential = await Promise.race([
        navigator.credentials.create({ publicKey: options }),
        new Promise<null>((_, reject) =>
          setTimeout(() => reject(new DOMException("The operation timed out.", "NotAllowedError")), 60000)
        ),
      ]);

      if (!credential) {
        throw new Error("Registration was cancelled");
      }

      // 3. Send result to backend
      const pkCred = credential as PublicKeyCredential;
      const attestation = pkCred.response as AuthenticatorAttestationResponse;
      const completeBody = {
        credential: {
          id: pkCred.id,
          rawId: arrayBufferToBase64url(pkCred.rawId),
          type: pkCred.type,
          response: {
            attestationObject: arrayBufferToBase64url(attestation.attestationObject),
            clientDataJSON: arrayBufferToBase64url(attestation.clientDataJSON),
          },
        },
      };
      const completeResponse = await fetch(
        `${apiBaseUrl}/api/v1/users/me/passkeys/register/complete`,
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${accessToken}`,
          },
          body: JSON.stringify(completeBody),
        }
      );
      if (!completeResponse.ok) {
        const err = await completeResponse.json().catch(() => ({ message: "Failed to complete registration" }));
        throw new Error(err.message || "Failed to complete registration");
      }

      setClientSuccess("Passkey registered successfully!");
      revalidator.revalidate();
    } catch (error) {
      if (error instanceof DOMException && error.name === "NotAllowedError") {
        setClientError("Registration was cancelled or timed out.");
      } else {
        const message = error instanceof Error ? error.message : "Registration failed";
        setClientError(message);
      }
    } finally {
      setRegistering(false);
    }
  }, [accessToken, apiBaseUrl, revalidator]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Passkeys</CardTitle>
              <CardDescription>
                Passkeys are a secure, passwordless way to sign in using your device&apos;s
                biometrics (fingerprint, face) or screen lock.
              </CardDescription>
            </div>
            <Button onClick={handleRegisterPasskey} disabled={registering || isSubmitting}>
              <PlusIcon className="h-4 w-4 mr-2" />
              {registering ? "Registering..." : "Add passkey"}
            </Button>
          </div>
        </CardHeader>
      </Card>

      {/* Error/Success Messages */}
      {loadError && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {loadError}
        </div>
      )}

      {clientError && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {clientError}
        </div>
      )}

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {actionData.error}
        </div>
      )}

      {clientSuccess && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {clientSuccess}
        </div>
      )}

      {actionData?.success && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {actionData.message}
        </div>
      )}

      {/* Passkeys List */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Your Passkeys</CardTitle>
        </CardHeader>
        <CardContent>
          {passkeys.length === 0 ? (
            <div className="text-center py-12">
              <div className="mx-auto w-12 h-12 bg-[var(--sidebar-item-hover)] rounded-full flex items-center justify-center mb-4">
                <LockClosedIcon className="h-6 w-6 text-[var(--text-tertiary)]" />
              </div>
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">
                No passkeys yet
              </h3>
              <p className="text-[var(--text-secondary)] mb-4">
                Add a passkey to sign in faster and more securely.
              </p>
              <Button onClick={handleRegisterPasskey} disabled={registering || isSubmitting}>
                <PlusIcon className="h-4 w-4 mr-2" />
                {registering ? "Registering..." : "Add your first passkey"}
              </Button>
            </div>
          ) : (
            <div className="divide-y">
              {passkeys.map((passkey: WebAuthnCredential) => (
                <div
                  key={passkey.id}
                  className="flex items-center gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div className="p-3 bg-blue-100 text-blue-700 rounded-full">
                    <LockClosedIcon className="h-5 w-5" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">
                      {passkey.user_label || "Passkey"}
                    </div>
                    <div className="text-sm text-[var(--text-secondary)] mt-0.5">
                      <span className="inline-block bg-[var(--sidebar-item-hover)] px-2 py-0.5 rounded text-xs mr-2">
                        {getCredentialTypeLabel(passkey.credential_type)}
                      </span>
                      Added {formatDate(passkey.created_at)}
                    </div>
                  </div>
                  <Form method="post">
                    <input type="hidden" name="intent" value="delete" />
                    <input type="hidden" name="credentialId" value={passkey.id} />
                    <Button
                      type="submit"
                      variant="ghost"
                      size="sm"
                      className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                      disabled={isSubmitting}
                    >
                      <TrashIcon className="h-4 w-4 mr-1" />
                      Remove
                    </Button>
                  </Form>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Info Card */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">About Passkeys</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4 text-sm text-[var(--text-secondary)]">
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-green-100 rounded-full flex items-center justify-center">
                <span className="text-[var(--accent-green)] font-bold">1</span>
              </div>
              <div>
                <h4 className="font-medium text-[var(--text-primary)]">More secure</h4>
                <p>Passkeys are resistant to phishing and cannot be stolen like passwords.</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-green-100 rounded-full flex items-center justify-center">
                <span className="text-[var(--accent-green)] font-bold">2</span>
              </div>
              <div>
                <h4 className="font-medium text-[var(--text-primary)]">Fast & easy</h4>
                <p>Sign in with a quick touch or glance using your device&apos;s biometrics.</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-green-100 rounded-full flex items-center justify-center">
                <span className="text-[var(--accent-green)] font-bold">3</span>
              </div>
              <div>
                <h4 className="font-medium text-[var(--text-primary)]">Works everywhere</h4>
                <p>Passkeys sync across your devices when signed into the same account.</p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
