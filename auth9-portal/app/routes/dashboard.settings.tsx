import type { MetaFunction } from "react-router";
import { Link, Outlet, useLocation } from "react-router";
import { cn } from "~/lib/utils";

export const meta: MetaFunction = () => {
  return [{ title: "Settings - Auth9" }];
};

const settingsNav = [
  { name: "Organization", href: "/dashboard/settings", description: "Tenant branding settings" },
  { name: "Login Branding", href: "/dashboard/settings/branding", description: "Customize login pages" },
  { name: "Email Provider", href: "/dashboard/settings/email", description: "Email delivery configuration" },
  { name: "Email Templates", href: "/dashboard/settings/email-templates", description: "Customize email content" },
  { name: "Password Policy", href: "/dashboard/settings/security", description: "Password requirements" },
  { name: "Identity Providers", href: "/dashboard/settings/identity-providers", description: "Social login and SSO" },
];

export default function SettingsLayout() {
  const location = useLocation();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Settings</h1>
        <p className="text-sm text-[var(--text-secondary)]">Manage system and organization preferences</p>
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
