import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Avatar, AvatarFallback, AvatarImage } from "~/components/ui/avatar";
import { userApi } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { redirect } from "react-router";

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  try {
    const response = await userApi.getMe(accessToken);
    return { user: response.data, error: null };
  } catch (error) {
    // Network errors (backend down) - show error on page instead of crashing
    if (error instanceof TypeError && error.message.includes("fetch")) {
      return { user: null, error: "Unable to connect to the server. Please try again later." };
    }
    throw redirect("/login");
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    return { error: "Not authenticated" };
  }

  const formData = await request.formData();
  const displayName = formData.get("display_name") as string;
  const avatarUrl = formData.get("avatar_url") as string;

  try {
    await userApi.updateMe(
      {
        display_name: displayName || undefined,
        avatar_url: avatarUrl || undefined,
      },
      accessToken
    );
    return { success: true, message: "Profile updated successfully" };
  } catch (error) {
    if (error instanceof TypeError && error.message.includes("fetch")) {
      return { error: "Unable to connect to the server. Please try again later." };
    }
    const message = error instanceof Error ? error.message : "Failed to update profile";
    return { error: message };
  }
}

export default function AccountProfilePage() {
  const { user, error: loaderError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";

  if (!user) {
    return (
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Profile</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
              {loaderError || "Failed to load profile. Please try again later."}
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  const displayName = user.display_name || "";
  const initials = (user.display_name || user.email)
    .split(" ")
    .map((n: string) => n[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  return (
    <div className="space-y-6">
      {/* Profile Card */}
      <Card>
        <CardHeader>
          <CardTitle>Profile</CardTitle>
          <CardDescription>
            Your personal information. This is how others see you on the platform.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-6 max-w-md">
            {/* Avatar Preview */}
            <div className="flex items-center gap-4">
              <Avatar className="h-16 w-16 text-lg">
                <AvatarImage src={user.avatar_url || ""} />
                <AvatarFallback>{initials}</AvatarFallback>
              </Avatar>
              <div>
                <p className="text-sm font-medium text-[var(--text-primary)]">
                  {user.display_name || user.email}
                </p>
                <p className="text-xs text-[var(--text-secondary)]">
                  {user.email}
                </p>
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="display_name">Display name</Label>
              <Input
                id="display_name"
                name="display_name"
                defaultValue={displayName}
                placeholder="Enter your display name"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="avatar_url">Avatar URL</Label>
              <Input
                id="avatar_url"
                name="avatar_url"
                defaultValue={user.avatar_url || ""}
                placeholder="https://example.com/avatar.png"
              />
              <p className="text-xs text-[var(--text-secondary)]">
                URL to your profile picture
              </p>
            </div>

            <div className="space-y-2">
              <Label>Email</Label>
              <Input
                value={user.email}
                disabled
                className="opacity-60"
              />
              <p className="text-xs text-[var(--text-secondary)]">
                Email cannot be changed here
              </p>
            </div>

            <div className="flex items-center gap-4 text-sm">
              <div>
                <span className="text-[var(--text-secondary)]">MFA: </span>
                <span className={user.mfa_enabled ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]"}>
                  {user.mfa_enabled ? "Enabled" : "Disabled"}
                </span>
              </div>
              <div>
                <span className="text-[var(--text-secondary)]">Joined: </span>
                <span className="text-[var(--text-primary)]">
                  {new Date(user.created_at).toLocaleDateString()}
                </span>
              </div>
            </div>

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

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving..." : "Save changes"}
            </Button>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
