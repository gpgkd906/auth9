import type { LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { webauthnApi, type WebAuthnCredential } from "~/services/api";
import { LockClosedIcon, TrashIcon, PlusIcon } from "@radix-ui/react-icons";
import { getAccessToken } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  try {
    const accessToken = await getAccessToken(request);
    const response = await webauthnApi.listPasskeys(accessToken || "");
    return { passkeys: response.data };
  } catch {
    return { passkeys: [], error: "Failed to load passkeys" };
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

    if (intent === "register") {
      const redirectUri = formData.get("redirectUri") as string;
      const response = await webauthnApi.getRegisterUrl(redirectUri, accessToken);
      return { redirect: response.data.url };
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

export default function AccountPasskeysPage() {
  const { passkeys, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";

  // Handle redirect for registration
  if (actionData?.redirect) {
    window.location.href = actionData.redirect;
    return null;
  }

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
            <Form method="post">
              <input type="hidden" name="intent" value="register" />
              <input
                type="hidden"
                name="redirectUri"
                value={typeof window !== "undefined" ? window.location.href : ""}
              />
              <Button type="submit" disabled={isSubmitting}>
                <PlusIcon className="h-4 w-4 mr-2" />
                Add passkey
              </Button>
            </Form>
          </div>
        </CardHeader>
      </Card>

      {/* Error/Success Messages */}
      {loadError && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {loadError}
        </div>
      )}

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {actionData.error}
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
              <Form method="post">
                <input type="hidden" name="intent" value="register" />
                <input
                  type="hidden"
                  name="redirectUri"
                  value={typeof window !== "undefined" ? window.location.href : ""}
                />
                <Button type="submit" disabled={isSubmitting}>
                  <PlusIcon className="h-4 w-4 mr-2" />
                  Add your first passkey
                </Button>
              </Form>
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
