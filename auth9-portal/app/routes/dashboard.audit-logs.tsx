import type { LoaderFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { useLoaderData } from "@remix-run/react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { auditApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Audit Logs - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "50");
  const logs = await auditApi.list(page, perPage);
  return json(logs);
}

export default function AuditLogsPage() {
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Audit Logs</h1>
        <p className="text-sm text-gray-500">Track administrative changes across tenants</p>
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
          <div className="overflow-hidden rounded-apple border border-gray-100">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-gray-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Action</th>
                  <th className="px-4 py-3 font-medium">Resource</th>
                  <th className="px-4 py-3 font-medium">Actor</th>
                  <th className="px-4 py-3 font-medium">Time</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {data.data.map((log) => (
                  <tr key={log.id} className="text-gray-700">
                    <td className="px-4 py-3 font-medium text-gray-900">{log.action}</td>
                    <td className="px-4 py-3">
                      {log.resource_type}
                      {log.resource_id ? `:${log.resource_id}` : ""}
                    </td>
                    <td className="px-4 py-3">{log.actor_id || "-"}</td>
                    <td className="px-4 py-3">
                      {new Date(log.created_at).toLocaleString()}
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-gray-500" colSpan={4}>
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
