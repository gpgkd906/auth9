import type { MetaFunction, LoaderFunctionArgs } from "react-router";
import { useLoaderData, Link } from "react-router";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { auditApi } from "~/services/api";
import { getAccessToken } from "~/services/session.server";
import { FormattedDate } from "~/components/ui/formatted-date";
import { useI18n } from "~/i18n";
import { buildMeta, resolveMetaLocale } from "~/i18n/meta";

export const meta: MetaFunction = ({ matches }) => {
  return buildMeta(resolveMetaLocale(matches), "audit.metaTitle");
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "50");
  const accessToken = await getAccessToken(request);
  const logs = await auditApi.list(page, perPage, accessToken || undefined);
  return logs;
}

export default function AuditLogsPage() {
  const { t } = useI18n();
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">{t("audit.title")}</h1>
        <p className="text-sm text-[var(--text-secondary)]">{t("audit.description")}</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>{t("audit.trail")}</CardTitle>
          <CardDescription>
            {t("audit.trailDescription", {
              total: data.pagination.total,
              page: data.pagination.page,
              totalPages: data.pagination.total_pages,
            })}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-xl border border-[var(--glass-border-subtle)]">
            <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-secondary)]">
                <tr>
                  <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-[0.04em]">{t("audit.action")}</th>
                  <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-[0.04em]">{t("audit.resource")}</th>
                  <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-[0.04em]">{t("audit.actor")}</th>
                  <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-[0.04em]">{t("audit.time")}</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--glass-border-subtle)]">
                {data.data.map((log) => (
                  <tr key={log.id} className="text-[var(--text-secondary)]">
                    <td className="px-4 py-3 font-medium text-[var(--text-primary)]">{log.action}</td>
                    <td className="px-4 py-3">
                      {log.resource_type}
                      {log.resource_id ? `:${log.resource_id}` : ""}
                    </td>
                    <td className="px-4 py-3">{log.actor_email || log.actor_display_name || "-"}</td>
                    <td className="px-4 py-3">
                      <FormattedDate date={log.created_at} />
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-[var(--text-tertiary)]" colSpan={4}>
                      {t("audit.noLogs")}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          {data.pagination.total_pages > 1 && (
            <div className="flex items-center justify-between mt-4">
              <div className="text-sm text-[var(--text-secondary)]">
                {t("audit.page", { page: data.pagination.page, totalPages: data.pagination.total_pages })}
              </div>
              <div className="flex gap-2">
                {data.pagination.page > 1 && (
                  <Link to={`?page=${data.pagination.page - 1}`}>
                    <Button variant="outline" size="sm">
                      {t("audit.previous")}
                    </Button>
                  </Link>
                )}
                {data.pagination.page < data.pagination.total_pages && (
                  <Link to={`?page=${data.pagination.page + 1}`}>
                    <Button variant="outline" size="sm">
                      {t("audit.next")}
                    </Button>
                  </Link>
                )}
              </div>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
