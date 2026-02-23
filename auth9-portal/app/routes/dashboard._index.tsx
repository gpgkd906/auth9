import { useLoaderData, redirect, Link, useOutletContext } from "react-router";
import type { LoaderFunctionArgs } from "react-router";
import { ArrowRightIcon, PlusIcon } from "@radix-ui/react-icons";
import { getAccessToken } from "~/services/session.server";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { FormattedDate } from "~/components/ui/formatted-date";
import { auditApi, serviceApi, tenantApi, userApi } from "~/services/api";
import type { TenantUserWithTenant, User } from "~/services/api";

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "5");

  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  try {
    const [tenants, users, services, audits] = await Promise.all([
      tenantApi.list(1, 1, undefined, accessToken),
      userApi.list(1, 1, undefined, accessToken),
      serviceApi.list(undefined, 1, 1, accessToken),
      auditApi.list(page, perPage, accessToken),
    ]);

    return {
      totals: {
        tenants: tenants.pagination.total,
        users: users.pagination.total,
        services: services.pagination.total,
      },
      audits: audits.data,
    };
  } catch {
    // Handle case where user may not have permissions (e.g., no tenant association)
    return {
      totals: { tenants: 0, users: 0, services: 0 },
      audits: [],
    };
  }
}

type OutletContext = {
  activeTenant?: TenantUserWithTenant;
  tenants: TenantUserWithTenant[];
  currentUser?: User | null;
};

export default function DashboardIndex() {
  const data = useLoaderData<typeof loader>();
  const { activeTenant } = useOutletContext<OutletContext>();
  const tenantName = activeTenant?.tenant?.name || "Dashboard";

  return (
    <div className="space-y-6">
      <div className="animate-fade-in-up">
        <h1 className="mb-1 text-[28px] font-bold text-[var(--text-primary)] tracking-tight">{tenantName}</h1>
        <p className="mb-6 text-[15px] text-[var(--text-secondary)]">
          Welcome to {tenantName}. Here&apos;s an overview of your identity service.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatsCard title="Total Tenants" value={data.totals.tenants.toString()} color="blue" delay="delay-1" href="/dashboard/tenants" />
        <StatsCard title="Active Users" value={data.totals.users.toString()} color="purple" delay="delay-2" href="/dashboard/users" />
        <StatsCard title="Services" value={data.totals.services.toString()} color="green" delay="delay-3" href="/dashboard/services" />
        <StatsCard title="Audit Events" value={data.audits.length.toString()} color="cyan" delay="delay-4" href="/dashboard/audit-logs" />
      </div>

      {data.totals.tenants === 0 && (
        <Card className="animate-fade-in-up delay-5">
          <CardContent className="p-6 md:p-8">
            <div className="mx-auto max-w-2xl rounded-2xl border border-[var(--glass-border-subtle)] bg-[var(--glass-bg)] px-5 py-6 text-center md:px-8 md:py-7">
              <h2 className="text-lg font-semibold text-[var(--text-primary)]">Start by creating your first tenant</h2>
              <p className="mt-2 text-sm leading-relaxed text-[var(--text-secondary)]">
                Tenants isolate identities and policies for each environment or customer. Create one to unlock the rest of the dashboard workflow.
              </p>
              <Button asChild className="mt-5 h-11 w-full !flex px-4 md:h-10 md:w-auto">
                <Link to="/dashboard/tenants/new">
                  <PlusIcon className="mr-2 h-4 w-4" />
                  开始创建
                </Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <Card className="animate-fade-in-up delay-6">
        <CardHeader>
          <CardTitle>Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-0">
            {data.audits.map((activity) => (
              <div
                key={activity.id}
                className="flex items-start gap-3 py-3 border-b border-[var(--glass-border-subtle)] last:border-0"
              >
                <div className="w-2 h-2 rounded-full bg-[var(--accent-blue)] mt-1.5 shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="text-[13px] font-semibold leading-snug text-[var(--text-primary)]">
                    {activity.action} • {activity.resource_type}
                  </p>
                  <p className="text-[11px] text-[var(--text-tertiary)] mt-0.5">
                    <FormattedDate date={activity.created_at} />
                  </p>
                </div>
              </div>
            ))}
            {data.audits.length === 0 && (
              <div className="py-6 text-center text-sm text-[var(--text-tertiary)]">
                No recent activity
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function StatsCard({
  title,
  value,
  color,
  delay,
  href,
}: {
  title: string;
  value: string;
  color: "blue" | "purple" | "green" | "cyan";
  delay: string;
  href: string;
}) {
  const colorClasses = {
    blue: "from-[var(--accent-blue)]/20 to-transparent",
    purple: "from-[var(--accent-purple)]/20 to-transparent",
    green: "from-[var(--accent-green)]/20 to-transparent",
    cyan: "from-[var(--accent-cyan)]/20 to-transparent",
  };

  return (
    <Card className={`animate-fade-in-up ${delay} relative overflow-hidden h-full shadow-[0_12px_36px_var(--glass-shadow-strong),inset_0_1px_0_var(--glass-highlight),inset_0_-1px_0_rgba(0,0,0,0.05)] hover:shadow-[0_16px_44px_var(--glass-shadow-strong),inset_0_1px_0_var(--glass-highlight),inset_0_-1px_0_rgba(0,0,0,0.05)]`}>
      <div className={`absolute inset-0 bg-gradient-to-br ${colorClasses[color]} pointer-events-none`} />
      <CardContent className="pt-5 relative h-full flex flex-col">
        <p className="text-[13px] font-medium text-[var(--text-secondary)]">{title}</p>
        <p className="mt-1 text-[28px] font-bold text-[var(--text-primary)] tracking-tight">{value}</p>
        <Button asChild variant="outline" size="sm" className="mt-4 -mx-5 -mb-5 h-11 w-[calc(100%+2.5rem)] rounded-t-none !flex justify-between px-4 text-xs sm:mt-3 sm:mx-0 sm:mb-0 sm:h-8 sm:w-auto sm:rounded-md sm:gap-1 sm:px-3 sm:justify-center">
          <Link to={href}>
            View details
            <ArrowRightIcon className="h-3.5 w-3.5" />
          </Link>
        </Button>
      </CardContent>
    </Card>
  );
}
