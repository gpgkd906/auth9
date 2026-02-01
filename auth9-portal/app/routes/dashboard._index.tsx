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
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
        <p className="mt-2 text-gray-600">
          Welcome to Auth9. Here&apos;s an overview of your identity service.
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatsCard title="Total Tenants" value={data.totals.tenants.toString()} />
        <StatsCard title="Active Users" value={data.totals.users.toString()} />
        <StatsCard title="Services" value={data.totals.services.toString()} />
        <StatsCard title="Audit Events" value={data.audits.length.toString()} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {data.audits.map((activity) => (
              <div
                key={activity.id}
                className="flex items-center gap-4 py-3 border-b border-gray-100 last:border-0"
              >
                <div className="w-2 h-2 rounded-full bg-apple-blue" />
                <div className="flex-1">
                  <p className="text-sm text-gray-900">
                    {activity.action} • {activity.resource_type}
                  </p>
                  <p className="text-xs text-gray-500">
                    {new Date(activity.created_at).toLocaleString()}
                  </p>
                </div>
              </div>
            ))}
            {data.audits.length === 0 && (
              <div className="py-6 text-center text-sm text-gray-500">
                No recent activity
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function StatsCard({ title, value }: { title: string; value: string }) {
  return (
    <Card>
      <CardContent className="pt-6">
        <p className="text-sm font-medium text-gray-500">{title}</p>
        <p className="mt-2 text-3xl font-bold text-gray-900">{value}</p>
      </CardContent>
    </Card>
  );
}
