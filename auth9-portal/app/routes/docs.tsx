import type { MetaFunction } from "react-router";
import { PublicPageLayout } from "~/components/PublicPageLayout";
import { ShowcaseCard } from "~/components/marketing/showcase-card";
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
        <ShowcaseCard
          title={t("docs.gettingStarted.title")}
          description={t("docs.gettingStarted.description")}
          headerExtra={<Badge variant="secondary">{t("docs.comingSoon")}</Badge>}
          contentClassName="min-h-[13rem]"
        />

        <ShowcaseCard
          title={t("docs.apiReference.title")}
          description={t("docs.apiReference.description")}
          headerExtra={<Badge variant="secondary">{t("docs.comingSoon")}</Badge>}
          contentClassName="min-h-[13rem]"
        />

        <a
          href="https://github.com/gpgkd906/auth9"
          target="_blank"
          rel="noopener noreferrer"
          className="block"
        >
          <ShowcaseCard
            className="transition-colors hover:border-[var(--accent-blue)]"
            title={t("docs.github.title")}
            description={t("docs.github.description")}
            headerExtra={
              <svg
                className="h-4 w-4 text-[var(--text-tertiary)]"
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
            }
            contentClassName="min-h-[13rem]"
          />
        </a>
      </div>
    </PublicPageLayout>
  );
}
