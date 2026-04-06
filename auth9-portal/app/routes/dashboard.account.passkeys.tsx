import type { LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, useRevalidator } from "react-router";
import { useState, useCallback } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { webauthnApi, type WebAuthnCredential } from "~/services/api";
import { LockClosedIcon, TrashIcon, PlusIcon } from "@radix-ui/react-icons";
import { requireIdentityAuthWithUpdate } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  const apiBaseUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  try {
    const { session, headers } = await requireIdentityAuthWithUpdate(request);
    const accessToken = session.identityAccessToken || "";
    const response = await webauthnApi.listPasskeys(accessToken);
    const data = { passkeys: response.data, accessToken, apiBaseUrl, error: null as string | null };
    if (headers) {
      return Response.json(data, { headers });
    }
    return data;
  } catch {
    const locale = await resolveLocale(request);
    return {
      passkeys: [] as WebAuthnCredential[],
      accessToken: "",
      apiBaseUrl,
      error: translate(locale, "accountPasskeys.loadError"),
    };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");
  const { session, headers } = await requireIdentityAuthWithUpdate(request);
  const accessToken = session.identityAccessToken || "";

  try {
    if (intent === "delete") {
      const credentialId = formData.get("credentialId") as string;
      await webauthnApi.deletePasskey(credentialId, accessToken || "");
      const locale = await resolveLocale(request);
      const data = {
        success: true as const,
        message: translate(locale, "accountPasskeys.deleted"),
        error: undefined as string | undefined,
      };
      if (headers) {
        return Response.json(data, { headers });
      }
      return data;
    }
  } catch (error) {
    const locale = await resolveLocale(request);
    const message = mapApiError(error, locale);
    return { success: undefined as true | undefined, message: undefined as string | undefined, error: message };
  }

  const locale = await resolveLocale(request);
  return {
    success: undefined as true | undefined,
    message: undefined as string | undefined,
    error: translate(locale, "accountPasskeys.invalidAction"),
  };
}

function getCredentialTypeLabel(type: string, t: ReturnType<typeof useI18n>["t"]) {
  switch (type) {
    case "webauthn-passwordless":
      return t("accountPasskeys.passwordless");
    case "webauthn":
      return t("accountPasskeys.twoFactor");
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
  const { t } = useI18n();
  const formatters = useFormatters();
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
        const err = await startResponse.json().catch(() => ({ message: t("accountPasskeys.startFailed") }));
        throw new Error(err.message || t("accountPasskeys.startFailed"));
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
        throw new Error(t("accountPasskeys.cancelled"));
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
        const err = await completeResponse.json().catch(() => ({ message: t("accountPasskeys.completeFailed") }));
        throw new Error(err.message || t("accountPasskeys.completeFailed"));
      }

      setClientSuccess(t("accountPasskeys.registered"));
      revalidator.revalidate();
    } catch (error) {
      if (error instanceof DOMException && error.name === "NotAllowedError") {
        setClientError(t("accountPasskeys.cancelled"));
      } else {
        const message = error instanceof Error ? error.message : t("accountPasskeys.registrationFailed");
        setClientError(message);
      }
    } finally {
      setRegistering(false);
    }
  }, [accessToken, apiBaseUrl, revalidator, t]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <Card>
        <CardHeader className="pb-5">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{t("accountPasskeys.title")}</CardTitle>
              <CardDescription>
                {t("accountPasskeys.description")}
              </CardDescription>
            </div>
            <Button onClick={handleRegisterPasskey} disabled={registering || isSubmitting}>
              <PlusIcon className="h-4 w-4 mr-2" />
              {registering ? t("accountPasskeys.registering") : t("accountPasskeys.add")}
            </Button>
          </div>
        </CardHeader>
      </Card>

      {/* Error/Success Messages */}
      {loadError && (
        <div className="text-sm text-[var(--accent-red)] bg-[var(--accent-red)]/10 p-3 rounded-md">
          {loadError}
        </div>
      )}

      {clientError && (
        <div className="text-sm text-[var(--accent-red)] bg-[var(--accent-red)]/10 p-3 rounded-md">
          {clientError}
        </div>
      )}

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-[var(--accent-red)]/10 p-3 rounded-md">
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
          <CardTitle className="text-lg">{t("accountPasskeys.yourPasskeys")}</CardTitle>
        </CardHeader>
        <CardContent>
          {passkeys.length === 0 ? (
            <div className="text-center py-12">
              <div className="mx-auto w-12 h-12 bg-[var(--sidebar-item-hover)] rounded-full flex items-center justify-center mb-4">
                <LockClosedIcon className="h-6 w-6 text-[var(--text-tertiary)]" />
              </div>
              <h3 className="text-[17px] font-semibold text-[var(--text-primary)] mb-2">
                {t("accountPasskeys.yourPasskeys")}
              </h3>
              <p className="text-[13px] text-[var(--text-secondary)] mb-4">
                {t("accountPasskeys.description")}
              </p>
              <Button onClick={handleRegisterPasskey} disabled={registering || isSubmitting}>
                <PlusIcon className="h-4 w-4 mr-2" />
                {registering ? t("accountPasskeys.registering") : t("accountPasskeys.addFirst")}
              </Button>
            </div>
          ) : (
            <div className="divide-y">
              {passkeys.map((passkey: WebAuthnCredential) => (
                <div
                  key={passkey.id}
                  className="flex items-center gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div className="p-3 bg-[var(--accent-cyan-light)] text-[var(--accent-cyan)] rounded-full">
                    <LockClosedIcon className="h-5 w-5" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">
                      {passkey.user_label || t("accountPasskeys.passkeyFallback")}
                    </div>
                    <div className="text-sm text-[var(--text-secondary)] mt-0.5">
                      <span className="inline-block bg-[var(--sidebar-item-hover)] px-2 py-0.5 rounded text-xs mr-2">
                        {getCredentialTypeLabel(passkey.credential_type, t)}
                      </span>
                      {t("account.sessions.started")} {formatters.date(passkey.created_at)}
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
                      {t("common.buttons.delete")}
                    </Button>
                  </Form>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Info Card */}
      <Card className="bg-[var(--accent-cyan-light)]">
        <CardHeader>
          <CardTitle className="text-lg">{t("accountPasskeys.about")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4 text-sm text-[var(--text-secondary)]">
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-[var(--accent-green)]/15 rounded-full flex items-center justify-center">
                <span className="text-[var(--accent-green)] font-bold">1</span>
              </div>
              <div>
                <h4 className="font-medium text-[var(--text-primary)]">{t("accountPasskeys.secureTitle")}</h4>
                <p>{t("accountPasskeys.secureDescription")}</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-[var(--accent-green)]/15 rounded-full flex items-center justify-center">
                <span className="text-[var(--accent-green)] font-bold">2</span>
              </div>
              <div>
                <h4 className="font-medium text-[var(--text-primary)]">{t("accountPasskeys.fastTitle")}</h4>
                <p>{t("accountPasskeys.fastDescription")}</p>
              </div>
            </div>
            <div className="flex gap-3">
              <div className="flex-shrink-0 w-8 h-8 bg-[var(--accent-green)]/15 rounded-full flex items-center justify-center">
                <span className="text-[var(--accent-green)] font-bold">3</span>
              </div>
              <div>
                <h4 className="font-medium text-[var(--text-primary)]">{t("accountPasskeys.everywhereTitle")}</h4>
                <p>{t("accountPasskeys.everywhereDescription")}</p>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
