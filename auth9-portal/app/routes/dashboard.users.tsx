import type { MetaFunction } from "@remix-run/node";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [{ title: "Users - Auth9" }];
};

export default function UsersPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Users</h1>
        <p className="text-sm text-gray-500">Manage users and tenant assignments</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>User Directory</CardTitle>
          <CardDescription>Connect to API to load user data</CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
