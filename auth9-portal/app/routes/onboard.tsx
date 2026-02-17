import { useState, useEffect } from "react";
import type { MetaFunction, ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { redirect, Form, useActionData, useNavigation, Link } from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { ThemeToggle } from "~/components/ThemeToggle";
import { requireAuthWithUpdate, commitSession } from "~/services/session.server";
import { organizationApi, userApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Create Organization - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const { session } = await requireAuthWithUpdate(request);

  // If user already has tenants, redirect to dashboard
  try {
    const res = await userApi.getMyTenants(session.accessToken);
    if (res.data && res.data.length > 0) {
      throw redirect("/dashboard");
    }
  } catch (e) {
    if (e instanceof Response) throw e;
    // API error, continue to show onboard page
  }

  // Get user email for domain suggestion
  let email = "";
  try {
    const userRes = await userApi.getMe(session.accessToken);
    email = userRes.data?.email || "";
  } catch {
    // fallback
  }

  return { email };
}

export async function action({ request }: ActionFunctionArgs) {
  const { session } = await requireAuthWithUpdate(request);
  const formData = await request.formData();

  const name = (formData.get("name") as string || "").trim();
  const slug = (formData.get("slug") as string || "").trim();
  const domain = (formData.get("domain") as string || "").trim();

  if (!name || !slug || !domain) {
    return { error: "All fields are required" };
  }

  try {
    const result = await organizationApi.create(
      { name, slug, domain },
      session.accessToken
    );

    const org = result.data.organization;

    // Update session with new active tenant
    const updatedSession = { ...session, activeTenantId: org.id };

    if (org.status === "pending") {
      return redirect("/onboard/pending", {
        headers: { "Set-Cookie": await commitSession(updatedSession) },
      });
    }

    return redirect("/dashboard", {
      headers: { "Set-Cookie": await commitSession(updatedSession) },
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to create organization";
    return { error: message };
  }
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 63);
}

export default function Onboard() {
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugManuallyEdited, setSlugManuallyEdited] = useState(false);

  useEffect(() => {
    if (!slugManuallyEdited && name) {
      setSlug(slugify(name));
    }
  }, [name, slugManuallyEdited]);

  return (
    <>
      <div className="fixed top-6 right-6 z-20">
        <ThemeToggle />
      </div>

      <div className="min-h-screen flex items-center justify-center px-6 relative">
        <div className="page-backdrop" />

        <Card className="w-full max-w-lg relative z-10 animate-fade-in-up">
          <CardHeader className="text-center">
            <div className="logo-icon mx-auto mb-4">A9</div>
            <CardTitle className="text-2xl">Create your organization</CardTitle>
            <CardDescription>
              Set up your organization to start managing users and access.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Form method="post" className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="name">Organization name</Label>
                <Input
                  id="name"
                  name="name"
                  required
                  placeholder="Acme Corporation"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="slug">Slug</Label>
                <Input
                  id="slug"
                  name="slug"
                  required
                  placeholder="acme-corp"
                  value={slug}
                  onChange={(e) => {
                    setSlug(e.target.value);
                    setSlugManuallyEdited(true);
                  }}
                />
                <p className="text-xs text-[var(--text-tertiary)]">
                  URL-friendly identifier (lowercase, hyphens only)
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="domain">Email domain</Label>
                <Input
                  id="domain"
                  name="domain"
                  required
                  placeholder="acme.com"
                />
                <p className="text-xs text-[var(--text-tertiary)]">
                  Users with this email domain will be auto-verified
                </p>
              </div>

              {actionData?.error && (
                <p className="text-sm text-[var(--accent-red)]">{actionData.error}</p>
              )}

              <Button type="submit" className="w-full" disabled={isSubmitting}>
                {isSubmitting ? "Creating..." : "Create organization"}
              </Button>
            </Form>

            <div className="mt-4 text-center">
              <Link
                to="/logout"
                className="text-sm text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] transition-colors"
              >
                Sign out
              </Link>
            </div>
          </CardContent>
        </Card>
      </div>
    </>
  );
}
