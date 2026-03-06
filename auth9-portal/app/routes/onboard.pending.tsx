import type { MetaFunction, LoaderFunctionArgs } from "react-router";
import { Link } from "react-router";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { requireAuthWithUpdate } from "~/services/session.server";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "onboarding.pendingMetaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  await requireAuthWithUpdate(request);
  return {};
}

export default function OnboardPending() {
  const { t } = useI18n();
  return (
    <Card className="w-full max-w-lg relative z-10 animate-fade-in-up">
      <CardHeader className="text-center">
        <div className="logo-icon mx-auto mb-4">A9</div>
        <CardTitle className="text-2xl">{t("onboarding.pendingTitle")}</CardTitle>
        <CardDescription>
          {t("onboarding.pendingDescription")}
        </CardDescription>
      </CardHeader>
      <CardContent className="text-center space-y-4">
        <p className="text-sm text-[var(--text-secondary)]">
          {t("onboarding.pendingHint")}
        </p>

        <div className="flex flex-col gap-2">
          <Button variant="outline" asChild>
            <Link to="/onboard">
              {t("onboarding.tryAnotherDomain")}
            </Link>
          </Button>
          <Button variant="outline" asChild>
            <Link to="/logout">
              {t("onboarding.signOut")}
            </Link>
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
