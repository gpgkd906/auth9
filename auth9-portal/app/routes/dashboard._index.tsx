import { useLoaderData } from "react-router";
import type { LoaderFunctionArgs } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { auditApi, serviceApi, tenantApi, userApi } from "~/services/api";

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "5");

  const [tenants, users, services, audits] = await Promise.all([
    tenantApi.list(1, 1),
    userApi.list(1, 1),
    serviceApi.list(undefined, 1, 1),
    auditApi.list(page, perPage),
  ]);

  return {
    totals: {
      tenants: tenants.pagination.total,
      users: users.pagination.total,
      services: services.pagination.total,
    },
    audits: audits.data,
  };
}

export default function DashboardIndex() {
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div className="animate-fade-in-up">
        <h1 className="text-[28px] font-bold text-[var(--text-primary)] tracking-tight">Dashboard</h1>
        <p className="mt-1 text-[15px] text-[var(--text-secondary)]">
          Welcome to Auth9. Here&apos;s an overview of your identity service.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatsCard title="Total Tenants" value={data.totals.tenants.toString()} color="blue" delay="delay-1" />
        <StatsCard title="Active Users" value={data.totals.users.toString()} color="purple" delay="delay-2" />
        <StatsCard title="Services" value={data.totals.services.toString()} color="green" delay="delay-3" />
        <StatsCard title="Audit Events" value={data.audits.length.toString()} color="cyan" delay="delay-4" />
      </div>

      <Card className="animate-fade-in-up delay-5">
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
                  <p className="text-[13px] leading-snug text-[var(--text-primary)]">
                    {activity.action} • {activity.resource_type}
                  </p>
                  <p className="text-[11px] text-[var(--text-tertiary)] mt-0.5">
                    {new Date(activity.created_at).toLocaleString()}
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
  delay
}: {
  title: string;
  value: string;
  color: "blue" | "purple" | "green" | "cyan";
  delay: string;
}) {
  const colorClasses = {
    blue: "from-[var(--accent-blue)]/20 to-transparent",
    purple: "from-[var(--accent-purple)]/20 to-transparent",
    green: "from-[var(--accent-green)]/20 to-transparent",
    cyan: "from-[var(--accent-cyan)]/20 to-transparent",
  };

  return (
    <Card className={`animate-fade-in-up ${delay} relative overflow-hidden`}>
      <div className={`absolute inset-0 bg-gradient-to-br ${colorClasses[color]} pointer-events-none`} />
      <CardContent className="pt-5 relative">
        <p className="text-[13px] font-medium text-[var(--text-secondary)]">{title}</p>
        <p className="mt-1 text-[28px] font-bold text-[var(--text-primary)] tracking-tight">{value}</p>
      </CardContent>
    </Card>
  );
}
