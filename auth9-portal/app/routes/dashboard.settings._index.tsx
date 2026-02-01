import type { LoaderFunctionArgs, ActionFunctionArgs } from "react-router";
import { Form, useActionData, useLoaderData, useNavigation } from "react-router";
import { Pencil2Icon } from "@radix-ui/react-icons";
import { useEffect, useState } from "react";
import { Card, CardDescription, CardHeader, CardTitle } from "~/components/ui/card";
import { Button } from "~/components/ui/button";
import { Label } from "~/components/ui/label";
import { Input } from "~/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { tenantApi, type Tenant } from "~/services/api";

interface TenantSettings {
  branding?: {
    logo_url?: string;
    primary_color?: string;
  };
  [key: string]: unknown;
}

export async function loader({ request }: LoaderFunctionArgs) {
  const url = new URL(request.url);
  const page = Number(url.searchParams.get("page") || "1");
  const perPage = Number(url.searchParams.get("perPage") || "10");
  const tenants = await tenantApi.list(page, perPage);
  return tenants;
}

export async function action({ request }: ActionFunctionArgs) {
  const formData = await request.formData();
  const intent = formData.get("intent");

  try {
    if (intent === "update_settings") {
      const id = formData.get("id") as string;

      const logo_url = formData.get("branding_logo_url") as string;
      const primary_color = formData.get("branding_primary_color") as string;

      const settings = {
        branding: {
          logo_url: logo_url || undefined,
          primary_color: primary_color || undefined,
        }
      };

      await tenantApi.update(id, { settings });
      return { success: true };
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown error";
    return Response.json({ error: message }, { status: 400 });
  }

  return Response.json({ error: "Invalid intent" }, { status: 400 });
}

export default function OrganizationSettingsPage() {
  const data = useLoaderData<typeof loader>();
  const actionData = useActionData<typeof action>();
  const navigation = useNavigation();
  const [editingTenant, setEditingTenant] = useState<Tenant | null>(null);

  const isSubmitting = navigation.state === "submitting";

  useEffect(() => {
    if (actionData && "success" in actionData && actionData.success) {
      setEditingTenant(null);
    }
  }, [actionData]);

  return (
    <>
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
                  <th className="px-4 py-3 font-medium">Branding</th>
                  <th className="px-4 py-3 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {data.data.map((tenant) => {
                  const settings = tenant.settings as TenantSettings;
                  return (
                    <tr key={tenant.id} className="text-gray-700 hover:bg-gray-50/50">
                      <td className="px-4 py-3 font-medium text-gray-900">{tenant.name}</td>
                      <td className="px-4 py-3 capitalize">{tenant.status}</td>
                      <td className="px-4 py-3 text-xs text-gray-600">
                        {settings?.branding?.logo_url && (
                          <div className="flex items-center gap-2">
                            <img src={settings.branding.logo_url} alt="Logo" className="h-6 w-6 object-contain rounded-sm bg-gray-100" />
                            <span className="truncate max-w-[150px]">{settings.branding.logo_url}</span>
                          </div>
                        )}
                        {!settings?.branding?.logo_url && <span className="text-gray-400">No branding</span>}
                      </td>
                      <td className="px-4 py-3">
                        <Button variant="ghost" size="sm" onClick={() => setEditingTenant(tenant)}>
                          <Pencil2Icon className="h-4 w-4" />
                        </Button>
                      </td>
                    </tr>
                  );
                })}
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
              Customize appearance and behavior.
            </DialogDescription>
          </DialogHeader>
          <Form method="post" className="space-y-4">
            <input type="hidden" name="intent" value="update_settings" />
            <input type="hidden" name="id" value={editingTenant?.id || ""} />

            <div className="space-y-4">
              <h3 className="text-sm font-medium text-gray-900 border-b pb-2">Branding</h3>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="branding_logo_url">Logo URL</Label>
                  <Input
                    id="branding_logo_url"
                    name="branding_logo_url"
                    defaultValue={(editingTenant?.settings as TenantSettings)?.branding?.logo_url || ""}
                    placeholder="https://example.com/logo.png"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="branding_primary_color">Primary Color</Label>
                  <div className="flex gap-2">
                    <Input
                      id="branding_primary_color"
                      name="branding_primary_color"
                      type="color"
                      className="w-12 h-10 p-1"
                      defaultValue={(editingTenant?.settings as TenantSettings)?.branding?.primary_color || "#000000"}
                    />
                    <Input
                      name="branding_primary_color_text"
                      defaultValue={(editingTenant?.settings as TenantSettings)?.branding?.primary_color || "#000000"}
                      placeholder="#000000"
                      readOnly
                      className="flex-1 bg-gray-50"
                    />
                  </div>
                </div>
              </div>
            </div>

            {actionData && "error" in actionData && (
              <p className="text-sm text-red-500">{String(actionData.error)}</p>
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
    </>
  );
}
