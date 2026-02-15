import { useState } from "react";
import type { LoaderFunctionArgs } from "react-router";
import { useLoaderData, Link, redirect, Outlet, useMatch, useNavigate } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { analyticsApi } from "~/services/api";
import type { DailyTrendPoint } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export async function loader({ request }: LoaderFunctionArgs) {
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  const url = new URL(request.url);
  const customStart = url.searchParams.get("start");
  const customEnd = url.searchParams.get("end");

  let startDate: string;
  let endDate: string;
  let days: number;
  let rangeLabel: string;

  if (customStart && customEnd) {
    startDate = new Date(customStart).toISOString();
    endDate = new Date(customEnd + "T23:59:59").toISOString();
    days = Math.max(1, Math.ceil((new Date(customEnd).getTime() - new Date(customStart).getTime()) / (24 * 60 * 60 * 1000)));
    rangeLabel = `${customStart} - ${customEnd}`;
  } else {
    days = Number(url.searchParams.get("days") || "7");
    endDate = new Date().toISOString();
    startDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();
    rangeLabel = `Last ${days} days`;
  }

  try {
    const [statsResponse, trendResponse] = await Promise.all([
      analyticsApi.getStats(startDate, endDate, accessToken),
      analyticsApi.getDailyTrend(days, accessToken, customStart ? startDate : undefined, customStart ? endDate : undefined),
    ]);
    return { stats: statsResponse.data, dailyTrend: trendResponse.data, days, rangeLabel, customStart, customEnd };
  } catch (error) {
    console.error("Analytics API error:", error);
    return {
      stats: null,
      dailyTrend: [] as DailyTrendPoint[],
      days,
      rangeLabel,
      customStart,
      customEnd,
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
      <CardContent className="pt-5">
        <div className="text-sm font-medium text-[var(--text-secondary)]">{title}</div>
        <div className="mt-1 flex items-baseline gap-2">
          <span className="text-[26px] font-bold tracking-tight">{value}</span>
          {trend && (
            <span
              className={`text-sm font-medium ${
                trend === "up"
                  ? "text-[var(--accent-green)]"
                  : trend === "down"
                  ? "text-[var(--accent-red)]"
                  : "text-[var(--text-secondary)]"
              }`}
            >
              {trend === "up" ? "↑" : trend === "down" ? "↓" : "→"}
            </span>
          )}
        </div>
        {subtitle && <div className="mt-1 text-sm text-[var(--text-secondary)]">{subtitle}</div>}
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
          <p className="text-[var(--text-secondary)] text-sm">No data available</p>
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
                    <span className="text-[var(--text-secondary)]">
                      {value} ({percentage.toFixed(1)}%)
                    </span>
                  </div>
                  <div className="h-2 bg-[var(--sidebar-item-hover)] rounded-full overflow-hidden">
                    <div
                      className="h-full bg-[var(--accent-blue)] rounded-full transition-all"
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

function DailyTrendChart({ data }: { data: DailyTrendPoint[] }) {
  if (data.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Daily Login Trend</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-[var(--text-secondary)] text-sm">No trend data available</p>
        </CardContent>
      </Card>
    );
  }

  const maxTotal = Math.max(...data.map((d) => d.total), 1);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Daily Login Trend</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-end gap-1.5" style={{ height: 160 }}>
          {data.map((point) => {
            const successHeight = (point.successful / maxTotal) * 100;
            const failedHeight = (point.failed / maxTotal) * 100;
            const dateLabel = point.date.slice(5); // "MM-DD"

            return (
              <div
                key={point.date}
                className="flex-1 flex flex-col items-center gap-1 min-w-0"
              >
                <div className="text-xs text-[var(--text-secondary)] tabular-nums">
                  {point.total}
                </div>
                <div
                  className="w-full flex flex-col justify-end rounded-t"
                  style={{ height: 120 }}
                >
                  {point.failed > 0 && (
                    <div
                      className="w-full bg-[var(--accent-red)] rounded-t opacity-80"
                      style={{ height: `${failedHeight}%`, minHeight: point.failed > 0 ? 2 : 0 }}
                    />
                  )}
                  {point.successful > 0 && (
                    <div
                      className="w-full bg-[var(--accent-blue)]"
                      style={{
                        height: `${successHeight}%`,
                        minHeight: point.successful > 0 ? 2 : 0,
                        borderRadius: point.failed > 0 ? 0 : "4px 4px 0 0",
                      }}
                    />
                  )}
                </div>
                <div className="text-[10px] text-[var(--text-secondary)] tabular-nums truncate w-full text-center">
                  {dateLabel}
                </div>
              </div>
            );
          })}
        </div>
        <div className="flex items-center gap-4 mt-3 text-xs text-[var(--text-secondary)]">
          <span className="flex items-center gap-1">
            <span className="inline-block w-2.5 h-2.5 rounded-sm bg-[var(--accent-blue)]" />
            Successful
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block w-2.5 h-2.5 rounded-sm bg-[var(--accent-red)] opacity-80" />
            Failed
          </span>
        </div>
      </CardContent>
    </Card>
  );
}

export default function AnalyticsPage() {
  const { stats, dailyTrend, days, rangeLabel, customStart, customEnd, error } = useLoaderData<typeof loader>();
  const isExactMatch = useMatch("/dashboard/analytics");
  const navigate = useNavigate();
  const [showCustomRange, setShowCustomRange] = useState(!!customStart);
  const [startInput, setStartInput] = useState(customStart || "");
  const [endInput, setEndInput] = useState(customEnd || "");

  const isCustomRange = !!customStart;

  const successRate =
    stats && stats.total_logins > 0
      ? ((stats.successful_logins / stats.total_logins) * 100).toFixed(1)
      : "0";

  // If we're on a child route (e.g., /dashboard/analytics/events), render the Outlet
  if (!isExactMatch) {
    return <Outlet />;
  }

  const handleCustomRangeApply = () => {
    if (startInput && endInput) {
      navigate(`?start=${startInput}&end=${endInput}`);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Analytics</h1>
          <p className="text-[var(--text-secondary)]">Login activity and statistics</p>
        </div>
        <div className="flex gap-2 items-center">
          {[7, 14, 30, 90].map((d) => (
            <Link
              key={d}
              to={`?days=${d}`}
              onClick={() => setShowCustomRange(false)}
              className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                days === d && !isCustomRange
                  ? "bg-blue-600 text-white"
                  : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
              }`}
            >
              {d}d
            </Link>
          ))}
          <button
            type="button"
            onClick={() => setShowCustomRange(!showCustomRange)}
            className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
              isCustomRange
                ? "bg-blue-600 text-white"
                : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
            }`}
          >
            Custom
          </button>
        </div>
      </div>

      {showCustomRange && (
        <div className="flex items-center gap-3">
          <input
            type="date"
            value={startInput}
            onChange={(e) => setStartInput(e.target.value)}
            className="px-3 py-1.5 text-sm rounded-md border border-[var(--border-primary)] bg-[var(--bg-primary)] text-[var(--text-primary)]"
          />
          <span className="text-sm text-[var(--text-secondary)]">to</span>
          <input
            type="date"
            value={endInput}
            onChange={(e) => setEndInput(e.target.value)}
            className="px-3 py-1.5 text-sm rounded-md border border-[var(--border-primary)] bg-[var(--bg-primary)] text-[var(--text-primary)]"
          />
          <button
            type="button"
            onClick={handleCustomRangeApply}
            disabled={!startInput || !endInput}
            className="px-4 py-1.5 text-sm rounded-md bg-blue-600 text-white disabled:opacity-50 disabled:cursor-not-allowed transition-colors hover:bg-blue-700"
          >
            Apply
          </button>
        </div>
      )}

      {error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{error}</div>
      )}

      {stats && (
        <>
          {/* Key Metrics */}
          <div className="grid gap-4 md:grid-cols-4">
            <StatCard
              title="Total Logins"
              value={stats.total_logins.toLocaleString()}
              subtitle={rangeLabel}
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

          {/* Daily Trend */}
          <DailyTrendChart data={dailyTrend} />

          {/* Breakdowns */}
          <div className="grid gap-6 md:grid-cols-2">
            <BreakdownCard title="By Event Type" data={stats.by_event_type} />
            <BreakdownCard title="By Device Type" data={stats.by_device_type} />
          </div>

          {/* Quick Links */}
          <Card>
            <CardContent className="pt-5">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-medium">View Login Events</h3>
                  <p className="text-sm text-[var(--text-secondary)]">
                    See detailed login activity and troubleshoot issues
                  </p>
                </div>
                <Link
                  to="/dashboard/analytics/events"
                  className="px-4 py-2 bg-[var(--sidebar-item-hover)] hover:bg-[var(--sidebar-item-hover)] rounded-md text-sm font-medium transition-colors"
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
