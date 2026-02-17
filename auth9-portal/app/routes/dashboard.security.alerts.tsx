import type { ActionFunctionArgs, LoaderFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation, Link, useSearchParams } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { securityAlertApi, type SecurityAlert, type AlertSeverity, type SecurityAlertType } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import {
  ExclamationTriangleIcon,
  CheckCircledIcon,
} from "@radix-ui/react-icons";

const SEVERITY_OPTIONS: { value: AlertSeverity; label: string }[] = [
  { value: "critical", label: "Critical" },
  { value: "high", label: "High" },
  { value: "medium", label: "Medium" },
  { value: "low", label: "Low" },
];

const ALERT_TYPE_OPTIONS: { value: SecurityAlertType; label: string }[] = [
  { value: "brute_force", label: "Brute Force" },
  { value: "new_device", label: "New Device" },
  { value: "impossible_travel", label: "Impossible Travel" },
  { value: "suspicious_ip", label: "Suspicious IP" },
];

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const unresolvedOnly = url.searchParams.get("unresolved") === "true";
  const severity = url.searchParams.get("severity") as AlertSeverity | null;
  const alertType = url.searchParams.get("alert_type") as SecurityAlertType | null;
  const accessToken = await getAccessToken(request);

  try {
    const response = await securityAlertApi.list(
      page, 50, unresolvedOnly, accessToken || undefined,
      severity || undefined, alertType || undefined,
    );
    return { alerts: response.data, pagination: response.pagination, unresolvedOnly, severity, alertType };
  } catch {
    return {
      alerts: [],
      pagination: { page: 1, per_page: 50, total: 0, total_pages: 0 },
      unresolvedOnly,
      severity,
      alertType,
      error: "Failed to load security alerts",
    };
  }
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");
  const accessToken = await getAccessToken(request);

  try {
    if (intent === "resolve") {
      const alertId = formData.get("alertId") as string;
      await securityAlertApi.resolve(alertId, accessToken || undefined);
      return { success: true, message: "Alert resolved" };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Operation failed";
    return { error: message };
  }

  return { error: "Invalid action" };
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

function getAlertTypeLabel(type: string) {
  switch (type) {
    case "brute_force":
      return "Brute Force Attack";
    case "new_device":
      return "New Device Login";
    case "impossible_travel":
      return "Impossible Travel";
    case "suspicious_ip":
      return "Suspicious IP";
    default:
      return type.replace(/_/g, " ");
  }
}

function formatDate(dateString: string) {
  const date = new Date(dateString);
  return date.toLocaleString();
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
  const { alerts, pagination, unresolvedOnly, severity, alertType, error } = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [searchParams] = useSearchParams();

  const isSubmitting = navigation.state === "submitting";

  const unresolvedCount = alerts.filter((a: SecurityAlert) => !a.resolved_at).length;

  const currentSeverity = searchParams.get("severity");
  const currentAlertType = searchParams.get("alert_type");

  // Build base params preserving existing filters
  function filterUrl(overrides: Record<string, string | null>) {
    const base: Record<string, string | null> = {
      unresolved: unresolvedOnly ? "true" : null,
      severity: currentSeverity,
      alert_type: currentAlertType,
    };
    return buildFilterUrl({ ...base, ...overrides, page: null });
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Security Alerts</h1>
          <p className="text-[var(--text-secondary)]">
            Monitor and respond to security threats
          </p>
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
            All
          </Link>
          <Link
            to={filterUrl({ unresolved: "true" })}
            className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
              unresolvedOnly
                ? "bg-blue-600 text-white"
                : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
            }`}
          >
            Unresolved ({unresolvedCount})
          </Link>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-4 flex-wrap">
        {/* Severity Filter */}
        <div className="flex items-center gap-2">
          <span className="text-sm text-[var(--text-secondary)]">Severity:</span>
          <div className="flex gap-1">
            <Link
              to={filterUrl({ severity: null })}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                !currentSeverity
                  ? "bg-blue-600 text-white"
                  : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
              }`}
            >
              All
            </Link>
            {SEVERITY_OPTIONS.map((opt) => (
              <Link
                key={opt.value}
                to={filterUrl({ severity: currentSeverity === opt.value ? null : opt.value })}
                className={`px-2 py-1 text-xs rounded transition-colors ${
                  currentSeverity === opt.value
                    ? "bg-blue-600 text-white"
                    : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
                }`}
              >
                {opt.label}
              </Link>
            ))}
          </div>
        </div>

        {/* Alert Type Filter */}
        <div className="flex items-center gap-2">
          <span className="text-sm text-[var(--text-secondary)]">Type:</span>
          <div className="flex gap-1">
            <Link
              to={filterUrl({ alert_type: null })}
              className={`px-2 py-1 text-xs rounded transition-colors ${
                !currentAlertType
                  ? "bg-blue-600 text-white"
                  : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
              }`}
            >
              All
            </Link>
            {ALERT_TYPE_OPTIONS.map((opt) => (
              <Link
                key={opt.value}
                to={filterUrl({ alert_type: currentAlertType === opt.value ? null : opt.value })}
                className={`px-2 py-1 text-xs rounded transition-colors ${
                  currentAlertType === opt.value
                    ? "bg-blue-600 text-white"
                    : "bg-[var(--sidebar-item-hover)] text-[var(--text-secondary)] hover:bg-[var(--sidebar-item-hover)]"
                }`}
              >
                {opt.label}
              </Link>
            ))}
          </div>
        </div>
      </div>

      {/* Messages */}
      {error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">{error}</div>
      )}

      {actionData?.error && (
        <div className="text-sm text-[var(--accent-red)] bg-red-50 p-3 rounded-md">
          {actionData.error}
        </div>
      )}

      {actionData?.success && (
        <div className="text-sm text-[var(--accent-green)] bg-[var(--accent-green)]/10 p-3 rounded-md">
          {actionData.message}
        </div>
      )}

      {/* Alerts List */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">
            Alerts
            <span className="ml-2 text-sm font-normal text-[var(--text-secondary)]">
              {pagination.total.toLocaleString()} total
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {alerts.length === 0 ? (
            <div className="text-center py-12">
              <CheckCircledIcon className="h-12 w-12 text-[var(--accent-green)] mx-auto mb-4" />
              <h3 className="text-lg font-medium text-[var(--text-primary)] mb-2">
                All clear!
              </h3>
              <p className="text-[var(--text-secondary)]">
                {unresolvedOnly
                  ? "No unresolved security alerts."
                  : "No security alerts found."}
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              {alerts.map((alert: SecurityAlert) => (
                <div
                  key={alert.id}
                  className={`border rounded-lg p-4 ${
                    alert.resolved_at ? "opacity-60" : ""
                  }`}
                >
                  <div className="flex items-start gap-4">
                    <div className="flex-shrink-0 mt-0.5">
                      {getSeverityIcon(alert.severity)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span
                          className={`px-2 py-0.5 text-xs font-medium rounded border ${getSeverityColor(
                            alert.severity
                          )}`}
                        >
                          {alert.severity.toUpperCase()}
                        </span>
                        <span className="font-medium">
                          {getAlertTypeLabel(alert.alert_type)}
                        </span>
                        {alert.resolved_at && (
                          <span className="inline-flex items-center gap-1 text-xs text-[var(--accent-green)]">
                            <CheckCircledIcon className="h-3 w-3" />
                            Resolved
                          </span>
                        )}
                      </div>
                      <div className="text-sm text-[var(--text-secondary)] mb-2" suppressHydrationWarning>
                        {formatDate(alert.created_at)}
                        {alert.user_id && ` • User: ${alert.user_id.slice(0, 8)}...`}
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
                        <Button
                          type="submit"
                          variant="outline"
                          size="sm"
                          disabled={isSubmitting}
                        >
                          <CheckCircledIcon className="h-4 w-4 mr-1" />
                          Resolve
                        </Button>
                      </Form>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Pagination */}
          {pagination.total_pages > 1 && (
            <div className="flex items-center justify-between mt-4 pt-4 border-t">
              <div className="text-sm text-[var(--text-secondary)]">
                Page {pagination.page} of {pagination.total_pages}
              </div>
              <div className="flex gap-2">
                {pagination.page > 1 && (
                  <Link
                    to={filterUrl({ page: String(pagination.page - 1) })}
                  >
                    <Button variant="outline" size="sm">
                      Previous
                    </Button>
                  </Link>
                )}
                {pagination.page < pagination.total_pages && (
                  <Link
                    to={filterUrl({ page: String(pagination.page + 1) })}
                  >
                    <Button variant="outline" size="sm">
                      Next
                    </Button>
                  </Link>
                )}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Security Recommendations */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Security Recommendations</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="text-sm text-[var(--text-secondary)] space-y-2">
            <li>• Review and resolve critical alerts within 24 hours</li>
            <li>• Enable MFA for all admin accounts</li>
            <li>• Configure rate limiting to prevent brute force attacks</li>
            <li>• Set up webhooks to receive real-time security notifications</li>
            <li>• Regularly review user sessions and revoke suspicious ones</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
