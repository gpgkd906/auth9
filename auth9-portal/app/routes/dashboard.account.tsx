import type { MetaFunction } from "react-router";
import { Link, Outlet, useLocation } from "react-router";
import { cn } from "~/lib/utils";

export const meta: MetaFunction = () => {
  return [{ title: "Account - Auth9" }];
};

const accountNav = [
  { name: "Profile", href: "/dashboard/account", description: "Your personal information" },
  { name: "Security", href: "/dashboard/account/security", description: "Change your password" },
  { name: "Passkeys", href: "/dashboard/account/passkeys", description: "Passwordless authentication" },
  { name: "Sessions", href: "/dashboard/account/sessions", description: "Active sessions" },
  { name: "Linked Identities", href: "/dashboard/account/identities", description: "Connected accounts" },
];

export default function AccountLayout() {
  const location = useLocation();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Account</h1>
        <p className="text-sm text-[var(--text-secondary)]">Manage your personal account settings</p>
      </div>

      <div className="flex gap-6">
        {/* Account Navigation */}
        <nav className="w-48 flex-shrink-0">
          <ul className="space-y-1">
            {accountNav.map((item) => {
              const isActive = location.pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    to={item.href}
                    className={cn(
                      "block px-3 py-2 rounded-xl text-sm transition-colors",
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

        {/* Account Content */}
        <div className="flex-1 min-w-0">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
