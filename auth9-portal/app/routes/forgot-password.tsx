import type { ActionFunctionArgs, MetaFunction } from "react-router";
import { Form, useActionData, useNavigation, Link } from "react-router";
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
  return buildMeta(resolveMetaLocale(matches), "auth.forgotPassword.metaTitle");
};

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const email = formData.get("email") as string;

  if (!email) {
    return { error: translate(locale, "auth.forgotPassword.emailRequired") };
  }

  try {
    await passwordApi.forgotPassword(email);
    return { success: true };
  } catch {
    // Don't reveal whether email exists - always show success
    return { success: true };
  }
}

export default function ForgotPasswordPage() {
  const { t } = useI18n();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [email, setEmail] = useState("");

  const isSubmitting = navigation.state === "submitting";

  if (actionData?.success) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 px-4 relative">
        <div className="fixed top-6 right-6 z-20 flex items-center gap-3">
          <LanguageSwitcher />
          <ThemeToggle />
        </div>
        <Card className="w-full max-w-md">
          <CardHeader className="text-center">
            <CardTitle>{t("auth.forgotPassword.successTitle")}</CardTitle>
            <CardDescription>
              {t("auth.forgotPassword.successDescription", { email })}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="text-sm text-gray-600 text-center">
              {t("auth.forgotPassword.successHint")}{" "}
              <Link to="/forgot-password" className="text-blue-600 hover:underline">
                {t("auth.forgotPassword.tryAgain")}
              </Link>
              .
            </p>
            <div className="text-center">
              <Link to="/login">
                <Button variant="outline">{t("common.buttons.backToLogin")}</Button>
              </Link>
            </div>
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
          <CardTitle>{t("auth.forgotPassword.title")}</CardTitle>
          <CardDescription>{t("auth.forgotPassword.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="email">{t("common.labels.emailAddress")}</Label>
              <Input
                id="email"
                name="email"
                type="email"
                placeholder={t("common.placeholders.email")}
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
                autoFocus
              />
            </div>

            {actionData?.error && (
              <div className="text-sm text-red-600 bg-red-50 p-3 rounded-md">
                {actionData.error}
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("common.buttons.sending") : t("common.buttons.sendResetLink")}
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
