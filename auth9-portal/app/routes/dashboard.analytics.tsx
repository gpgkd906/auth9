import { useState } from "react";
import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, Outlet, redirect, useLoaderData, useMatch, useNavigate } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { analyticsApi } from "~/services/api";
import type { DailyTrendPoint } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "analytics.metaTitle");

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
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
    endDate = new Date(`${customEnd}T23:59:59`).toISOString();
    days = Math.max(1, Math.ceil((new Date(customEnd).getTime() - new Date(customStart).getTime()) / (24 * 60 * 60 * 1000)));
    rangeLabel = `${customStart} - ${customEnd}`;
  } else {
    days = Number(url.searchParams.get("days") || "7");
    endDate = new Date().toISOString();
    startDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();
    rangeLabel = translate(locale, "analytics.lastDays", { days });
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
      error: translate(locale, "analytics.loadFailed"),
    };
  }
}

function StatCard({ title, value, subtitle, trend }: { title: string; value: number | string; subtitle?: string; trend?: "up" | "down" | "neutral" }) {
  return (
    <Card>
      <CardContent className="pt-5">
        <div className="text-sm font-medium text-[var(--text-secondary)]">{title}</div>
        <div className="mt-1 flex items-baseline gap-2">
          <span className="text-[26px] font-bold tracking-tight">{value}</span>
          {trend && (
            <span
              className={`text-sm font-medium ${
                trend === "up" ? "text-[var(--accent-green)]" : trend === "down" ? "text-[var(--accent-red)]" : "text-[var(--text-secondary)]"
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

function BreakdownCard({ title, data, noDataLabel }: { title: string; data: Record<string, number>; noDataLabel: string }) {
  const entries = Object.entries(data).sort((a, b) => b[1] - a[1]);
  const total = entries.reduce((sum, [, value]) => sum + value, 0);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        {entries.length === 0 ? (
          <p className="text-sm text-[var(--text-secondary)]">{noDataLabel}</p>
        ) : (
          <div className="space-y-3">
            {entries.map(([key, value]) => {
              const percentage = total > 0 ? (value / total) * 100 : 0;
              return (
                <div key={key}>
                  <div className="mb-1 flex justify-between text-sm">
                    <span className="font-medium capitalize">{key.replace(/_/g, " ")}</span>
                    <span className="text-[var(--text-secondary)]">{value} ({percentage.toFixed(1)}%)</span>
                  </div>
                  <div className="h-2 overflow-hidden rounded-full bg-[var(--sidebar-item-hover)]">
                    <div className="h-full rounded-full bg-[var(--accent-blue)] transition-all" style={{ width: `${percentage}%` }} />
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
  const { t } = useI18n();

  if (data.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">{t("analytics.dailyTrend")}</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-[var(--text-secondary)]">{t("analytics.noTrend")}</p>
        </CardContent>
      </Card>
    );
  }

  const maxTotal = Math.max(...data.map((point) => point.total), 1);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">{t("analytics.dailyTrend")}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-end gap-1.5" style={{ height: 160 }}>
          {data.map((point) => {
            const successHeight = (point.successful / maxTotal) * 100;
            const failedHeight = (point.failed / maxTotal) * 100;
            const dateLabel = point.date.slice(5);

            return (
              <div key={point.date} className="flex min-w-0 flex-1 flex-col items-center gap-1">
                <div className="text-xs tabular-nums text-[var(--text-secondary)]">{point.total}</div>
                <div className="flex h-[120px] w-full flex-col justify-end rounded-t">
                  {point.failed > 0 && (
                    <div className="w-full rounded-t bg-[var(--accent-red)] opacity-80" style={{ height: `${failedHeight}%`, minHeight: point.failed > 0 ? 2 : 0 }} />
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
                <div className="w-full truncate text-center text-[10px] tabular-nums text-[var(--text-secondary)]">{dateLabel}</div>
              </div>
            );
          })}
        </div>
        <div className="mt-3 flex items-center gap-4 text-xs text-[var(--text-secondary)]">
          <span className="flex items-center gap-1">
            <span className="inline-block h-2.5 w-2.5 rounded-sm bg-[var(--accent-blue)]" />
            {t("analytics.successfulLegend")}
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block h-2.5 w-2.5 rounded-sm bg-[var(--accent-red)] opacity-80" />
            {t("analytics.failedLegend")}
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
  const [showCustomRange, setShowCustomRange] = useState(Boolean(customStart));
  const [startInput, setStartInput] = useState(customStart || "");
  const [endInput, setEndInput] = useState(customEnd || "");
  const { t } = useI18n();
  const formatters = useFormatters();

  const isCustomRange = Boolean(customStart);
  const successRate = stats && stats.total_logins > 0 ? ((stats.successful_logins / stats.total_logins) * 100).toFixed(1) : "0";

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
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t("analytics.title")}</h1>
          <p className="text-[var(--text-secondary)]">{t("analytics.description")}</p>
        </div>
        <div className="flex items-center gap-2">
          {[7, 14, 30, 90].map((value) => (
            <Link
              key={value}
              to={`?days=${value}`}
              onClick={() => setShowCustomRange(false)}
              className={`rounded-md px-3 py-1.5 text-sm transition-colors ${
                days === value && !isCustomRange
                  ? "bg-blue-600 text-white"
                  : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
              }`}
            >
              {value}d
            </Link>
          ))}
          <button
            type="button"
            onClick={() => setShowCustomRange(!showCustomRange)}
            className={`rounded-md px-3 py-1.5 text-sm transition-colors ${
              isCustomRange ? "bg-blue-600 text-white" : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
            }`}
          >
            {t("analytics.custom")}
          </button>
        </div>
      </div>

      {showCustomRange && (
        <div className="flex items-center gap-3">
          <input type="date" value={startInput} onChange={(event) => setStartInput(event.target.value)} className="rounded-md border border-[var(--border-primary)] bg-[var(--bg-primary)] px-3 py-1.5 text-sm text-[var(--text-primary)]" />
          <span className="text-sm text-[var(--text-secondary)]">{t("analytics.to")}</span>
          <input type="date" value={endInput} onChange={(event) => setEndInput(event.target.value)} className="rounded-md border border-[var(--border-primary)] bg-[var(--bg-primary)] px-3 py-1.5 text-sm text-[var(--text-primary)]" />
          <button type="button" onClick={handleCustomRangeApply} disabled={!startInput || !endInput} className="rounded-md bg-blue-600 px-4 py-1.5 text-sm text-white transition-colors hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50">
            {t("analytics.apply")}
          </button>
        </div>
      )}

      {error && <div className="rounded-md bg-red-50 p-3 text-sm text-[var(--accent-red)]">{error}</div>}

      {stats && (
        <>
          <div className="grid gap-4 md:grid-cols-4">
            <StatCard title={t("analytics.totalLogins")} value={formatters.number(stats.total_logins)} subtitle={rangeLabel} />
            <StatCard title={t("analytics.successful")} value={formatters.number(stats.successful_logins)} subtitle={t("analytics.successRate", { rate: successRate })} trend="up" />
            <StatCard title={t("analytics.failed")} value={formatters.number(stats.failed_logins)} trend={stats.failed_logins > 0 ? "down" : "neutral"} />
            <StatCard title={t("analytics.uniqueUsers")} value={formatters.number(stats.unique_users)} />
          </div>

          <DailyTrendChart data={dailyTrend} />

          <div className="grid gap-6 md:grid-cols-2">
            <BreakdownCard title={t("analytics.byEventType")} data={stats.by_event_type} noDataLabel={t("analytics.noData")} />
            <BreakdownCard title={t("analytics.byDeviceType")} data={stats.by_device_type} noDataLabel={t("analytics.noData")} />
          </div>

          <Card>
            <CardContent className="pt-5">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-medium">{t("analytics.viewLoginEvents")}</h3>
                  <p className="text-sm text-[var(--text-secondary)]">{t("analytics.viewLoginEventsDescription")}</p>
                </div>
                <Link to="/dashboard/analytics/events" className="rounded-md bg-[var(--sidebar-item-hover)] px-4 py-2 text-sm font-medium transition-colors hover:bg-[var(--sidebar-item-hover)]">
                  {t("analytics.viewEvents")}
                </Link>
              </div>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
