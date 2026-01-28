import type { LoaderFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { useLoaderData } from "@remix-run/react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { serviceApi } from "~/services/api";

export const meta: MetaFunction = () => {
  return [{ title: "Services - Auth9" }];
};

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "20");
  const services = await serviceApi.list(undefined, page, perPage);
  return json(services);
}

export default function ServicesPage() {
  const data = useLoaderData<typeof loader>();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Services</h1>
        <p className="text-sm text-gray-500">Register and manage OIDC clients</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Service Registry</CardTitle>
          <CardDescription>
            {data.pagination.total} services • Page {data.pagination.page} of{" "}
            {data.pagination.total_pages}
          </CardDescription>
        </CardHeader>
        <div className="px-6 pb-6">
          <div className="overflow-hidden rounded-apple border border-gray-100">
            <table className="min-w-full divide-y divide-gray-100 text-sm">
              <thead className="bg-gray-50 text-left text-gray-500">
                <tr>
                  <th className="px-4 py-3 font-medium">Name</th>
                  <th className="px-4 py-3 font-medium">Client ID</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Updated</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {data.data.map((service) => (
                  <tr key={service.id} className="text-gray-700">
                    <td className="px-4 py-3 font-medium text-gray-900">{service.name}</td>
                    <td className="px-4 py-3">{service.client_id}</td>
                    <td className="px-4 py-3 capitalize">{service.status}</td>
                    <td className="px-4 py-3">
                      {new Date(service.updated_at).toLocaleString()}
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-gray-500" colSpan={4}>
                      No services found
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
