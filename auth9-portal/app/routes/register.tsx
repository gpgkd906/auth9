import type { MetaFunction, ActionFunctionArgs } from "react-router";
import { redirect, Form, Link, useActionData, useNavigation } from "react-router";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { ThemeToggle } from "~/components/ThemeToggle";
import { userApi, publicBrandingApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Sign Up - Auth9" }];
};

export async function loader() {
  try {
    const { data: branding } = await publicBrandingApi.get();
    if (!branding.allow_registration) {
      return redirect("/login");
    }
    return null;
  } catch {
    // If we can't fetch branding config, default to disallowing registration
    return redirect("/login");
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const email = formData.get("email");
  const password = formData.get("password");
  const displayName = formData.get("display_name");

  if (!email || !password) {
    return Response.json({ error: "Email and password are required" }, { status: 400 });
  }

  try {
    await userApi.create({
      email: String(email),
      display_name: displayName ? String(displayName) : undefined,
      password: String(password),
    });
    return redirect("/login");
  } catch (error) {
    return Response.json(
      { error: error instanceof Error ? error.message : "Registration failed" },
      { status: 400 }
    );
  }
}

export default function Register() {
  const actionData = useActionData<{ error?: string }>();
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
          <CardTitle className="text-2xl">Create your account</CardTitle>
          <CardDescription>Start managing identity with Auth9</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            {actionData?.error && (
              <div className="p-3 rounded-xl bg-[var(--accent-red)]/10 text-[var(--accent-red)] text-sm border border-[var(--accent-red)]/20">
                {actionData.error}
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                name="email"
                type="email"
                placeholder="you@example.com"
                required
                autoComplete="email"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="display_name">Display Name</Label>
              <Input id="display_name" name="display_name" placeholder="Jane Doe" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                name="password"
                type="password"
                placeholder="••••••••"
                required
                autoComplete="new-password"
              />
            </div>
            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? "Creating..." : "Create account"}
            </Button>
          </Form>
          <div className="mt-6 text-center text-sm text-[var(--text-tertiary)]">
            Already have an account?{" "}
            <Link to="/login" className="text-[var(--accent-blue)] hover:underline font-medium">
              Sign in
            </Link>
          </div>
          </CardContent>
        </Card>
      </div>
    </>
  );
}
