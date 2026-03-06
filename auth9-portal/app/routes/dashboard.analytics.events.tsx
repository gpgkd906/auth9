import { useState } from "react";
import type { LoaderFunctionArgs, MetaFunction } from "react-router";
import { Link, redirect, useLoaderData, useNavigate } from "react-router";
import { CheckCircledIcon, CrossCircledIcon, LockClosedIcon, PersonIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Input } from "~/components/ui/input";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { analyticsApi, type LoginEvent } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { resolveLocale } from "~/services/locale.server";
import { translate } from "~/i18n/translate";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "analyticsEvents.metaTitle");

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const accessToken = await getAccessToken(request);
  if (!accessToken) {
    throw redirect("/login");
  }

  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = 20;
  const email = url.searchParams.get("email") || undefined;

  try {
    const response = await analyticsApi.listEvents(page, perPage, email, accessToken);
    return { events: response.data, pagination: response.pagination, email };
  } catch {
    return {
      events: [],
      pagination: { page: 1, per_page: perPage, total: 0, total_pages: 0 },
      error: translate(locale, "analyticsEvents.loadFailed"),
      email,
    };
  }
}

function getEventIcon(eventType: string) {
  switch (eventType) {
    case "success":
    case "social":
      return <CheckCircledIcon className="h-4 w-4 text-[var(--accent-green)]" />;
    case "failed_password":
    case "failed_mfa":
      return <CrossCircledIcon className="h-4 w-4 text-[var(--accent-red)]" />;
    case "locked":
      return <LockClosedIcon className="h-4 w-4 text-[var(--accent-orange)]" />;
    default:
      return <PersonIcon className="h-4 w-4 text-[var(--text-secondary)]" />;
  }
}

function getEventLabel(t: (key: string) => string, eventType: string) {
  switch (eventType) {
    case "success":
      return t("analyticsEvents.labels.success");
    case "social":
      return t("analyticsEvents.labels.social");
    case "failed_password":
      return t("analyticsEvents.labels.failedPassword");
    case "failed_mfa":
      return t("analyticsEvents.labels.failedMfa");
    case "locked":
      return t("analyticsEvents.labels.locked");
    default:
      return eventType;
  }
}

function getEventBadgeColor(eventType: string) {
  switch (eventType) {
    case "success":
    case "social":
      return "bg-green-100 text-[var(--accent-green)]";
    case "failed_password":
    case "failed_mfa":
      return "bg-red-100 text-red-700";
    case "locked":
      return "bg-orange-100 text-orange-700";
    default:
      return "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)]";
  }
}

export default function LoginEventsPage() {
  const { events, pagination, error, email } = useLoaderData<typeof loader>();
  const navigate = useNavigate();
  const [emailFilter, setEmailFilter] = useState(email || "");
  const { t } = useI18n();
  const formatters = useFormatters();

  const handleFilterSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const params = new URLSearchParams();
    if (emailFilter.trim()) {
      params.set("email", emailFilter);
    }
    params.set("page", "1");
    navigate(`/dashboard/analytics/events?${params.toString()}`);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t("analyticsEvents.title")}</h1>
          <p className="text-[var(--text-secondary)]">{t("analyticsEvents.description")}</p>
        </div>
        <Link to="/dashboard/analytics">
          <Button variant="outline">{t("analyticsEvents.back")}</Button>
        </Link>
      </div>

      {error && <div className="rounded-md bg-red-50 p-3 text-sm text-[var(--accent-red)]">{error}</div>}

      <form onSubmit={handleFilterSubmit} className="flex gap-2">
        <Input type="text" placeholder={t("analyticsEvents.filterPlaceholder")} value={emailFilter} onChange={(event) => setEmailFilter(event.target.value)} className="flex-1" />
        <Button type="submit" variant="outline">{t("analyticsEvents.filter")}</Button>
        {emailFilter && (
          <Button
            type="button"
            variant="ghost"
            onClick={() => {
              setEmailFilter("");
              navigate("/dashboard/analytics/events");
            }}
          >
            {t("analyticsEvents.clear")}
          </Button>
        )}
      </form>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">
            {t("analyticsEvents.recentEvents")}
            <span className="ml-2 text-sm font-normal text-[var(--text-secondary)]">{t("analyticsEvents.total", { count: pagination.total })}</span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {events.length === 0 ? (
            <p className="py-8 text-center text-[var(--text-secondary)]">{t("analyticsEvents.noEvents")}</p>
          ) : (
            <div className="overflow-x-auto">
              <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
                <thead className="bg-[var(--sidebar-item-hover)]">
                  <tr>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">{t("analyticsEvents.headers.time")}</th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">{t("analyticsEvents.headers.event")}</th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">{t("analyticsEvents.headers.user")}</th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">{t("analyticsEvents.headers.ipAddress")}</th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">{t("analyticsEvents.headers.device")}</th>
                    <th className="px-4 py-3 text-left font-medium text-[var(--text-secondary)]">{t("analyticsEvents.headers.details")}</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                  {events.map((event: LoginEvent) => (
                    <tr key={event.id} className="hover:bg-[var(--sidebar-item-hover)]">
                      <td className="whitespace-nowrap px-4 py-3 text-[var(--text-secondary)]" suppressHydrationWarning>
                        {formatters.dateTime(event.created_at)}
                      </td>
                      <td className="whitespace-nowrap px-4 py-3">
                        <span className={`inline-flex items-center gap-1.5 rounded-full px-2 py-1 text-xs font-medium ${getEventBadgeColor(event.event_type)}`}>
                          {getEventIcon(event.event_type)}
                          {getEventLabel(t, event.event_type)}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <div className="max-w-[200px] truncate">{event.email || event.user_id || t("analyticsEvents.unknown")}</div>
                      </td>
                      <td className="whitespace-nowrap px-4 py-3 text-[var(--text-secondary)]">{event.ip_address || "-"}</td>
                      <td className="whitespace-nowrap px-4 py-3">
                        <span className="capitalize text-[var(--text-secondary)]">{event.device_type || "-"}</span>
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex flex-col gap-0.5">
                          {event.failure_reason && <span className="text-xs font-medium text-[var(--accent-red)]">{event.failure_reason}</span>}
                          {event.location && <span className="text-xs text-[var(--text-secondary)]">{event.location}</span>}
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {pagination.total_pages > 1 && (
            <div className="mt-4 flex items-center justify-between border-t pt-4">
              <div className="text-sm text-[var(--text-secondary)]">{t("analyticsEvents.page", { page: pagination.page, totalPages: pagination.total_pages })}</div>
              <div className="flex gap-2">
                {pagination.page > 1 && (
                  <Link to={`?page=${pagination.page - 1}${email ? `&email=${encodeURIComponent(email)}` : ""}`}>
                    <Button variant="outline" size="sm">{t("analyticsEvents.previous")}</Button>
                  </Link>
                )}
                {pagination.page < pagination.total_pages && (
                  <Link to={`?page=${pagination.page + 1}${email ? `&email=${encodeURIComponent(email)}` : ""}`}>
                    <Button variant="outline" size="sm">{t("analyticsEvents.next")}</Button>
                  </Link>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
