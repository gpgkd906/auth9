import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { mapApiError } from "~/lib/error-messages";
import { resolveLocale } from "~/services/locale.server";
import { invitationApi } from "~/services/api";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "invite.metaTitle");
};

type InvitationState = "pending" | "accepted" | "expired" | "revoked" | "invalid" | null;

interface LoaderData {
  token: string | null;
  invitationStatus: InvitationState;
  invitationEmail?: string;
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const token = url.searchParams.get("token");
  if (!token) {
    return { token: null, invitationStatus: null } satisfies LoaderData;
  }

  try {
    const result = await invitationApi.validate(token);
    return {
      token,
      invitationStatus: result.data.status as InvitationState,
      invitationEmail: result.data.email,
    } satisfies LoaderData;
  } catch {
    return { token, invitationStatus: "invalid" as InvitationState } satisfies LoaderData;
  }
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

    return redirect("/login?invite_accepted=true");
  } catch (error) {
    const locale = await resolveLocale(request);
    const message = mapApiError(error, locale);
    return Response.json({ error: message }, { status: 400 });
  }
}

function InvitationStatusCard({ titleKey, descriptionKey }: { titleKey: string; descriptionKey: string }) {
  const { t } = useI18n();
  return (
    <div className="min-h-screen flex items-center justify-center px-6 relative">
      <div className="page-backdrop" />
      <Card className="w-full max-w-md relative z-10">
        <CardHeader className="text-center">
          <CardTitle>{t(titleKey)}</CardTitle>
          <CardDescription>{t(descriptionKey)}</CardDescription>
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

export default function InviteAcceptPage() {
  const { t } = useI18n();
  const { token, invitationStatus } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>() as { error?: string } | undefined;
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  if (!token || invitationStatus === "invalid") {
    return <InvitationStatusCard titleKey="invite.invalidTitle" descriptionKey="invite.invalidDescription" />;
  }

  if (invitationStatus === "expired") {
    return <InvitationStatusCard titleKey="invite.expiredTitle" descriptionKey="invite.expiredDescription" />;
  }

  if (invitationStatus === "accepted") {
    return <InvitationStatusCard titleKey="invite.acceptedTitle" descriptionKey="invite.acceptedDescription" />;
  }

  if (invitationStatus === "revoked") {
    return <InvitationStatusCard titleKey="invite.revokedTitle" descriptionKey="invite.revokedDescription" />;
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
