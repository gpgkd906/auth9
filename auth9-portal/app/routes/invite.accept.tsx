import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { resolveLocale } from "~/services/locale.server";
import { invitationApi } from "~/services/api";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "invite.metaTitle");
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
    const locale = await resolveLocale(request);
    return Response.json({ error: translate(locale, "invite.missingToken") }, { status: 400 });
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
    const locale = await resolveLocale(request);
    const message =
      error instanceof Error ? error.message : translate(locale, "invite.unknownError");
    return Response.json({ error: message }, { status: 400 });
  }
}

export default function InviteAcceptPage() {
  const { t } = useI18n();
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
            <CardTitle>{t("invite.invalidTitle")}</CardTitle>
            <CardDescription>{t("invite.invalidDescription")}</CardDescription>
          </CardHeader>
          <CardContent className="text-center">
            <Link to="/login" className="text-[var(--accent-blue)] hover:underline text-sm">
              {t("invite.goToLogin")}
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
          <CardTitle>{t("invite.title")}</CardTitle>
          <CardDescription>
            {t("invite.description")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="token" value={token} />

            <div className="space-y-2">
              <Label htmlFor="email">{t("invite.emailOptional")}</Label>
              <Input id="email" name="email" type="email" placeholder={t("invite.emailPlaceholder")} />
            </div>

            <div className="space-y-2">
              <Label htmlFor="display_name">{t("invite.displayName")}</Label>
              <Input id="display_name" name="display_name" placeholder={t("invite.displayNamePlaceholder")} />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">{t("invite.password")}</Label>
              <Input id="password" name="password" type="password" placeholder={t("invite.passwordPlaceholder")} />
              <p className="text-xs text-[var(--text-tertiary)]">
                {t("invite.passwordHint")}
              </p>
            </div>

            {actionData && "error" in actionData && (
              <p className="text-sm text-[var(--accent-red)]">{String(actionData.error)}</p>
            )}

            {actionData && "success" in actionData && actionData.success && (
              <div className="rounded-xl bg-[var(--accent-green)]/10 border border-[var(--accent-green)]/20 p-3 text-sm text-[var(--accent-green)]">
                {t("invite.accepted")}
              </div>
            )}

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? t("invite.submitting") : t("invite.accept")}
            </Button>
          </Form>

          <div className="mt-6 text-center text-sm text-[var(--text-tertiary)]">
            {t("invite.existingAccount")}{" "}
            <Link to={`/login?invite_token=${encodeURIComponent(token)}`} className="text-[var(--accent-blue)] hover:underline font-medium">
              {t("invite.signIn")}
            </Link>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
