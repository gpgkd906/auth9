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
];

export default function SettingsLayout() {
  const location = useLocation();

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Settings</h1>
        <p className="text-sm text-gray-500">Manage system and organization preferences</p>
      </div>

      <div className="flex gap-6">
        {/* Settings Navigation */}
        <nav className="w-48 flex-shrink-0">
          <ul className="space-y-1">
            {settingsNav.map((item) => {
              const isActive = location.pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    to={item.href}
                    className={cn(
                      "block px-3 py-2 rounded-apple text-sm transition-colors",
                      isActive
                        ? "bg-apple-blue text-white"
                        : "text-gray-700 hover:bg-gray-100"
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
