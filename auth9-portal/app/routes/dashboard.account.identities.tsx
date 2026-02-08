import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { redirect } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { identityProviderApi, type LinkedIdentity } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { Cross2Icon } from "@radix-ui/react-icons";

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  try {
    const response = await identityProviderApi.listMyLinkedIdentities(accessToken);
    return { identities: response.data };
  } catch {
    return { identities: [], error: "Failed to load linked identities" };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return { error: "Not authenticated" };
  }

  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "unlink") {
      const identityId = formData.get("identityId") as string;
      await identityProviderApi.unlinkIdentity(identityId, accessToken);
      return { success: true, message: "Identity unlinked successfully" };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
}

function getProviderIcon(providerType: string) {
  switch (providerType.toLowerCase()) {
    case "google":
      return "G";
    case "github":
      return "GH";
    case "microsoft":
      return "MS";
    case "apple":
      return "AP";
    case "facebook":
      return "FB";
    default:
      return providerType.slice(0, 2).toUpperCase();
  }
}

function getProviderName(providerAlias: string, providerType: string) {
  const name = providerAlias || providerType;
  return name.charAt(0).toUpperCase() + name.slice(1);
}

export default function AccountIdentitiesPage() {
  const { identities, error: loadError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Linked Identities</CardTitle>
          <CardDescription>
            External accounts connected to your Auth9 account. These allow you to sign in
            using third-party providers.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loadError && (
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md mb-4">
              {loadError}
            </div>
          )}

          {actionData?.error && (
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md mb-4">
              {actionData.error}
            </div>
          )}

          {actionData?.success && (
            <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md mb-4">
              {actionData.message}
            </div>
          )}

          {identities.length === 0 ? (
            <div className="text-center py-12">
              <div className="mx-auto w-12 h-12 bg-[var(--sidebar-item-hover)] rounded-full flex items-center justify-center mb-4">
                <LinkIcon className="h-6 w-6 text-[var(--text-tertiary)]" />
              </div>
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">
                No linked identities
              </h3>
              <p className="text-[var(--text-secondary)]">
                You haven&apos;t connected any external accounts yet.
                Link an identity provider from the login page to enable social sign-in.
              </p>
            </div>
          ) : (
            <div className="divide-y">
              {identities.map((identity: LinkedIdentity) => (
                <div
                  key={identity.id}
                  className="flex items-center gap-4 py-4 first:pt-0 last:pb-0"
                >
                  <div className="w-10 h-10 bg-[var(--sidebar-item-hover)] rounded-full flex items-center justify-center text-sm font-semibold text-[var(--text-secondary)]">
                    {getProviderIcon(identity.provider_type)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">
                      {getProviderName(identity.provider_alias, identity.provider_type)}
                    </div>
                    <div className="text-sm text-[var(--text-secondary)] mt-0.5">
                      {identity.external_email || identity.external_user_id}
                      <span className="text-xs text-[var(--text-tertiary)] ml-2">
                        Linked {new Date(identity.linked_at).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                  <Form method="post">
                    <input type="hidden" name="intent" value="unlink" />
                    <input type="hidden" name="identityId" value={identity.id} />
                    <Button
                      type="submit"
                      variant="ghost"
                      size="sm"
                      className="text-[var(--accent-red)] hover:text-[var(--accent-red)] hover:bg-[var(--accent-red)]/10"
                      disabled={isSubmitting}
                    >
                      <Cross2Icon className="h-4 w-4 mr-1" />
                      Unlink
                    </Button>
                  </Form>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function LinkIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
    </svg>
  );
}
