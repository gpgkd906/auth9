import type { LoaderFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { useLoaderData } from "@remix-run/react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { tenantApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Settings - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "10");
  const tenants = await tenantApi.list(page, perPage);
  return json(tenants);
}

export default function SettingsPage() {
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Settings</h1>
        <p className="text-sm text-gray-500">Configure organization preferences</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Organization Settings</CardTitle>
          <CardDescription>
            {data.pagination.total} tenants • Page {data.pagination.page} of{" "}
            {data.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-apple border border-gray-100">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-gray-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Tenant</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Settings</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {data.data.map((tenant) => (
                  <tr key={tenant.id} className="text-gray-700">
                    <td className="px-4 py-3 font-medium text-gray-900">{tenant.name}</td>
                    <td className="px-4 py-3 capitalize">{tenant.status}</td>
                    <td className="px-4 py-3 text-xs text-gray-600">
                      {JSON.stringify(tenant.settings)}
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-gray-500" colSpan={3}>
                      No tenant settings found
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
