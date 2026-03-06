import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "react-router";
import { Form, Link, useActionData, useLoaderData, useNavigation, useSearchParams } from "react-router";
import { CheckCircledIcon, ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Button } from "~/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { useFormatters } from "~/i18n/format";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";
import { translate } from "~/i18n/translate";
import { resolveLocale } from "~/services/locale.server";
import { securityAlertApi, type AlertSeverity, type SecurityAlert, type SecurityAlertType } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction = ({ matches }) => buildMeta(resolveMetaLocale(matches), "securityAlerts.metaTitle");

const SEVERITY_OPTIONS: AlertSeverity[] = ["critical", "high", "medium", "low"];
const ALERT_TYPE_OPTIONS: SecurityAlertType[] = ["brute_force", "new_device", "impossible_travel", "suspicious_ip"];

export async function loader({ request }: LoaderFunctionArgs) {
  const locale = await resolveLocale(request);
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const unresolvedOnly = url.searchParams.get("unresolved") === "true";
  const severity = url.searchParams.get("severity") as AlertSeverity | null;
  const alertType = url.searchParams.get("alert_type") as SecurityAlertType | null;
  const accessToken = await getAccessToken(request);

  try {
    const response = await securityAlertApi.list(
      page,
      50,
      unresolvedOnly,
      accessToken || undefined,
      severity || undefined,
      alertType || undefined,
    );
    return { alerts: response.data, pagination: response.pagination, unresolvedOnly, severity, alertType };
  } catch {
    return {
      alerts: [],
      pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
      unresolvedOnly,
      severity,
      alertType,
      error: translate(locale, "securityAlerts.loadFailed"),
    };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const locale = await resolveLocale(request);
  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request);

  try {
    if (intent === "resolve") {
      const alertId = formData.get("alertId") as string;
      await securityAlertApi.resolve(alertId, accessToken || undefined);
      return { success: true, message: translate(locale, "securityAlerts.resolved") };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : translate(locale, "securityAlerts.operationFailed");
    return { error: message };
  }

  return { error: translate(locale, "securityAlerts.invalidAction") };
}

function getSeverityColor(severity: AlertSeverity) {
  switch (severity) {
    case "critical":
      return "bg-red-100 text-red-800 border-red-200";
    case "high":
      return "bg-orange-100 text-orange-800 border-orange-200";
    case "medium":
      return "bg-yellow-100 text-yellow-800 border-yellow-200";
    case "low":
      return "bg-blue-100 text-blue-800 border-blue-200";
    default:
      return "bg-[var(--sidebar-item-hover)] text-[var(--text-primary)] border-[var(--glass-border-subtle)]";
  }
}

function getSeverityIcon(severity: AlertSeverity) {
  switch (severity) {
    case "critical":
    case "high":
      return <ExclamationTriangleIcon className="h-5 w-5 text-[var(--accent-red)]" />;
    case "medium":
      return <ExclamationTriangleIcon className="h-5 w-5 text-yellow-600" />;
    default:
      return <ExclamationTriangleIcon className="h-5 w-5 text-[var(--accent-blue)]" />;
  }
}

function buildFilterUrl(params: Record<string, string | null>) {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value) search.set(key, value);
  }
  const qs = search.toString();
  return qs ? `?${qs}` : "?";
}

