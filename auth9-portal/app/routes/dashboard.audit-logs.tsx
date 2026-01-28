import type { MetaFunction } from "@remix-run/node";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [{ title: "Audit Logs - Auth9" }];
};

export default function AuditLogsPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Audit Logs</h1>
        <p className="text-sm text-gray-500">Track administrative changes across tenants</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Audit Trail</CardTitle>
          <CardDescription>Connect to API to load audit events</CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
