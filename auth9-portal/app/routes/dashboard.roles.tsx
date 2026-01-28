import type { LoaderFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { useLoaderData } from "@remix-run/react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { rbacApi, serviceApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Roles - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "50");
  const services = await serviceApi.list(undefined, page, perPage);

  const entries = await Promise.all(
    services.data.map(async (service) => {
      const [roles, permissions] = await Promise.all([
        rbacApi.listRoles(service.id),
        rbacApi.listPermissions(service.id),
      ]);
      return { service, roles: roles.data, permissions: permissions.data };
    })
  );

  return json({ entries, pagination: services.pagination });
}

export default function RolesPage() {
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Roles</h1>
        <p className="text-sm text-gray-500">Define roles and permissions per service</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Role Management</CardTitle>
          <CardDescription>
            {data.pagination.total} services • Page {data.pagination.page} of{" "}
            {data.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6 space-y-4">
          {data.entries.map((entry) => (
            <div
              key={entry.service.id}
              className="rounded-apple border border-gray-100 p-4"
            >
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-sm text-gray-500">Service</div>
                  <div className="text-base font-semibold text-gray-900">
                    {entry.service.name}
                  </div>
                  <div className="text-xs text-gray-500">{entry.service.client_id}</div>
                </div>
                <div className="text-right text-sm text-gray-600">
                  <div>{entry.roles.length} roles</div>
                  <div>{entry.permissions.length} permissions</div>
                </div>
              </div>
              <div className="mt-4 flex flex-wrap gap-2">
                {entry.roles.map((role) => (
                  <span
                    key={role.id}
                    className="rounded-apple bg-gray-100 px-3 py-1 text-xs text-gray-700"
                  >
                    {role.name}
                  </span>
                ))}
                {entry.roles.length === 0 && (
                  <span className="text-xs text-gray-500">No roles defined</span>
                )}
              </div>
            </div>
          ))}
          {data.entries.length === 0 && (
            <div className="py-8 text-center text-sm text-gray-500">No services found</div>
          )}
        </div>
      </Card>
    </div>
  );
}
