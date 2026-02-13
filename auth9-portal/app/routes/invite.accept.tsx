import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { invitationApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Accept Invitation - Auth9" }];
};

interface LoaderData {
  token: string | null;
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const token = url.searchParams.get("token");
  return { token } satisfies LoaderData;
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const token = formData.get("token") as string | null;
  const email = (formData.get("email") as string | null) || undefined;
  const displayName = (formData.get("display_name") as string | null) || undefined;
  const password = (formData.get("password") as string | null) || undefined;

  if (!token) {
    return Response.json({ error: "Invitation token is missing" }, { status: 400 });
  }

  try {
    const response = await invitationApi.accept({
      token,
      email,
      display_name: displayName,
      password,
    });

    return { success: true, invitation: response.data };
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }
}

export default function InviteAcceptPage() {
  const { token } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  if (!token) {
    return (
      <div className="min-h-screen flex items-center justify-center px-6 relative">
        <div className="page-backdrop" />
        <Card className="w-full max-w-md relative z-10">
          <CardHeader className="text-center">
            <CardTitle>Invalid Invitation</CardTitle>
            <CardDescription>The invitation link is missing or malformed.</CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Link to="/login" className="text-[var(--accent-blue)] hover:underline text-sm">
              Go to login
            </Link>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center px-6 relative">
      <div className="page-backdrop" />

      <Card className="w-full max-w-md relative z-10 animate-fade-in-up">
        <CardHeader className="text-center">
          <CardTitle>Accept Invitation</CardTitle>
          <CardDescription>
            Create your account or confirm your details to join the tenant.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="token" value={token} />

            <div className="space-y-2">
              <Label htmlFor="email">Email (optional)</Label>
              <Input id="email" name="email" type="email" placeholder="you@example.com" />
            </div>

            <div className="space-y-2">
              <Label htmlFor="display_name">Display Name</Label>
              <Input id="display_name" name="display_name" placeholder="Your name" />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input id="password" name="password" type="password" placeholder="Create a password" />
              <p className="text-xs text-[var(--text-tertiary)]">
                If you already have an account, you can leave this blank.
              </p>
            </div>

            {actionData && "error" in actionData && (
              <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>
            )}

            {actionData && "success" in actionData && actionData.success && (
              <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-3 text-sm text-[var(--accent-green)]">
                Invitation accepted successfully. You can now sign in.
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? "Submitting..." : "Accept Invitation"}
            </Button>
          </Form>

          <div className="mt-6 text-center text-sm text-[var(--text-tertiary)]">
            Already have an account?{" "}
            <Link to={`/login?invite_token=${encodeURIComponent(token)}`} className="text-[var(--accent-blue)] hover:underline font-medium">
              Sign in
            </Link>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
