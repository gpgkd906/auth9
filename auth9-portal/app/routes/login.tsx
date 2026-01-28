import type { ActionFunctionArgs, MetaFunction } from "@remix-run/node";
import { redirect } from "@remix-run/node";
import { Form, Link, useNavigation } from "@remix-run/react";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [{ title: "Sign In - Auth9" }];
};

export async function action({ request }: ActionFunctionArgs) {
  const url = new URL(request.url);
  const coreUrl = process.env.AUTH9_CORE_URL || "http://localhost:8080";
  const portalUrl = process.env.AUTH9_PORTAL_URL || url.origin;
  const clientId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
  const redirectUri = `${portalUrl}/dashboard`;

  const authorizeUrl = new URL(`${coreUrl}/api/v1/auth/authorize`);
  authorizeUrl.searchParams.set("response_type", "code");
  authorizeUrl.searchParams.set("client_id", clientId);
  authorizeUrl.searchParams.set("redirect_uri", redirectUri);
  authorizeUrl.searchParams.set("scope", "openid email profile");

  return redirect(authorizeUrl.toString());
}

export default function Login() {
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 px-6">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <div className="w-12 h-12 rounded-apple-lg bg-apple-blue flex items-center justify-center mx-auto mb-4">
            <span className="text-white font-bold text-xl">A</span>
          </div>
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

          <div className="mt-6 text-center text-sm text-gray-500">
            Don&apos;t have an account?{" "}
            <Link to="/register" className="text-apple-blue hover:underline font-medium">
              Sign up
            </Link>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
