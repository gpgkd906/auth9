import type { MetaFunction, LoaderFunctionArgs } from "react-router";
import { useLoaderData } from "react-router";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { auditApi } from "~/services/api";
import { getAccessToken } from "~/services/session.server";

export const meta: MetaFunction = () => {
  return [{ title: "Audit Logs - Auth9" }];
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
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-[24px] font-semibold text-[var(--text-primary)] tracking-tight">Audit Logs</h1>
        <p className="text-sm text-[var(--text-secondary)]">Track administrative changes across tenants</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Audit Trail</CardTitle>
          <CardDescription>
            {data.pagination.total} events • Page {data.pagination.page} of{" "}
            {data.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-xl border border-[var(--glass-border-subtle)]">
            <table className="min-w-full divide-y divide-[var(--glass-border-subtle)] text-sm">
              <thead className="bg-[var(--sidebar-item-hover)] text-left text-[var(--text-secondary)]">
                <tr>
                  <th className="px-4 py-3 font-medium">Action</th>
                  <th className="px-4 py-3 font-medium">Resource</th>
                  <th className="px-4 py-3 font-medium">Actor</th>
                  <th className="px-4 py-3 font-medium">Time</th>
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
                      {new Date(log.created_at).toLocaleString()}
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-[var(--text-tertiary)]" colSpan={4}>
                      No audit logs found
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </Card>
    </div>
  );
}
