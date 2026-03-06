import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, Link } from "react-router";
import { useState } from "react";
import { LanguageSwitcher } from "~/components/LanguageSwitcher";
import { ThemeToggle } from "~/components/ThemeToggle";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { useI18n } from "~/i18n";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";
import { passwordApi } from "~/services/api";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "auth.resetPassword.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const url = new URL(request.url);
  const token = url.searchParams.get("token");

  if (!token) {
    return { error: translate(locale, "auth.resetPassword.invalidToken") };
  }

  return { token };
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const token = formData.get("token") as string;
  const password = formData.get("password") as string;
  const confirmPassword = formData.get("confirmPassword") as string;

  if (!token) {
    return { error: translate(locale, "auth.resetPassword.invalidToken") };
  }

  if (!password) {
    return { error: translate(locale, "auth.resetPassword.passwordRequired") };
  }

  if (password !== confirmPassword) {
    return { error: translate(locale, "auth.resetPassword.passwordMismatch") };
  }

  try {
    await passwordApi.resetPassword(token, password);
    return { success: true };
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "auth.resetPassword.failed");
    return { error: message };
  }
}

export default function ResetPasswordPage() {
  const { t } = useI18n();
  const loaderData = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");

  const isSubmitting = navigation.state === "submitting";

  // Show error if no token
  if ("error" in loaderData) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4 relative">
        <div className="fixed top-6 right-6 z-20 flex items-center gap-3">
          <LanguageSwitcher />
          <ThemeToggle />
        </div>
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <CardTitle>{t("auth.resetPassword.invalidTitle")}</CardTitle>
            <CardDescription>{loaderData.error}</CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Link to="/forgot-password">
              <Button>{t("common.buttons.requestNewResetLink")}</Button>
            </Link>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Show success message
  if (actionData?.success) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4 relative">
        <div className="fixed top-6 right-6 z-20 flex items-center gap-3">
          <LanguageSwitcher />
          <ThemeToggle />
        </div>
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <CardTitle>{t("auth.resetPassword.successTitle")}</CardTitle>
            <CardDescription>
              {t("auth.resetPassword.successDescription")}
            </CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Link to="/login">
              <Button>{t("common.buttons.signIn")}</Button>
            </Link>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4 relative">
      <div className="fixed top-6 right-6 z-20 flex items-center gap-3">
        <LanguageSwitcher />
        <ThemeToggle />
      </div>
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <CardTitle>{t("auth.resetPassword.title")}</CardTitle>
          <CardDescription>{t("auth.resetPassword.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="token" value={loaderData.token} />

            <div className="space-y-2">
              <Label htmlFor="password">{t("common.labels.newPassword")}</Label>
              <Input
                id="password"
                name="password"
                type="password"
                placeholder={t("common.placeholders.newPassword")}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                required
                autoFocus
              />
              <p className="text-xs text-gray-500">{t("auth.resetPassword.passwordHint")}</p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirmPassword">{t("common.labels.confirmPassword")}</Label>
              <Input
                id="confirmPassword"
                name="confirmPassword"
                type="password"
                placeholder={t("common.placeholders.confirmNewPassword")}
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                required
              />
            </div>

            {actionData?.error && (
              <div className="text-sm text-red-600 bg-red-50 p-3 rounded-md">
                {actionData.error}
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("common.buttons.resetting") : t("common.buttons.resetPassword")}
            </Button>

            <div className="text-center text-sm">
              <Link to="/login" className="text-blue-600 hover:underline">
                {t("common.buttons.backToLogin")}
              </Link>
            </div>
          </Form>
        </CardContent>
      </Card>
    </div>
  );
}
