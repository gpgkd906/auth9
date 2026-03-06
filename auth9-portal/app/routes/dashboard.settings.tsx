import type { MetaFunction } from "react-router";
import { Link, Outlet, useLocation } from "react-router";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { cn } from "~/lib/utils";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "settings.metaTitle");
};

export default function SettingsLayout() {
  const location = useLocation();
  const { t } = useI18n();
  const settingsNav = [
    { name: t("settings.nav.organization"), href: "/dashboard/settings", description: t("settings.navDescriptions.organization") },
    { name: t("settings.nav.branding"), href: "/dashboard/settings/branding", description: t("settings.navDescriptions.branding") },
    { name: t("settings.nav.email"), href: "/dashboard/settings/email", description: t("settings.navDescriptions.email") },
    { name: t("settings.nav.emailTemplates"), href: "/dashboard/settings/email-templates", description: t("settings.navDescriptions.emailTemplates") },
    { name: t("settings.nav.security"), href: "/dashboard/settings/security", description: t("settings.navDescriptions.security") },
    { name: t("settings.nav.identityProviders"), href: "/dashboard/settings/identity-providers", description: t("settings.navDescriptions.identityProviders") },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("settings.title")}</h1>
        <p className="text-sm text-[var(--text-secondary)]">{t("settings.description")}</p>
      </div>

      <div className="flex flex-col gap-4 md:flex-row md:gap-6">
        {/* Settings Navigation */}
        <nav className="w-full flex-shrink-0 md:w-48">
          <ul className="space-y-1">
            {settingsNav.map((item) => {
              const isActive = location.pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    to={item.href}
                    className={cn(
                      "flex min-h-11 items-center rounded-xl px-3 text-sm transition-colors",
                      isActive
                        ? "bg-[var(--accent-blue)] text-white"
                        : "text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
                    )}
                  >
                    {item.name}
                  </Link>
                </li>
              );
            })}
          </ul>
        </nav>

        {/* Settings Content */}
        <div className="flex-1 min-w-0">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
