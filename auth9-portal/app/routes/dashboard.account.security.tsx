import type { ActionFunctionArgs } from "react-router";
import { Form, useActionData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { passwordApi } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const currentPassword = formData.get("currentPassword") as string;
  const newPassword = formData.get("newPassword") as string;
  const confirmPassword = formData.get("confirmPassword") as string;

  if (!currentPassword || !newPassword || !confirmPassword) {
    const locale = await resolveLocale(request);
    return { error: translate(locale, "account.security.required") };
  }

  if (newPassword !== confirmPassword) {
    const locale = await resolveLocale(request);
    return { error: translate(locale, "account.security.mismatch") };
  }

  try {
    const accessToken = await getAccessToken(request) || "";
    await passwordApi.changePassword(currentPassword, newPassword, accessToken);
    const locale = await resolveLocale(request);
    return { success: true, message: translate(locale, "account.security.success") };
  } catch (error) {
    const locale = await resolveLocale(request);
    const message = mapApiError(error, locale);
    return { error: message };
  }
}

export default function AccountSecurityPage() {
  const { t } = useI18n();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();

  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{t("account.security.title")}</CardTitle>
          <CardDescription>
            {t("account.security.description")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4 max-w-md" noValidate>
            <div className="space-y-2">
              <Label htmlFor="currentPassword">{t("account.security.currentPassword")}</Label>
              <Input
                id="currentPassword"
                name="currentPassword"
                type="password"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="newPassword">{t("account.security.newPassword")}</Label>
              <Input
                id="newPassword"
                name="newPassword"
                type="password"
              />
              <p className="text-xs text-[var(--text-secondary)]">{t("auth.resetPassword.passwordHint")}</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirmPassword">{t("account.security.confirmPassword")}</Label>
              <Input
                id="confirmPassword"
                name="confirmPassword"
                type="password"
              />
            </div>

            {actionData?.error && (
              <div className="text-sm text-[var(--accent-red)] bg-[var(--accent-red)]/10 p-3 rounded-md">
                {actionData.error}
              </div>
            )}

            {actionData?.success && (
              <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
                {actionData.message}
              </div>
            )}

            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? t("account.security.changing") : t("account.security.change")}
            </Button>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
