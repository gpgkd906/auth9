import type { MetaFunction, ActionFunctionArgs } from "react-router";
import { redirect, Form, Link, useNavigation } from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { ThemeToggle } from "~/components/ThemeToggle";

export const meta: MetaFunction = () => {
  return [{ title: "Sign In - Auth9" }];
};

export async function action({ request }: ActionFunctionArgs) {
  const url = new URL(request.url);
  // Use public URL for browser redirects (defaults to localhost for local dev)
  const corePublicUrl = process.env.AUTH9_CORE_PUBLIC_URL || process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const portalUrl = process.env.AUTH9_PORTAL_URL || url.origin;
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  const redirectUri = `${portalUrl}/auth/callback`;

  // Generate random state for CSRF protection
  const state = crypto.randomUUID();

  const authorizeUrl = new URL(`${corePublicUrl}/api/v1/auth/authorize`);
  authorizeUrl.searchParams.set("response_type", "code");
  authorizeUrl.searchParams.set("client_id", clientId);
  authorizeUrl.searchParams.set("redirect_uri", redirectUri);
  authorizeUrl.searchParams.set("scope", "openid email profile");
  authorizeUrl.searchParams.set("state", state);

  return redirect(authorizeUrl.toString());
}

export default function Login() {
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <>
      {/* Theme Toggle - outside flex container to avoid layout issues */}
      <div className="fixed top-6 right-6 z-20">
        <ThemeToggle />
      </div>

      <div className="min-h-screen flex items-center justify-center px-6 relative">
        {/* Dynamic Background */}
        <div className="page-backdrop" />

        <Card className="w-full max-w-md relative z-10 animate-fade-in-up">
        <CardHeader className="text-center">
          <div className="logo-icon mx-auto mb-4">A9</div>
          <CardTitle className="text-2xl">Welcome back</CardTitle>
          <CardDescription>
            Sign in to your Auth9 account
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? "Redirecting..." : "Sign in with SSO"}
            </Button>
          </Form>

          <div className="mt-6 text-center text-sm text-[var(--text-tertiary)]">
            Don&apos;t have an account?{" "}
            <Link to="/register" className="text-[var(--accent-blue)] hover:underline font-medium">
              Sign up
            </Link>
          </div>
        </CardContent>
        </Card>
      </div>
    </>
  );
}
