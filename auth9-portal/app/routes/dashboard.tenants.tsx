import type { MetaFunction } from "@remix-run/node";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [{ title: "Tenants - Auth9" }];
};

export default function TenantsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Tenants</h1>
        <p className="text-sm text-gray-500">Manage tenant lifecycle and settings</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Tenant List</CardTitle>
          <CardDescription>Connect to API to load tenant data</CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
