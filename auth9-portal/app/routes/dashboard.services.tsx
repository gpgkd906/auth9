import type { MetaFunction } from "@remix-run/node";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";

export const meta: MetaFunction = () => {
  return [{ title: "Services - Auth9" }];
};

export default function ServicesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold text-gray-900">Services</h1>
        <p className="text-sm text-gray-500">Register and manage OIDC clients</p>
      </div>
      <Card>
        <CardHeader>
          <CardTitle>Service Registry</CardTitle>
          <CardDescription>Connect to API to load service data</CardDescription>
        </CardHeader>
      </Card>
    </div>
  );
}
