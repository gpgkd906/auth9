import { useState } from "react";
import type { LoaderFunctionArgs, ActionFunctionArgs, MetaFunction } from "react-router";
import { Link, Outlet, useLocation, useLoaderData, useRouteError, isRouteErrorResponse, redirect } from "react-router";
import { cn } from "~/lib/utils";
import { Avatar, AvatarFallback, AvatarImage } from "~/components/ui/avatar";
import { ThemeToggle } from "~/components/ThemeToggle";
import { LanguageSwitcher } from "~/components/LanguageSwitcher";
import { OrgSwitcher } from "~/components/OrgSwitcher";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { useI18n } from "~/i18n";
import { requireTenantAuthWithUpdate, trySetActiveTenant, NO_STORE_HEADERS } from "~/services/session.server";
import { userApi, type User, type TenantUserWithTenant } from "~/services/api";

export const meta: MetaFunction<typeof loader> = ({ data, matches }) => {
  const locale = resolveMetaLocale(matches);
  const tenantName = data?.activeTenant?.tenant?.name || translate(locale, "dashboard.fallbackTitle");
  return buildMeta(locale, "dashboard.metaTitle", undefined, { tenantName });
};

// Protect all dashboard routes - requires authenticated user with active tenant token.
// If identity auth fails → redirect to /login.
// If no active tenant or tenant token exchange fails → redirect to /tenant/select.
export async function loader({ request }: LoaderFunctionArgs) {
  const { session, headers } = await requireTenantAuthWithUpdate(request);
  const identityToken = session.identityAccessToken || session.accessToken;

  let currentUser: User | null = null;
  try {
    const response = await userApi.getMe(identityToken);
    currentUser = response.data;
  } catch {
    // fallback to null
  }

  let tenants: TenantUserWithTenant[] = [];
  try {
    const serviceId = process.env.AUTH9_PORTAL_CLIENT_ID || "auth9-portal";
    const res = await userApi.getMyTenants(identityToken, serviceId);
    tenants = res.data;
  } catch {
    // API may be down — org switcher will show empty list
  }

  const activeTenantId = session.activeTenantId;
  const activeTenant = tenants.find((t) => t.tenant_id === activeTenantId);

  if (activeTenant?.tenant?.status === "pending") {
    throw redirect("/onboard/pending", { headers: NO_STORE_HEADERS });
  }

  const data = { currentUser, tenants, activeTenant, activeTenantId };

  if (headers) {
    return Response.json(data, { headers });
  }
  return data;
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  if (intent === "switch-tenant") {
    const tenantId = formData.get("tenantId") as string;
    if (tenantId) {
      const result = await trySetActiveTenant(request, tenantId);
      if ("error" in result) {
        return { error: result.error };
      }
      // Return JSON (not redirect) so the Set-Cookie header is reliably
      // delivered to the browser. The OrgSwitcher component detects this
      // success response and performs a hard navigation (window.location)
      // to ensure the loader reads the updated cookie.
      return Response.json({ ok: true }, {
        headers: { "Set-Cookie": result.cookie },
      });
    }
  }

  return null;
}

