import type { MetaFunction } from "@remix-run/node";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [{ title: "Roles - Auth9" }];
};

export default function RolesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Roles</h1>
        <p className="text-sm text-gray-500">Define roles and permissions per service</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Role Management</CardTitle>
          <CardDescription>Connect to API to load role data</CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
