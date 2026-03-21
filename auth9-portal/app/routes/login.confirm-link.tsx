import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { redirect, Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { AuthPageShell } from "~/components/AuthPageShell";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { API_BASE_URL } from "~/services/api/client";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.login.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const token = url.searchParams.get("token");

  if (!token) {
    return redirect("/login");
  }

  return { token };
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const token = formData.get("token") as string | null;
  const intent = formData.get("intent") as string | null;

  if (!token) {
    return { error: "Missing token." };
  }

  const actionValue = intent === "create_new" ? "create_new" : undefined;

  try {
    const response = await fetch(`${API_BASE_URL}/api/v1/auth/confirm-link`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        token,
        ...(actionValue ? { action: actionValue } : {}),
      }),
    });

    if (!response.ok) {
      const err = await response.json().catch(() => ({
        message: "An unexpected error occurred.",
      }));
      return { error: err.message || "An unexpected error occurred." };
    }

    const result = await response.json();

    if (result.redirect_url) {
      return redirect(result.redirect_url);
    }

    return { error: "No redirect URL received from server." };
  } catch {
    return { error: "Failed to connect to the server. Please try again." };
  }
}

export default function ConfirmLink() {
  const { token } = useLoaderData<typeof loader>() as { token: string };
  const actionData = useActionData<typeof action>() as { error?: string } | undefined;
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <AuthPageShell>
      <Card className="w-full max-w-xl animate-fade-in-up">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl">Link Your Account</CardTitle>
          <CardDescription>
            An external identity was found that matches an existing account.
            Would you like to link them?
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {actionData?.error && (
              <p className="text-sm text-[var(--accent-red)] bg-red-50 dark:bg-red-950/20 p-3 rounded-md text-center">
                {actionData.error}
              </p>
            )}

            <Form method="post">
              <input type="hidden" name="token" value={token} />
              <input type="hidden" name="intent" value="link" />
              <Button
                type="submit"
                className="w-full"
                disabled={isSubmitting}
              >
                {isSubmitting ? "Linking..." : "Link to Existing Account"}
              </Button>
            </Form>

            <Form method="post">
              <input type="hidden" name="token" value={token} />
              <input type="hidden" name="intent" value="create_new" />
              <Button
                type="submit"
                variant="outline"
                className="w-full"
                disabled={isSubmitting}
              >
                {isSubmitting ? "Creating..." : "Create New Account"}
              </Button>
            </Form>

            <div className="text-center pt-2">
              <Link
                to="/login"
                className="text-sm text-[var(--text-tertiary)] hover:text-[var(--text-primary)] underline-offset-4 hover:underline"
              >
                Back to Login
              </Link>
            </div>
          </div>
        </CardContent>
      </Card>
    </AuthPageShell>
  );
}
