import { useState, useEffect } from "react";
import type { MetaFunction, ActionFunctionArgs } from "react-router";
import {
  redirect,
  Form,
  useActionData,
  useNavigation,
  Link,
  useOutletContext,
} from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { mapApiError } from "~/lib/error-messages";
import { requireAuthWithUpdate, commitSession, setActiveTenant } from "~/services/session.server";
import { organizationApi } from "~/services/api";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "onboarding.createMetaTitle");
};

export async function action({ request }: ActionFunctionArgs) {
  const { session } = await requireAuthWithUpdate(request);
  const identityToken = session.identityAccessToken || session.accessToken;
  if (!identityToken) {
    throw redirect("/login");
  }
  const formData = await request.formData();

  const name = (formData.get("name") as string || "").trim();
  const slug = (formData.get("slug") as string || "").trim();
  const domain = (formData.get("domain") as string || "").trim();

  if (!name || !slug || !domain) {
    const locale = await resolveLocale(request);
    return { error: locale === "zh-CN" ? "所有字段均为必填项" : "All fields are required" };
  }

  try {
    const result = await organizationApi.create(
      { name, slug, domain },
      identityToken
    );

    const org = result.data;

    // Update session with new active tenant
    if (org.status === "pending") {
      const updatedSession = { ...session, activeTenantId: org.id };
      return redirect("/onboard/pending", {
        headers: { "Set-Cookie": await commitSession(updatedSession) },
      });
    }

    const tenantCookie = await setActiveTenant(request, org.id);
    return redirect("/dashboard", {
      headers: { "Set-Cookie": tenantCookie },
    });
  } catch (error) {
    const locale = await resolveLocale(request);
    const message = mapApiError(error, locale);
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

export default function OnboardIndex() {
  const { t } = useI18n();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const { email } = useOutletContext<{ email: string }>();

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugManuallyEdited, setSlugManuallyEdited] = useState(false);
  const [domain, setDomain] = useState("");
  const [domainManuallyEdited, setDomainManuallyEdited] = useState(false);

  useEffect(() => {
    if (!slugManuallyEdited && name) {
      setSlug(slugify(name));
    }
  }, [name, slugManuallyEdited]);

  useEffect(() => {
    if (domainManuallyEdited) {
      return;
    }

    const emailDomain = email.split("@")[1]?.toLowerCase() || "";
    setDomain(emailDomain);
  }, [domainManuallyEdited, email]);

  return (
    <Card className="w-full max-w-lg relative z-10 animate-fade-in-up">
      <CardHeader className="text-center">
        <div className="logo-icon mx-auto mb-4">A9</div>
        <CardTitle className="text-2xl">{t("onboarding.createTitle")}</CardTitle>
        <CardDescription>
          {t("onboarding.createDescription")}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Form method="post" className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="name">{t("onboarding.organizationName")}</Label>
            <Input
              id="name"
              name="name"
              required
              placeholder={t("onboarding.organizationNamePlaceholder")}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="slug">{t("onboarding.slug")}</Label>
            <Input
              id="slug"
              name="slug"
              required
              placeholder={t("onboarding.slugPlaceholder")}
              value={slug}
              onChange={(e) => {
                setSlug(e.target.value);
                setSlugManuallyEdited(true);
              }}
            />
            <p className="text-xs text-[var(--text-tertiary)]">
              {t("onboarding.slugHint")}
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="domain">{t("onboarding.emailDomain")}</Label>
            <Input
              id="domain"
              name="domain"
              required
              placeholder={t("onboarding.emailDomainPlaceholder")}
              value={domain}
              onChange={(e) => {
                setDomain(e.target.value);
                setDomainManuallyEdited(true);
              }}
            />
            <p className="text-xs text-[var(--text-tertiary)]">
              {t("onboarding.emailDomainHint")}
            </p>
          </div>

          {actionData?.error && (
            <p className="text-sm text-[var(--accent-red)]">{actionData.error}</p>
          )}

          <Button type="submit" className="w-full" disabled={isSubmitting}>
            {isSubmitting ? t("onboarding.creating") : t("onboarding.create")}
          </Button>
        </Form>

        <div className="mt-4 text-center space-y-2">
          <div>
            <Link
              to="/onboard/invitation"
              className="text-sm text-[var(--accent-blue)] hover:underline transition-colors"
            >
              {t("onboarding.waitingForInvitation")}
            </Link>
          </div>
          <div>
            <Link
              to="/logout"
              className="text-sm text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] transition-colors"
            >
              {t("onboarding.signOut")}
            </Link>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
