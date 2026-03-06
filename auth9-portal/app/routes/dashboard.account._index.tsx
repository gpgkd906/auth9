import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Avatar, AvatarFallback, AvatarImage } from "~/components/ui/avatar";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
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
      const locale = await resolveLocale(request);
      return { user: null, error: translate(locale, "account.profile.serverUnavailable") };
    }
    throw redirect("/login");
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    const locale = await resolveLocale(request);
    return { error: translate(locale, "account.profile.notAuthenticated") };
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
    const locale = await resolveLocale(request);
    return { success: true, message: translate(locale, "account.profile.updated") };
  } catch (error) {
    if (error instanceof TypeError && error.message.includes("fetch")) {
      const locale = await resolveLocale(request);
      return { error: translate(locale, "account.profile.serverUnavailable") };
    }
    const locale = await resolveLocale(request);
    const message =
      error instanceof Error ? error.message : translate(locale, "account.profile.updateFailed");
    return { error: message };
  }
}

export default function AccountProfilePage() {
  const { t } = useI18n();
  const formatters = useFormatters();
  const { user, error: loaderError } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";

  if (!user) {
    return (
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>{t("account.profile.title")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
              {loaderError || t("account.profile.loadError")}
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
          <CardTitle>{t("account.profile.title")}</CardTitle>
          <CardDescription>
            {t("account.profile.description")}
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
              <Label htmlFor="display_name">{t("account.profile.displayName")}</Label>
              <Input
                id="display_name"
                name="display_name"
                defaultValue={displayName}
                placeholder={t("account.profile.displayNamePlaceholder")}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="avatar_url">{t("account.profile.avatarUrl")}</Label>
              <Input
                id="avatar_url"
                name="avatar_url"
                defaultValue={user.avatar_url || ""}
                placeholder={t("account.profile.avatarUrlPlaceholder")}
              />
              <p className="text-xs text-[var(--text-secondary)]">
                {t("account.profile.avatarHint")}
              </p>
            </div>

            <div className="space-y-2">
              <Label>{t("account.profile.email")}</Label>
              <Input
                value={user.email}
                disabled
                className="opacity-60"
              />
              <p className="text-xs text-[var(--text-secondary)]">
                {t("account.profile.emailHint")}
              </p>
            </div>

            <div className="flex items-center gap-4 text-sm">
              <div>
                <span className="text-[var(--text-secondary)]">{t("account.profile.mfa")}: </span>
                <span className={user.mfa_enabled ? "text-[var(--accent-green)]" : "text-[var(--text-tertiary)]"}>
                  {user.mfa_enabled ? t("account.profile.enabled") : t("account.profile.disabled")}
                </span>
              </div>
              <div>
                <span className="text-[var(--text-secondary)]">{t("account.profile.joined")}: </span>
                <span className="text-[var(--text-primary)]">
                  {formatters.date(user.created_at)}
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
              {isSubmitting ? t("account.profile.saving") : t("account.profile.save")}
            </Button>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