export default function Dashboard() {
  const { t } = useI18n();
  const location = useLocation();
  const { currentUser, tenants, activeTenant, activeTenantId } = useLoaderData<typeof loader>() as {
    currentUser: User | null;
    tenants: TenantUserWithTenant[];
    activeTenant: TenantUserWithTenant | undefined;
    activeTenantId: string | undefined;
  };
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);

  const navigation = [
    { name: t("dashboard.nav.overview"), href: "/dashboard", icon: HomeIcon },
    { name: t("dashboard.nav.tenants"), href: "/dashboard/tenants", icon: BuildingIcon },
    { name: t("dashboard.nav.users"), href: "/dashboard/users", icon: UsersIcon },
    { name: t("dashboard.nav.services"), href: "/dashboard/services", icon: ServerIcon },
    { name: t("dashboard.nav.roles"), href: "/dashboard/roles", icon: ShieldIcon },
    { name: t("dashboard.nav.abac"), href: "/dashboard/abac", icon: SlidersIcon },
    { name: t("dashboard.nav.analytics"), href: "/dashboard/analytics", icon: ChartIcon },
    { name: t("dashboard.nav.security"), href: "/dashboard/security/alerts", icon: LockIcon },
    { name: t("dashboard.nav.securityRisk"), href: "/dashboard/security/risk", icon: ShieldIcon },
    { name: t("dashboard.nav.auditLogs"), href: "/dashboard/audit-logs", icon: ClipboardIcon },
    { name: t("dashboard.nav.settings"), href: "/dashboard/settings", icon: SettingsIcon },
  ];

  const displayName = currentUser?.display_name || currentUser?.email || t("dashboard.userFallback");
  const email = currentUser?.email || "";
  const avatarUrl = currentUser?.avatar_url || "";
  const initials = displayName
    .split(" ")
    .map((n) => n[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  return (
    <div className="min-h-screen">
      {/* Skip to Content */}
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4 focus:z-[100] focus:px-4 focus:py-2 focus:bg-[var(--accent-blue)] focus:text-white focus:rounded-lg focus:shadow-lg focus:ring-2 focus:ring-[var(--accent-blue)] focus:ring-offset-2"
      >
        {t("common.navigation.skipToMain")}
      </a>

      {/* Dynamic Background */}
      <div className="page-backdrop" />

      {/* Mobile Header */}
      <header aria-label={t("common.navigation.mobileHeader")} className="lg:hidden fixed top-0 left-0 right-0 h-16 z-[60] px-4 flex items-center justify-between bg-[var(--glass-bg)] backdrop-blur-md border-b border-[var(--glass-border-subtle)]">
        <Link to="/dashboard" className="flex items-center gap-2">
          <div className="logo-icon w-8 h-8 text-sm">A9</div>
          <span className="logo-text text-lg">Auth9</span>
        </Link>
        <button
          onClick={() => setIsSidebarOpen(!isSidebarOpen)}
          className="h-11 w-11 inline-flex items-center justify-center rounded-lg text-[var(--text-secondary)] hover:bg-[var(--glass-border-subtle)] transition-colors"
          aria-label={isSidebarOpen ? t("common.navigation.closeSidebar") : t("common.navigation.openSidebar")}
        >
          {isSidebarOpen ? (
            <XIcon className="w-6 h-6" />
          ) : (
            <MenuIcon className="w-6 h-6" />
          )}
        </button>
      </header>

      {/* Sidebar Overlay */}
      {isSidebarOpen && (
        <div
          className="lg:hidden fixed inset-0 z-40 bg-black/50 backdrop-blur-sm transition-opacity"
          onClick={() => setIsSidebarOpen(false)}
        />
      )}

      {/* Sidebar - Floating Glass Card */}
      <aside
        aria-label={t("common.navigation.sidebar")}
        className={cn(
          "sidebar",
          isSidebarOpen && "open"
        )}
      >
        {/* Logo */}
        <div className="sidebar-header">
          <Link to="/dashboard" className="flex items-center gap-3">
            <div className="logo-icon">A9</div>
            <span className="logo-text">Auth9</span>
          </Link>
        </div>

        {/* Org Switcher */}
        <OrgSwitcher tenants={tenants} activeTenantId={activeTenantId} />

        {/* Navigation */}
        <nav className="sidebar-nav" aria-label={t("common.navigation.mainNavigation")}>
          <div className="nav-section">
            <div className="nav-section-title">{t("dashboard.nav.main")}</div>
            {navigation.slice(0, 4).map((item) => {
              const isActive = location.pathname === item.href ||
                (item.href !== "/dashboard" && location.pathname.startsWith(item.href));

              return (
                <Link
                  key={item.name}
                  to={item.href}
                  onClick={() => setIsSidebarOpen(false)}
                  className={cn(
                    "sidebar-item",
                    isActive && "active"
                  )}
                  aria-current={isActive ? "page" : undefined}
                >
                  <item.icon className="w-5 h-5" />
                  {item.name}
                </Link>
              );
            })}
          </div>

          <div className="nav-section">
            <div className="nav-section-title">{t("dashboard.nav.securityGroup")}</div>
            {navigation.slice(4, 10).map((item) => {
              const isActive = location.pathname === item.href ||
                (item.href !== "/dashboard" && location.pathname.startsWith(item.href));

              return (
                <Link
                  key={item.name}
                  to={item.href}
                  onClick={() => setIsSidebarOpen(false)}
                  className={cn(
                    "sidebar-item",
                    isActive && "active"
                  )}
                  aria-current={isActive ? "page" : undefined}
                >
                  <item.icon className="w-5 h-5" />
                  {item.name}
                </Link>
              );
            })}
          </div>

          <div className="nav-section">
            <div className="nav-section-title">{t("dashboard.nav.system")}</div>
            {navigation.slice(10).map((item) => {
              const isActive = location.pathname === item.href ||
                (item.href !== "/dashboard" && location.pathname.startsWith(item.href));

              return (
                <Link
                  key={item.name}
                  to={item.href}
                  onClick={() => setIsSidebarOpen(false)}
                  className={cn(
                    "sidebar-item",
                    isActive && "active"
                  )}
                  aria-current={isActive ? "page" : undefined}
                >
                  <item.icon className="w-5 h-5" />
                  {item.name}
                </Link>
              );
            })}
          </div>
        </nav>

        {/* Preferences & User */}
        <div className="sidebar-footer">
          <div className="flex items-center justify-center gap-2 pb-3 mb-3 border-b border-[var(--glass-border-subtle)]">
            <LanguageSwitcher />
            <ThemeToggle />
          </div>
          <div className="user-card">
            <Link to="/dashboard/account" className="flex items-center gap-3 flex-1 min-w-0" onClick={() => setIsSidebarOpen(false)}>
              <Avatar>
                <AvatarImage src={avatarUrl} />
                <AvatarFallback>{initials}</AvatarFallback>
              </Avatar>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-[var(--text-primary)] truncate">
                  {displayName}
                </p>
                <p className="text-xs text-[var(--text-tertiary)] truncate">
                  {email}
                </p>
              </div>
            </Link>
            <Link
              to="/logout"
              onClick={(e) => {
                if (!window.confirm(t("common.buttons.signOutConfirm"))) {
                  e.preventDefault();
                }
              }}
              className="p-2 rounded-lg text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-[var(--surface-secondary)] transition-colors"
              title={t("common.buttons.signOut")}
              aria-label={t("common.buttons.signOut")}
            >
              <LogOutIcon className="w-4 h-4" />
            </Link>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main id="main-content" aria-label={t("common.navigation.mainContent")} className="main-content pt-20 lg:pt-0" tabIndex={-1}>
        <div className="content-wrapper">
          <Outlet context={{ activeTenant, tenants, currentUser }} />
        </div>
      </main>
      <footer role="contentinfo" className="sr-only">
        Auth9 dashboard footer
      </footer>
    </div>
  );
}

// Dashboard-level ErrorBoundary — renders errors within the dashboard shell
export function ErrorBoundary() {
  const error = useRouteError();
  const { t } = useI18n();

  const status = isRouteErrorResponse(error) ? error.status : 500;
  const title = status === 404 ? t("common.errors.pageNotFound") : String(status);
  const message = status === 404
    ? t("common.errors.pageNotFound")
    : t("common.errors.somethingWentWrong");

  return (
    <div className="flex flex-col items-center justify-center py-24 px-6 text-center">
      <h1 className="text-6xl font-bold text-[var(--text-primary)]">{title}</h1>
      <p className="mt-4 text-xl text-[var(--text-secondary)]">{message}</p>
      <Link
        to="/dashboard"
        className="mt-8 inline-block px-6 py-3 bg-[var(--accent-blue)] text-white rounded-[12px] font-medium hover:opacity-90 transition-opacity"
      >
        {t("common.errors.goBackHome")}
      </Link>
    </div>
  );
}

// Icons
function MenuIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
    </svg>
  );
}

function XIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function HomeIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
    </svg>
  );
}

function BuildingIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
    </svg>
  );
}

function UsersIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
    </svg>
  );
}

function ServerIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01" />
    </svg>
  );
}

function ShieldIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
    </svg>
  );
}

function ClipboardIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01" />
    </svg>
  );
}

function SettingsIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    </svg>
  );
}

function ChartIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
    </svg>
  );
}

function LockIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
    </svg>
  );
}

function SlidersIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 6h8m4 0h4M10 6a2 2 0 11-4 0 2 2 0 014 0zm10 12h-8m-4 0H4m10 0a2 2 0 11-4 0 2 2 0 014 0zM4 12h4m4 0h8m-8 0a2 2 0 11-4 0 2 2 0 014 0z" />
    </svg>
  );
}

function LogOutIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
    </svg>
  );
}
