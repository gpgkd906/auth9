import type { MetaFunction } from "react-router";
import { PublicPageLayout } from "~/components/PublicPageLayout";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
} from "~/components/ui/card";
import { Badge } from "~/components/ui/badge";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(
    resolveMetaLocale(matches),
    "docs.metaTitle",
    "docs.metaDescription"
  );
};

export default function DocsPage() {
  const { t } = useI18n();

  return (
    <PublicPageLayout title={t("docs.title")}>
      <p className="mb-8 text-[var(--text-secondary)]">
        {t("docs.description")}
      </p>
      <div className="grid gap-4 sm:grid-cols-2">
        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <CardTitle>{t("docs.gettingStarted.title")}</CardTitle>
              <Badge variant="secondary">{t("docs.comingSoon")}</Badge>
            </div>
            <CardDescription>
              {t("docs.gettingStarted.description")}
            </CardDescription>
          </CardHeader>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center gap-2">
              <CardTitle>{t("docs.apiReference.title")}</CardTitle>
              <Badge variant="secondary">{t("docs.comingSoon")}</Badge>
            </div>
            <CardDescription>
              {t("docs.apiReference.description")}
            </CardDescription>
          </CardHeader>
        </Card>

        <a
          href="https://github.com/gpgkd906/auth9"
          target="_blank"
          rel="noopener noreferrer"
          className="block"
        >
          <Card className="h-full transition-colors hover:border-[var(--accent-blue)]">
            <CardHeader>
              <div className="flex items-center gap-2">
                <CardTitle>{t("docs.github.title")}</CardTitle>
                <svg
                  className="w-4 h-4 text-[var(--text-tertiary)]"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                  />
                </svg>
              </div>
              <CardDescription>
                {t("docs.github.description")}
              </CardDescription>
            </CardHeader>
          </Card>
        </a>
      </div>
    </PublicPageLayout>
  );
}
