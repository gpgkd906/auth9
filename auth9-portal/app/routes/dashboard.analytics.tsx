import type { LoaderFunctionArgs } from "react-router";
import { useLoaderData, Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { analyticsApi } from "~/services/api";

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const days = Number(url.searchParams.get("days") || "7");

  const endDate = new Date().toISOString();
  const startDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();

  try {
    const response = await analyticsApi.getStats(startDate, endDate);
    return { stats: response.data, days };
  } catch {
    return {
      stats: null,
      days,
      error: "Failed to load analytics",
    };
  }
}

function StatCard({
  title,
  value,
  subtitle,
  trend,
}: {
  title: string;
  value: number | string;
  subtitle?: string;
  trend?: "up" | "down" | "neutral";
}) {
  return (
    <Card>
      <CardContent className="pt-6">
        <div className="text-sm font-medium text-gray-500">{title}</div>
        <div className="mt-2 flex items-baseline gap-2">
          <span className="text-3xl font-bold">{value}</span>
          {trend && (
            <span
              className={`text-sm font-medium ${
                trend === "up"
                  ? "text-green-600"
                  : trend === "down"
                  ? "text-red-600"
                  : "text-gray-500"
              }`}
            >
              {trend === "up" ? "↑" : trend === "down" ? "↓" : "→"}
            </span>
          )}
        </div>
        {subtitle && <div className="mt-1 text-sm text-gray-500">{subtitle}</div>}
      </CardContent>
    </Card>
  );
}

function BreakdownCard({
  title,
  data,
}: {
  title: string;
  data: Record<string, number>;
}) {
  const entries = Object.entries(data).sort((a, b) => b[1] - a[1]);
  const total = entries.reduce((sum, [, value]) => sum + value, 0);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        {entries.length === 0 ? (
          <p className="text-gray-500 text-sm">No data available</p>
        ) : (
          <div className="space-y-3">
            {entries.map(([key, value]) => {
              const percentage = total > 0 ? (value / total) * 100 : 0;
              return (
                <div key={key}>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="font-medium capitalize">
                      {key.replace(/_/g, " ")}
                    </span>
                    <span className="text-gray-500">
                      {value} ({percentage.toFixed(1)}%)
                    </span>
                  </div>
                  <div className="h-2 bg-gray-100 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-blue-500 rounded-full transition-all"
                      style={{ width: `${percentage}%` }}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default function AnalyticsPage() {
  const { stats, days, error } = useLoaderData<typeof loader>();

  const successRate =
    stats && stats.total_logins > 0
      ? ((stats.successful_logins / stats.total_logins) * 100).toFixed(1)
      : "0";

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Analytics</h1>
          <p className="text-gray-500">Login activity and statistics</p>
        </div>
        <div className="flex gap-2">
          {[7, 14, 30, 90].map((d) => (
            <Link
              key={d}
              to={`?days=${d}`}
              className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                days === d
                  ? "bg-blue-600 text-white"
                  : "bg-gray-100 text-gray-700 hover:bg-gray-200"
              }`}
            >
              {d}d
            </Link>
          ))}
        </div>
      </div>

      {error && (
        <div className="text-sm text-red-600 bg-red-50 p-3 rounded-md">{error}</div>
      )}

      {stats && (
        <>
          {/* Key Metrics */}
          <div className="grid gap-4 md:grid-cols-4">
            <StatCard
              title="Total Logins"
              value={stats.total_logins.toLocaleString()}
              subtitle={`Last ${days} days`}
            />
            <StatCard
              title="Successful"
              value={stats.successful_logins.toLocaleString()}
              subtitle={`${successRate}% success rate`}
              trend="up"
            />
            <StatCard
              title="Failed"
              value={stats.failed_logins.toLocaleString()}
              trend={stats.failed_logins > 0 ? "down" : "neutral"}
            />
            <StatCard
              title="Unique Users"
              value={stats.unique_users.toLocaleString()}
            />
          </div>

          {/* Breakdowns */}
          <div className="grid gap-6 md:grid-cols-2">
            <BreakdownCard title="By Event Type" data={stats.by_event_type} />
            <BreakdownCard title="By Device Type" data={stats.by_device_type} />
          </div>

          {/* Quick Links */}
          <Card>
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-medium">View Login Events</h3>
                  <p className="text-sm text-gray-500">
                    See detailed login activity and troubleshoot issues
                  </p>
                </div>
                <Link
                  to="/dashboard/analytics/events"
                  className="px-4 py-2 bg-gray-100 hover:bg-gray-200 rounded-md text-sm font-medium transition-colors"
                >
                  View events →
                </Link>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
