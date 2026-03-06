import type { MetaFunction } from "react-router";
import { Link } from "react-router";
import { LanguageSwitcher } from "~/components/LanguageSwitcher";
import { Button } from "~/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { ThemeToggle } from "~/components/ThemeToggle";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(
    resolveMetaLocale(matches),
    "home.meta.title",
    "home.meta.description"
  );
};

export default function Index() {
  const { t } = useI18n();
  return (
    <div className="min-h-screen relative">
      {/* Dynamic Background */}
      <div className="page-backdrop" />

      {/* Theme Toggle - fixed position */}
      <div className="fixed top-6 right-6 z-[60] flex items-center gap-3">
        <LanguageSwitcher />
        <ThemeToggle />
      </div>

      {/* Header */}
      <header className="fixed top-0 left-0 right-0 z-50 liquid-glass border-b border-[var(--glass-border-subtle)]">
        <div className="max-w-7xl mx-auto px-6 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="logo-icon">A9</div>
            <span className="text-xl font-semibold text-[var(--text-primary)]">Auth9</span>
          </div>
          <nav className="flex items-center gap-6">
            <Link to="/login" className="text-[var(--text-secondary)] hover:text-[var(--text-primary)] font-medium transition-colors">
              {t("home.nav.signIn")}
            </Link>
            <Button asChild>
              <Link to="/register">{t("home.nav.getStarted")}</Link>
            </Button>
          </nav>
        </div>
      </header>

      {/* Hero */}
      <main className="pt-32 pb-20 px-6 relative z-10">
        <div className="max-w-4xl mx-auto text-center animate-fade-in-up">
          <h1 className="text-5xl md:text-6xl font-bold tracking-tight text-[var(--text-primary)]">
            {t("home.hero.titleLine1")}
            <br />
            <span className="bg-gradient-to-r from-[var(--accent-cyan)] via-[var(--accent-blue)] to-[var(--accent-purple)] bg-clip-text text-transparent">
              {t("home.hero.titleLine2")}
            </span>
          </h1>
          <p className="mt-6 text-xl text-[var(--text-secondary)] max-w-2xl mx-auto">
            {t("home.hero.description")}
          </p>
          <div className="mt-10 flex items-center justify-center gap-4">
            <Button size="lg" asChild>
              <Link to="/register">{t("home.hero.startTrial")}</Link>
            </Button>
            <Button size="lg" variant="glass" asChild>
              <Link to="/docs">{t("home.hero.readDocs")}</Link>
            </Button>
          </div>
        </div>

        {/* Features */}
        <div className="mt-32 max-w-6xl mx-auto grid md:grid-cols-3 gap-6">
          <Card className="animate-fade-in-up delay-1">
            <CardHeader>
              <div className="w-12 h-12 rounded-2xl bg-[var(--accent-blue)]/10 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-[var(--accent-blue)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                </svg>
              </div>
              <CardTitle>{t("home.features.ssoTitle")}</CardTitle>
              <CardDescription>
                {t("home.features.ssoDescription")}
              </CardDescription>
            </CardHeader>
          </Card>

          <Card className="animate-fade-in-up delay-2">
            <CardHeader>
              <div className="w-12 h-12 rounded-2xl bg-[var(--accent-purple)]/10 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-[var(--accent-purple)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                </svg>
              </div>
              <CardTitle>{t("home.features.tenantTitle")}</CardTitle>
              <CardDescription>
                {t("home.features.tenantDescription")}
              </CardDescription>
            </CardHeader>
          </Card>

          <Card className="animate-fade-in-up delay-3">
            <CardHeader>
              <div className="w-12 h-12 rounded-2xl bg-[var(--accent-green)]/10 flex items-center justify-center mb-4">
                <svg className="w-6 h-6 text-[var(--accent-green)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                </svg>
              </div>
              <CardTitle>{t("home.features.rbacTitle")}</CardTitle>
              <CardDescription>
                {t("home.features.rbacDescription")}
              </CardDescription>
            </CardHeader>
          </Card>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-[var(--glass-border-subtle)] py-12 px-6 relative z-10">
        <div className="max-w-6xl mx-auto flex items-center justify-between">
          <p className="text-[var(--text-tertiary)] text-sm">
            {t("home.footer.copyright")}
          </p>
          <div className="flex items-center gap-6">
            <Link to="/privacy" className="text-[var(--text-tertiary)] hover:text-[var(--text-primary)] text-sm transition-colors">
              {t("home.footer.privacy")}
            </Link>
            <Link to="/terms" className="text-[var(--text-tertiary)] hover:text-[var(--text-primary)] text-sm transition-colors">
              {t("home.footer.terms")}
            </Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
