import { useFetcher, Link } from "react-router";
import type { TenantUserWithTenant } from "~/services/api";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";

interface OrgSwitcherProps {
  tenants: TenantUserWithTenant[];
  activeTenantId?: string;
}

export function OrgSwitcher({ tenants, activeTenantId }: OrgSwitcherProps) {
  const fetcher = useFetcher();

  const activeTenant = tenants.find((t) => t.tenant_id === activeTenantId);
  const displayName = activeTenant?.tenant?.name || "Select organization";

  if (tenants.length <= 1 && activeTenant) {
    // Single tenant - just display, no switcher
    return (
      <div className="px-3 py-2 mb-1">
        <div className="flex items-center gap-2 text-sm">
          <div className="w-6 h-6 rounded bg-[var(--accent-blue)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
            {displayName.charAt(0).toUpperCase()}
          </div>
          <span className="text-[var(--text-secondary)] truncate font-medium text-xs">
            {displayName}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="px-3 py-2 mb-1">
      <DropdownMenu>
        <DropdownMenuTrigger className="w-full flex items-center gap-2 text-sm rounded-lg px-2 py-1.5 hover:bg-[var(--surface-secondary)] transition-colors outline-none">
          <div className="w-6 h-6 rounded bg-[var(--accent-blue)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
            {displayName.charAt(0).toUpperCase()}
          </div>
          <span className="text-[var(--text-secondary)] truncate font-medium text-xs flex-1 text-left">
            {displayName}
          </span>
          <ChevronDownIcon className="w-3 h-3 text-[var(--text-tertiary)] flex-shrink-0" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-56">
          {tenants.map((t) => (
            <DropdownMenuItem
              key={t.tenant_id}
              className={t.tenant_id === activeTenantId ? "bg-[var(--surface-secondary)]" : ""}
              onSelect={() => {
                if (t.tenant_id !== activeTenantId) {
                  fetcher.submit(
                    { intent: "switch-tenant", tenantId: t.tenant_id },
                    { method: "post", action: "/dashboard" }
                  );
                }
              }}
            >
              <div className="flex items-center gap-2 w-full">
                <div className="w-5 h-5 rounded bg-[var(--accent-blue)] flex items-center justify-center text-white text-[10px] font-bold flex-shrink-0">
                  {t.tenant.name.charAt(0).toUpperCase()}
                </div>
                <span className="truncate">{t.tenant.name}</span>
              </div>
            </DropdownMenuItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuItem asChild>
            <Link to="/onboard" className="text-[var(--accent-blue)]">
              + Create new organization
            </Link>
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

function ChevronDownIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
    </svg>
  );
}