export default function SecurityAlertsPage() {
  const { alerts, pagination, unresolvedOnly, error } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [searchParams] = useSearchParams();
  const { t } = useI18n();
  const formatters = useFormatters();

  const isSubmitting = navigation.state === "submitting";
  const unresolvedCount = alerts.filter((alert: SecurityAlert) => !alert.resolved_at).length;
  const currentSeverity = searchParams.get("severity");
  const currentAlertType = searchParams.get("alert_type");

  function filterUrl(overrides: Record<string, string | null>) {
    const base: Record<string, string | null> = {
      unresolved: unresolvedOnly ? "true" : null,
      severity: currentSeverity,
      alert_type: currentAlertType,
    };
    return buildFilterUrl({ ...base, ...overrides, page: null });
  }

  function severityLabel(severity: string) {
    if (SEVERITY_OPTIONS.includes(severity as AlertSeverity)) {
      return t(`securityAlerts.severities.${severity as AlertSeverity}` as const);
    }
    return severity;
  }

  function alertTypeLabel(type: string) {
    if (ALERT_TYPE_OPTIONS.includes(type as SecurityAlertType)) {
      return t(`securityAlerts.types.${type as SecurityAlertType}` as const);
    }
    return type.replace(/_/g, " ");
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">{t("securityAlerts.title")}</h1>
          <p className="text-[var(--text-secondary)]">{t("securityAlerts.description")}</p>
        </div>
        <div className="flex gap-2">
          <Link
            to={filterUrl({ unresolved: null })}
            className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
              !unresolvedOnly
                ? "bg-blue-600 text-white"
                : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
            }`}
          >
            {t("securityAlerts.all")}
          </Link>
          <Link
            to={filterUrl({ unresolved: "true" })}
            className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
              unresolvedOnly
                ? "bg-blue-600 text-white"
                : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
            }`}
          >
            {t("securityAlerts.unresolved", { count: unresolvedCount })}
          </Link>
        </div>
      </div>

      <div className="flex gap-4 flex-wrap">
        <div className="flex items-center gap-2">
          <span className="text-sm text-[var(--text-secondary)]">{t("securityAlerts.severity")}</span>
          <div className="flex gap-1">
            <Link
              to={filterUrl({ severity: null })}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                !currentSeverity
                  ? "bg-blue-600 text-white"
                  : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
              }`}
            >
              {t("securityAlerts.all")}
            </Link>
            {SEVERITY_OPTIONS.map((value) => (
              <Link
                key={value}
                to={filterUrl({ severity: currentSeverity === value ? null : value })}
                className={`px-2 py-1 text-xs rounded transition-colors ${
                  currentSeverity === value
                    ? "bg-blue-600 text-white"
                    : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
                }`}
              >
                {severityLabel(value)}
              </Link>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-sm text-[var(--text-secondary)]">{t("securityAlerts.type")}</span>
          <div className="flex gap-1">
            <Link
              to={filterUrl({ alert_type: null })}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                !currentAlertType
                  ? "bg-blue-600 text-white"
                  : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
              }`}
            >
              {t("securityAlerts.all")}
            </Link>
            {ALERT_TYPE_OPTIONS.map((value) => (
              <Link
                key={value}
                to={filterUrl({ alert_type: currentAlertType === value ? null : value })}
                className={`px-2 py-1 text-xs rounded transition-colors ${
                  currentAlertType === value
                    ? "bg-blue-600 text-white"
                    : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
                }`}
              >
                {alertTypeLabel(value)}
              </Link>
            ))}
          </div>
        </div>
      </div>

      {error && <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{error}</div>}
      {actionData?.error && <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{actionData.error}</div>}
      {actionData?.success && <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">{actionData.message}</div>}

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">
            {t("securityAlerts.alertsTitle")}
            <span className="ml-2 text-sm font-normal text-[var(--text-secondary)]">
              {t("securityAlerts.total", { count: pagination.total })}
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {alerts.length === 0 ? (
            <div className="text-center py-12">
              <CheckCircledIcon className="h-12 w-12 text-[var(--accent-green)] mx-auto mb-4" />
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">{t("securityAlerts.allClear")}</h3>
              <p className="text-[var(--text-secondary)]">
                {unresolvedOnly ? t("securityAlerts.noUnresolved") : t("securityAlerts.noAlerts")}
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              {alerts.map((alert: SecurityAlert) => (
                <div key={alert.id} className={`border rounded-lg p-4 ${alert.resolved_at ? "opacity-60" : ""}`}>
                  <div className="flex items-start gap-4">
                    <div className="flex-shrink-0 mt-0.5">{getSeverityIcon(alert.severity)}</div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className={`px-2 py-0.5 text-xs font-medium rounded border ${getSeverityColor(alert.severity)}`}>
                          {severityLabel(alert.severity).toUpperCase()}
                        </span>
                        <span className="font-medium">{alertTypeLabel(alert.alert_type)}</span>
                        {alert.resolved_at && (
                          <span className="inline-flex items-center gap-1 text-xs text-[var(--accent-green)]">
                            <CheckCircledIcon className="h-3 w-3" />
                            {t("securityAlerts.resolvedBadge")}
                          </span>
                        )}
                      </div>
                      <div className="text-sm text-[var(--text-secondary)] mb-2" suppressHydrationWarning>
                        {formatters.dateTime(alert.created_at)}
                        {alert.user_id && ` • ${t("securityAlerts.user")}: ${alert.user_id.slice(0, 8)}...`}
                      </div>
                      {alert.details && (
                        <div className="text-sm bg-[var(--sidebar-item-hover)] p-2 rounded mt-2">
                          <pre className="whitespace-pre-wrap text-xs text-[var(--text-secondary)]">
                            {JSON.stringify(alert.details, null, 2)}
                          </pre>
                        </div>
                      )}
                    </div>
                    {!alert.resolved_at && (
                      <Form method="post">
                        <input type="hidden" name="intent" value="resolve" />
                        <input type="hidden" name="alertId" value={alert.id} />
                        <Button type="submit" variant="outline" size="sm" disabled={isSubmitting}>
                          <CheckCircledIcon className="h-4 w-4 mr-1" />
                          {t("securityAlerts.resolve")}
                        </Button>
                      </Form>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {pagination.total_pages > 1 && (
            <div className="flex items-center justify-between mt-4 pt-4 border-t">
              <div className="text-sm text-[var(--text-secondary)]">
                {t("securityAlerts.pageOf", { page: pagination.page, totalPages: pagination.total_pages })}
              </div>
              <div className="flex gap-2">
                {pagination.page > 1 && (
                  <Link to={filterUrl({ page: String(pagination.page - 1) })}>
                    <Button variant="outline" size="sm">{t("securityAlerts.previous")}</Button>
                  </Link>
                )}
                {pagination.page < pagination.total_pages && (
                  <Link to={filterUrl({ page: String(pagination.page + 1) })}>
                    <Button variant="outline" size="sm">{t("securityAlerts.next")}</Button>
                  </Link>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="text-lg">{t("securityAlerts.recommendationsTitle")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="text-sm text-[var(--text-secondary)] space-y-2">
            <li>• {t("securityAlerts.recommendations.reviewCritical")}</li>
            <li>• {t("securityAlerts.recommendations.enableMfa")}</li>
            <li>• {t("securityAlerts.recommendations.configureRateLimit")}</li>
            <li>• {t("securityAlerts.recommendations.setupWebhooks")}</li>
            <li>• {t("securityAlerts.recommendations.reviewSessions")}</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
