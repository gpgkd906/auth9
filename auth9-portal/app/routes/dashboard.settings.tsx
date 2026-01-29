import type { ActionFunctionArgs, LoaderFunctionArgs, MetaFunction } from "@remix-run/node";
import { json } from "@remix-run/node";
import { Form, useActionData, useLoaderData, useNavigation, useSubmit } from "@remix-run/react";
import { Pencil2Icon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Label } from "~/components/ui/label";
import { Textarea } from "~/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
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

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "update_settings") {
      const id = formData.get("id") as string;
      const settingsJson = formData.get("settings") as string;

      let settings: Record<string, unknown>;
      try {
        settings = JSON.parse(settingsJson);
      } catch (e) {
        return json({ error: "Invalid JSON format" }, { status: 400 });
      }

      await tenantApi.update(id, { settings });
      return json({ success: true });
    }
  } catch (error: any) {
    return json({ error: error.message }, { status: 400 });
  }

  return json({ error: "Invalid intent" }, { status: 400 });
}

export default function SettingsPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [editingTenant, setEditingTenant] = useState<any>(null);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setEditingTenant(null);
    }
  }, [actionData]);

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
                  <th className="px-4 py-3 font-medium">Settings (Preview)</th>
                  <th className="px-4 py-3 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {data.data.map((tenant) => (
                  <tr key={tenant.id} className="text-gray-700 hover:bg-gray-50/50">
                    <td className="px-4 py-3 font-medium text-gray-900">{tenant.name}</td>
                    <td className="px-4 py-3 capitalize">{tenant.status}</td>
                    <td className="px-4 py-3 text-xs text-gray-600 font-mono">
                      {JSON.stringify(tenant.settings).substring(0, 50)}
                      {JSON.stringify(tenant.settings).length > 50 ? "..." : ""}
                    </td>
                    <td className="px-4 py-3">
                      <Button variant="ghost" size="sm" onClick={() => setEditingTenant(tenant)}>
                        <Pencil2Icon className="h-4 w-4" />
                      </Button>
                    </td>
                  </tr>
                ))}
                {data.data.length === 0 && (
                  <tr>
                    <td className="px-4 py-6 text-center text-gray-500" colSpan={4}>
                      No tenant settings found
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </Card>

      {/* Edit Dialog */}
      <Dialog open={!!editingTenant} onOpenChange={(open) => !open && setEditingTenant(null)}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Edit Settings: {editingTenant?.name}</DialogTitle>
            <DialogDescription>
              Update tenant configuration JSON.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update_settings" />
            <input type="hidden" name="id" value={editingTenant?.id || ""} />
            <div className="space-y-2">
              <Label htmlFor="edit-settings">Settings (JSON)</Label>
              <Textarea
                id="edit-settings"
                name="settings"
                className="min-h-[200px] font-mono"
                defaultValue={editingTenant ? JSON.stringify(editingTenant.settings, null, 2) : "{}"}
                required
              />
            </div>
            {actionData && "error" in actionData && (
              <p className="text-sm text-red-500">{actionData.error}</p>
            )}
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setEditingTenant(null)}>
                Cancel
              </Button>
              <Button type="submit" disabled={isSubmitting}>
                {isSubmitting ? "Saving..." : "Save Changes"}
              </Button>
            </DialogFooter>
          </Form>
        </DialogContent>
      </Dialog>
    </div>
  );
}
