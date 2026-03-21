import type { MetaFunction } from "react-router";
import { Link, Outlet, useLocation } from "react-router";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { cn } from "~/lib/utils";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "account.metaTitle");
};

export default function AccountLayout() {
  const location = useLocation();
  const { t } = useI18n();
  const accountNav = [
    { name: t("account.nav.profile"), href: "/dashboard/account", description: t("account.navDescriptions.profile") },
    { name: t("account.nav.security"), href: "/dashboard/account/security", description: t("account.navDescriptions.security") },
    { name: t("account.nav.mfa"), href: "/dashboard/account/mfa", description: t("account.navDescriptions.mfa") },
    { name: t("account.nav.passkeys"), href: "/dashboard/account/passkeys", description: t("account.navDescriptions.passkeys") },
    { name: t("account.nav.sessions"), href: "/dashboard/account/sessions", description: t("account.navDescriptions.sessions") },
    { name: t("account.nav.identities"), href: "/dashboard/account/identities", description: t("account.navDescriptions.identities") },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("account.title")}</h1>
        <p className="text-sm text-[var(--text-secondary)]">{t("account.description")}</p>
      </div>

      <div className="flex flex-col gap-x-6 gap-y-4 lg:flex-row">
        {/* Account Navigation */}
        <nav className="w-full flex-shrink-0 lg:w-48">
          <ul className="grid gap-1 sm:grid-cols-2 lg:grid-cols-1">
            {accountNav.map((item) => {
              const isActive = location.pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    to={item.href}
                    className={cn(
                      "block rounded-xl px-3 py-2 text-sm font-medium transition-colors",
                      isActive
                        ? "bg-[var(--accent-blue-light)] text-[var(--accent-blue)]"
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

        {/* Account Content */}
        <div className="flex-1 min-w-0">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
